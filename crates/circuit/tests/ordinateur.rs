//! Test d'acceptation : l'ordinateur miniature en NAND exécute de vrais programmes, et se
//! comporte à chaque cycle comme le modèle de référence.

use ce_circuit::ordinateur::{
    assembler, bibliotheque, decoder, programmes, Emulateur, Machine, Op, Reglages,
};
use ce_circuit::Banc;
use ce_moteur::alea::Alea;
use ce_moteur::Valeur;

fn machine(programme: &[u16]) -> Machine {
    Machine::nouvelle(programme, Reglages::default()).expect("la machine se fabrique")
}

/// Exécute le programme sur la machine en NAND et sur le modèle, et compare l'état visible
/// (PC, A, Z, C, écran) après chaque cycle, puis la mémoire vive à la fin.
fn comparer(
    programme: &[u16],
    reglages: Reglages,
    cycles: usize,
    mut clavier: impl FnMut(usize) -> u8,
) -> (Machine, Emulateur) {
    let mut m = Machine::nouvelle(programme, reglages).unwrap();
    let mut e = Emulateur::nouveau(reglages.bits_ram);
    // Une sonde sur le signal qui commande l'écran : il ne doit monter qu'une fois par OUT.
    let commande = m.banc.circuit.net("ecran_ecrit").unwrap();
    m.banc.sim.sonder(commande);
    for i in 0..cycles {
        let touche = clavier(i);
        let (avant_pc, mot) = (e.pc, programme.get(usize::from(e.pc)).copied().unwrap_or(0));
        let attendu = e.pas(programme, touche);
        let obtenu = m.cycle(touche);
        assert!(!e.indefini, "le programme lit une case jamais écrite");
        let contexte = format!("cycle {i}, pc = {avant_pc}, instruction {:?}", decoder(mot));
        assert_eq!(obtenu, attendu, "écran, {contexte}");
        assert_eq!(m.pc(), Some(e.pc), "PC, {contexte}");
        assert_eq!(m.a(), Some(e.a), "A, {contexte}");
        assert_eq!(m.z(), Some(e.z), "Z, {contexte}");
        assert_eq!(m.c(), Some(e.c), "C, {contexte}");
        assert_eq!(m.arret(), Some(e.arret(programme)), "arrêt, {contexte}");
    }
    for (adresse, case) in e.ram.iter().enumerate() {
        assert_eq!(m.ram(adresse), *case, "mémoire vive, case {adresse}");
    }
    let journal = m.banc.sim.vider_journal();
    let montees = journal.iter().filter(|c| c.valeur == Valeur::Un).count();
    // Si la machine s'arrête sur un OUT, sa commande est déjà montée sans avoir été comptée.
    let en_cours = usize::from(journal.last().is_some_and(|c| c.valeur == Valeur::Un));
    assert_eq!(
        montees - en_cours,
        m.ecran.len(),
        "la commande de l'écran monte exactement une fois par caractère, sans parasite"
    );
    assert!(journal.iter().all(|c| c.valeur.est_binaire()));
    (m, e)
}

/// Un programme au hasard : il commence par écrire toutes les cases de la mémoire vive, puis
/// enchaîne des instructions quelconques, sauts compris.
fn programme_aleatoire(alea: &mut Alea, bits_ram: u32, longueur: usize) -> Vec<u16> {
    let mut p = Vec::new();
    for case in 0..1u64 << bits_ram {
        p.push(Op::Ldi.mot(alea.sous(256) as u8));
        p.push(Op::Sta.mot(case as u8));
    }
    while p.len() < longueur {
        let op = match alea.sous(40) {
            0 => Op::Hlt,
            1..=2 => Op::Nop,
            3..=5 => Op::Jmp,
            6..=8 => Op::Jz,
            9..=11 => Op::Jc,
            12..=13 => Op::In,
            14..=15 => Op::Out,
            n => [
                Op::Sta,
                Op::Lda,
                Op::Add,
                Op::Sub,
                Op::Nand,
                Op::Ldi,
                Op::Addi,
                Op::Subi,
                Op::Nandi,
            ][n as usize % 9],
        };
        let operande = if matches!(op, Op::Jmp | Op::Jz | Op::Jc) {
            alea.sous(longueur as u64 + 4) as u8
        } else {
            // Des opérandes souvent extrêmes : ce sont eux qui font les retenues et les zéros.
            match alea.sous(6) {
                0 => 0,
                1 => 255,
                2 => 1,
                _ => alea.sous(256) as u8,
            }
        };
        p.push(op.mot(operande));
    }
    p
}

#[test]
fn apres_la_remise_a_zero_tout_est_determine_sauf_la_memoire_vive() {
    let m = machine(&[Op::Hlt.mot(0)]);
    assert_eq!(m.pc(), Some(0));
    assert_eq!(m.a(), Some(0));
    assert_eq!(m.z(), Some(false));
    assert_eq!(m.c(), Some(false));
    assert_eq!(m.arret(), Some(true));
    for adresse in 0..16 {
        assert_eq!(m.ram(adresse), None, "case {adresse} jamais écrite");
    }
}

#[test]
fn bonjour() {
    let programme = assembler(&programmes::afficher("BONJOUR")).unwrap();
    let mut m = machine(&programme);
    let ecran = m.executer(100, |_| 0);
    assert_eq!(String::from_utf8(ecran).unwrap(), "BONJOUR");
    assert_eq!(m.arret(), Some(true));
    assert_eq!(m.cycles, 14);
    comparer(&programme, Reglages::default(), 20, |_| 0);
}

#[test]
fn fibonacci() {
    let programme = assembler(programmes::FIBONACCI).unwrap();
    let mut attendu = vec![0u8, 1];
    while let Some(suivant) = attendu[attendu.len() - 1].checked_add(attendu[attendu.len() - 2]) {
        attendu.push(suivant);
    }
    // Le programme s'arrête quand la somme dépasse : le dernier terme calculé n'est pas affiché.
    attendu.pop();
    assert_eq!(attendu.last(), Some(&144));
    let mut m = machine(&programme);
    assert_eq!(m.executer(1000, |_| 0), attendu);
    assert_eq!(m.arret(), Some(true));
    let (m, _) = comparer(&programme, Reglages::default(), 200, |_| 0);
    assert_eq!(m.ecran, attendu);
}

#[test]
fn calculatrice_au_clavier() {
    let programme = assembler(programmes::CALCULATRICE).unwrap();
    // Chaque touche reste enfoncée 9 cycles, puis est relâchée.
    let clavier = |x: u8, y: u8| {
        move |i: usize| match i {
            5..=13 => b'0' + x,
            30..=38 => b'0' + y,
            _ => 0,
        }
    };
    for (x, y) in (0..10u8).flat_map(|x| (0..10u8).map(move |y| (x, y))) {
        let mut m = machine(&programme);
        let ecran = String::from_utf8(m.executer(2000, clavier(x, y))).unwrap();
        assert_eq!(ecran, format!("{x}*{y}={}", x * y));
        assert_eq!(m.arret(), Some(true));
    }
    comparer(&programme, Reglages::default(), 400, clavier(9, 9));
}

#[test]
fn majuscules_au_clavier() {
    let programme = assembler(programmes::MAJUSCULES).unwrap();
    let texte = b"Bonjour, Lune 42 {z}`a";
    let clavier = |i: usize| {
        // 12 cycles enfoncée, 12 cycles relâchée.
        let (rang, phase) = (i / 24, i % 24);
        if phase < 12 {
            texte.get(rang).copied().unwrap_or(0)
        } else {
            0
        }
    };
    let (m, _) = comparer(
        &programme,
        Reglages::default(),
        24 * texte.len() + 30,
        clavier,
    );
    assert_eq!(
        String::from_utf8(m.ecran).unwrap(),
        "BONJOUR, LUNE 42 {Z}`A"
    );
}

#[test]
fn programmes_aleatoires_compares_au_modele_cycle_par_cycle() {
    let mut alea = Alea::nouveau(2026);
    let mut instructions_vues = std::collections::HashSet::new();
    let mut cycles = 0;
    for _ in 0..60 {
        let programme = programme_aleatoire(&mut alea, 4, 200);
        let mut touches = Alea::nouveau(alea.u64());
        comparer(&programme, Reglages::default(), 400, |_| {
            if touches.chance(1, 3) {
                0
            } else {
                touches.sous(256) as u8
            }
        });
        // Quelles instructions ont réellement été exécutées ?
        let mut rejoue = Emulateur::nouveau(4);
        for _ in 0..400 {
            let (op, _) = decoder(programme.get(usize::from(rejoue.pc)).copied().unwrap_or(0));
            instructions_vues.insert(op);
            if rejoue.arret(&programme) {
                break;
            }
            rejoue.pas(&programme, 0);
            cycles += 1;
        }
    }
    assert_eq!(
        instructions_vues.len(),
        16,
        "les seize instructions ont été exécutées"
    );
    assert!(cycles > 5000, "{cycles} cycles comparés");
}

#[test]
fn toutes_les_tailles_de_memoire_vive() {
    let mut alea = Alea::nouveau(7);
    for bits_ram in 1..=6 {
        let reglages = Reglages {
            bits_ram,
            ..Reglages::default()
        };
        let longueur = (2usize << bits_ram) + 60;
        let programme = programme_aleatoire(&mut alea, bits_ram, longueur.min(250));
        let (m, _) = comparer(&programme, reglages, 300, |i| i as u8);
        assert!(m.ram(0).is_some());
    }
}

#[test]
fn la_remise_a_zero_en_marche_repart_du_debut_et_garde_la_memoire() {
    let programme = assembler("LDI 7\nSTA 5\nLDI 'A'\nOUT\nADDI 1\nOUT\nJMP 4\n").unwrap();
    let mut m = machine(&programme);
    assert_eq!(m.executer(12, |_| 0), b"ABCD");
    m.remettre_a_zero();
    assert_eq!(
        (m.pc(), m.a(), m.z(), m.c()),
        (Some(0), Some(0), Some(false), Some(false))
    );
    assert_eq!(m.ram(5), Some(7), "la mémoire vive n'est pas effacée");
    assert_eq!(m.executer(6, |_| 0), b"AB");
}

#[test]
fn l_unite_de_calcul_est_juste_pour_toutes_les_entrees() {
    let bib = bibliotheque(&[], Reglages::default()).unwrap();
    let mut banc = Banc::nouveau(&bib, "UAL").unwrap();
    for op in 0..4u64 {
        banc.ecrire("op", op).unwrap();
        for a in 0..256u64 {
            banc.ecrire("a", a).unwrap();
            for b in 0..256u64 {
                banc.ecrire("b", b).unwrap();
                assert!(banc.stabiliser(200).est_stable());
                let (r, rs) = match op {
                    0 => (b, None),
                    1 => ((a + b) & 255, Some(u64::from(a + b > 255))),
                    2 => (a.wrapping_sub(b) & 255, Some(u64::from(a >= b))),
                    _ => (!(a & b) & 255, None),
                };
                assert_eq!(banc.lire("r").unwrap(), Some(r), "op {op}, a {a}, b {b}");
                if let Some(rs) = rs {
                    assert_eq!(
                        banc.lire("rs").unwrap(),
                        Some(rs),
                        "retenue, op {op}, a {a}, b {b}"
                    );
                }
            }
        }
    }
}

#[test]
fn le_decodeur_donne_les_bonnes_commandes_pour_les_seize_instructions() {
    let bib = bibliotheque(&[], Reglages::default()).unwrap();
    let mut banc = Banc::nouveau(&bib, "DECODEUR").unwrap();
    for code in 0..16u64 {
        for (z, c) in [(0, 0), (0, 1), (1, 0), (1, 1)] {
            banc.ecrire("o", code).unwrap();
            banc.ecrire("z", z).unwrap();
            banc.ecrire("c", c).unwrap();
            assert!(banc.stabiliser(100).est_stable());
            let op = Op::de_code(code as u8);
            let lire = |nom: &str| banc.lire(nom).unwrap().unwrap();
            let calcul = matches!(
                op,
                Op::Lda | Op::Add | Op::Sub | Op::Nand | Op::Ldi | Op::Addi | Op::Subi | Op::Nandi
            );
            assert_eq!(lire("ecrit_a") == 1, calcul || op == Op::In, "{op:?}");
            assert_eq!(lire("entree") == 1, op == Op::In, "{op:?}");
            assert_eq!(lire("sortie") == 1, op == Op::Out, "{op:?}");
            assert_eq!(lire("ecrit_ram") == 1, op == Op::Sta, "{op:?}");
            assert_eq!(lire("arret") == 1, op == Op::Hlt, "{op:?}");
            assert_eq!(
                lire("charge_c") == 1,
                matches!(op, Op::Add | Op::Sub | Op::Addi | Op::Subi),
                "{op:?}"
            );
            let saut = op == Op::Jmp || (op == Op::Jz && z == 1) || (op == Op::Jc && c == 1);
            assert_eq!(lire("saut") == 1, saut, "{op:?} z {z} c {c}");
            if calcul {
                assert_eq!(
                    lire("imm") == 1,
                    matches!(op, Op::Ldi | Op::Addi | Op::Subi | Op::Nandi),
                    "{op:?}"
                );
                assert_eq!(lire("op"), code & 3, "{op:?}");
            }
        }
    }
}

#[test]
fn la_memoire_morte_rend_chaque_mot_a_son_adresse() {
    let mut alea = Alea::nouveau(99);
    for longueur in [0usize, 1, 17, 100, 256] {
        let programme: Vec<u16> = (0..longueur)
            .map(|_| {
                if alea.chance(1, 8) {
                    0
                } else {
                    alea.sous(4096) as u16
                }
            })
            .collect();
        let bib = bibliotheque(&programme, Reglages::default()).unwrap();
        let mut banc = Banc::nouveau(&bib, "MEMOIRE_MORTE").unwrap();
        for adresse in 0..256usize {
            banc.ecrire("a", adresse as u64).unwrap();
            assert!(banc.stabiliser(100).est_stable());
            let attendu = programme.get(adresse).copied().unwrap_or(0);
            assert_eq!(
                banc.lire("d").unwrap(),
                Some(u64::from(attendu)),
                "{longueur} mots, adresse {adresse}"
            );
        }
    }
}

#[test]
fn la_memoire_vive_se_souvient_de_ce_qu_on_y_ecrit() {
    let bib = bibliotheque(&[], Reglages::default()).unwrap();
    let mut banc = Banc::nouveau(&bib, "RAM16").unwrap();
    let mut alea = Alea::nouveau(5);
    let mut modele: [Option<u64>; 16] = [None; 16];
    banc.ecrire("clk", 0).unwrap();
    for _ in 0..600 {
        let (adresse, d, ecrire) = (alea.sous(16), alea.sous(256), alea.sous(2));
        banc.ecrire("adresse", adresse).unwrap();
        banc.ecrire("d", d).unwrap();
        banc.ecrire("ecrire", ecrire).unwrap();
        banc.avancer(40);
        banc.ecrire("clk", 1).unwrap();
        banc.avancer(40);
        banc.ecrire("clk", 0).unwrap();
        if ecrire == 1 {
            modele[adresse as usize] = Some(d);
        }
        banc.avancer(40);
        assert_eq!(banc.lire("q").unwrap(), modele[adresse as usize]);
    }
    assert!(modele.iter().all(Option::is_some));
}

#[test]
fn l_incrementeur_et_le_detecteur_de_zero_sont_justes_partout() {
    let bib = bibliotheque(&[], Reglages::default()).unwrap();
    let mut incr = Banc::nouveau(&bib, "INCR8").unwrap();
    let mut nul = Banc::nouveau(&bib, "NUL8").unwrap();
    for e in 0..256u64 {
        incr.ecrire("e", e).unwrap();
        nul.ecrire("e", e).unwrap();
        assert!(incr.stabiliser(100).est_stable() && nul.stabiliser(100).est_stable());
        assert_eq!(incr.lire("s").unwrap(), Some((e + 1) & 255));
        assert_eq!(nul.lire("z").unwrap(), Some(u64::from(e == 0)));
    }
}

#[test]
fn l_horloge_laisse_le_temps_au_plus_long_chemin() {
    // On fabrique la machine avec une horloge très lente et on mesure, après chaque front,
    // combien de temps il faut pour que plus aucune porte ne bouge.
    let lent = Reglages {
        demi_periode: 600,
        ..Reglages::default()
    };
    let mut alea = Alea::nouveau(31);
    let (mut apres_montant, mut apres_descendant) = (0u64, 0u64);
    for _ in 0..12 {
        let programme = programme_aleatoire(&mut alea, 4, 200);
        let mut m = Machine::nouvelle(&programme, lent).unwrap();
        // La machine est au milieu d'un niveau bas : on va jusqu'au front montant.
        m.banc.avancer(lent.demi_periode / 2);
        for _ in 0..250 {
            for pire in [&mut apres_montant, &mut apres_descendant] {
                let mut dernier = 0;
                for t in 1..=lent.demi_periode {
                    m.banc.avancer(1);
                    // Le dernier instant est déjà le front suivant : il ne compte pas.
                    if t < lent.demi_periode && !m.banc.sim.est_stable() {
                        dernier = t;
                    }
                }
                assert!(
                    dernier < lent.demi_periode / 2,
                    "la mesure exige une horloge plus lente"
                );
                *pire = (*pire).max(dernier + 1);
            }
        }
    }
    let p = Reglages::default().demi_periode;
    println!(
        "Temps d'établissement mesuré : {apres_montant} après un front montant, {apres_descendant} après un front descendant ; demi-période {p}."
    );
    // Tout doit être établi avant le front montant suivant, avec un quart de marge.
    assert!(
        apres_montant * 5 <= 2 * p * 4,
        "chemin le plus long : {apres_montant}, période : {}",
        2 * p
    );
    // Le décodage de l'instruction doit être fini avant la seconde moitié du cycle, où
    // l'écran prend son caractère ; la seconde moitié doit suffire aux bascules.
    assert!(apres_descendant * 5 <= p * 4);
}

#[test]
fn la_machine_est_faite_de_quelques_milliers_de_nand_et_de_rien_d_autre() {
    let programme = assembler(programmes::CALCULATRICE).unwrap();
    let m = machine(&programme);
    let netlist = &m.banc.circuit.netlist;
    assert!(netlist
        .portes
        .iter()
        .all(|p| p.genre == ce_moteur::GenrePorte::Nand));
    assert!(netlist.constantes.is_empty());
    assert_eq!(netlist.horloges.len(), 1);
    println!(
        "Ordinateur miniature : {} portes NAND, {} instructions.",
        m.nb_portes(),
        programme.len()
    );
    assert!((3000..6000).contains(&m.nb_portes()), "{}", m.nb_portes());
}

/// La machine à 256 octets de mémoire vive fait tourner le miroir, cycle par cycle comme le
/// modèle ; avec 64 octets, des touches se marchent dessus.
#[test]
fn le_miroir_demande_toute_la_grande_memoire() {
    let programme = assembler(programmes::MIROIR).unwrap();
    let texte = b"BONJOUR DEPUIS LES NAND!";
    let clavier = |i: usize| {
        let (rang, phase) = (i / 16, i % 16);
        if phase < 8 {
            texte.get(rang).copied().unwrap_or(0)
        } else {
            0
        }
    };
    let grande = Reglages {
        bits_ram: 8,
        ..Reglages::default()
    };
    let (m, _) = comparer(&programme, grande, 16 * texte.len() + 80, clavier);
    let mut envers = texte.to_vec();
    envers.reverse();
    assert_eq!(m.ecran, envers);
    let mut petite = ce_circuit::ordinateur::Emulateur::nouveau(6);
    let ecran = petite.executer(&programme, 16 * texte.len() + 80, clavier);
    assert_ne!(ecran, envers);
}
