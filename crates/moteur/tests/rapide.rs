//! Le mode rapide ne change rien de ce qu'on peut observer.
//!
//! Chaque essai fabrique un circuit synchrone au hasard : des bascules D maître-esclave
//! de neuf NAND (délais variés, entrées des portes dans un ordre ou dans l'autre),
//! cadencées par une horloge, et de la logique quelconque entre elles (NAND et tampons
//! trois états, délais de 1 à 3, lectures de l'horloge, des entrées, des constantes).
//! Certains circuits ne s'y prêtent pas (boucle dans la logique, logique trop lente) :
//! ils doivent alors être simulés porte par porte, sans erreur non plus.
//!
//! La même suite d'appels est jouée sur deux simulateurs, l'un avec les réglages par
//! défaut (mode rapide et sommeil) et l'autre porte par porte. Les appels avancent
//! surtout d'un nombre entier de périodes, ce qui laisse le mode rapide travailler,
//! mais aussi d'une durée quelconque, attendent le repos ou posent des sondes, ce qui
//! le fait quitter. Après chaque appel, tout ce que l'interface publique laisse voir
//! doit être identique : le temps, la valeur de chaque net, l'état complet de chaque
//! porte, le repos, le journal des sondes.
//!
//! C'est la confrontation du théorème 6 de `docs/semantique.md` à des milliers de cas.
//!
//! Le mode rapide a deux façons de calculer une période : événementielle, ou par le
//! code machine du circuit compilé (lemme C du théorème 6). Chaque série est jouée avec l'une,
//! avec l'autre, et en passant de l'une à l'autre en cours de route ; des copies du
//! simulateur (qui repartent sans code instancié) s'y glissent aussi.

use ce_moteur::alea::Alea;
use ce_moteur::{NetId, Netlist, Options, Simulateur, Valeur};

/// Tout ce que l'interface publique laisse voir d'un simulateur.
#[derive(PartialEq, Eq, Debug)]
struct Vue {
    temps: u64,
    nets: Vec<u8>,
    portes: Vec<[u32; 6]>,
    stable: bool,
    conflits: Vec<u32>,
    premier_conflit: Option<(u64, u32)>,
}

fn vue(sim: &Simulateur) -> Vue {
    Vue {
        temps: sim.temps(),
        nets: sim.codes().to_vec(),
        portes: (0..sim.nb_portes())
            .map(|g| sim.etat_relatif_porte(g))
            .collect(),
        stable: sim.est_stable(),
        conflits: sim.conflits(),
        premier_conflit: sim.premier_conflit(),
    }
}

const EXACT: Options = Options {
    sommeil: false,
    rapide: false,
    compilation: false,
};

/// Délais des neuf portes d'une bascule : la bibliothèque (tout à 1) et des variantes.
const DELAIS: [[u16; 9]; 6] = [
    [1; 9],
    [1; 9],
    [2, 1, 1, 1, 1, 1, 1, 1, 1],
    [1, 2, 1, 1, 2, 1, 2, 1, 1],
    [1, 1, 1, 2, 1, 1, 1, 1, 2],
    [1, 1, 3, 1, 1, 2, 1, 3, 1],
];

/// Une bascule D maître-esclave de neuf NAND, dont les sorties sont `q` et `qn`.
fn bascule(
    alea: &mut Alea,
    nl: &mut Netlist,
    c: NetId,
    d: NetId,
    q: NetId,
    qn: NetId,
    delais: [u16; 9],
) {
    let o = nl.nets(7);
    let (nclk, sm, rm, mq, mqn, ss, rs) = (o[0], o[1], o[2], o[3], o[4], o[5], o[6]);
    let entrees = [
        (c, c, nclk),
        (d, nclk, sm),
        (sm, nclk, rm),
        (sm, mqn, mq),
        (rm, mq, mqn),
        (mq, c, ss),
        (ss, c, rs),
        (ss, qn, q),
        (rs, q, qn),
    ];
    for (role, &(a, b, s)) in entrees.iter().enumerate() {
        let (a, b) = if alea.chance(1, 2) { (a, b) } else { (b, a) };
        nl.nand(a, b, s, delais[role]);
    }
}

fn valeur_aleatoire(alea: &mut Alea) -> Valeur {
    match alea.sous(10) {
        0 => Valeur::X,
        1 => Valeur::Z,
        k => Valeur::de_bool(k % 2 == 0),
    }
}

/// Un circuit synchrone au hasard, et sa demi-période d'horloge.
fn circuit(alea: &mut Alea) -> (Netlist, u64) {
    let mut nl = Netlist::nouvelle();
    let demi = [8, 16, 24, 32, 64][alea.sous(5) as usize];
    let c = nl.net();
    nl.horloge(c, demi);
    let nb_entrees = alea.entre(1, 4) as usize;
    let mut sources: Vec<NetId> = Vec::new();
    for _ in 0..nb_entrees {
        let n = nl.net();
        nl.entree(n);
        sources.push(n);
    }
    if alea.chance(1, 2) {
        let n = nl.net();
        nl.constante(n, Valeur::de_bool(alea.chance(1, 2)));
        sources.push(n);
    }
    if alea.chance(1, 4) {
        sources.push(c);
    }
    let nb_bascules = alea.entre(1, 8) as usize;
    let sorties: Vec<(NetId, NetId)> = (0..nb_bascules).map(|_| (nl.net(), nl.net())).collect();
    for &(q, qn) in &sorties {
        sources.push(q);
        if alea.chance(1, 2) {
            sources.push(qn);
        }
    }

    // La logique, sans boucle : chaque porte lit des sources ou des portes précédentes.
    let nb_logique = alea.entre(0, 30) as usize;
    let profondeur = alea.entre(1, 6) as usize;
    let mut logique: Vec<NetId> = Vec::new();
    for _ in 0..nb_logique {
        let choisir = |alea: &mut Alea, logique: &[NetId]| {
            // Les portes récentes surtout : la profondeur reste raisonnable.
            if !logique.is_empty() && alea.chance(1, 2) {
                let debut = logique.len().saturating_sub(profondeur);
                logique[debut + alea.sous((logique.len() - debut) as u64) as usize]
            } else {
                sources[alea.sous(sources.len() as u64) as usize]
            }
        };
        let a = choisir(alea, &logique);
        let b = choisir(alea, &logique);
        let s = nl.net();
        let delai = alea.entre(1, 3) as u16;
        if alea.chance(1, 10) {
            nl.tri(a, b, s, delai);
        } else {
            nl.nand(a, b, s, delai);
        }
        logique.push(s);
    }
    // Parfois une boucle dans la logique : le circuit n'est plus synchrone.
    if logique.len() >= 2 && alea.chance(1, 20) {
        let s = nl.net();
        nl.nand(logique[logique.len() - 1], s, s, 1);
    }

    // Les bascules, et ce qu'elles mémorisent.
    for &(q, qn) in &sorties {
        let d = if !logique.is_empty() && alea.chance(3, 4) {
            logique[alea.sous(logique.len() as u64) as usize]
        } else {
            loop {
                let n = sources[alea.sous(sources.len() as u64) as usize];
                if n != c {
                    break n;
                }
            }
        };
        let delais = DELAIS[alea.sous(DELAIS.len() as u64) as usize];
        bascule(alea, &mut nl, c, d, q, qn, delais);
    }

    // Des régions pour le sommeil : chaque bascule, parfois.
    let premiere = nl.portes.len() - 9 * nb_bascules;
    for b in 0..nb_bascules {
        if alea.chance(1, 2) {
            let debut = (premiere + 9 * b) as u32;
            nl.region(debut..debut + 9);
        }
    }
    (nl, demi)
}

/// Comment le mode rapide calcule les périodes pendant un essai.
#[derive(Clone, Copy, Debug)]
enum Calcul {
    /// Toujours événementiel : le circuit n'est jamais compilé.
    Evenementiel,
    /// Le circuit est compilé dès le départ.
    Compile,
    /// Le circuit est compilé à un appel tiré au hasard, et le simulateur est
    /// parfois remplacé par une copie.
    Mixte,
}

/// Joue un essai et rend le nombre de périodes calculées en mode rapide, et parmi
/// elles par le code compilé.
fn essai(graine: u64, nb_appels: usize, calcul: Calcul) -> (u64, u64) {
    let mut alea = Alea::nouveau(graine);
    let (nl, demi) = circuit(&mut alea);
    let periode = 2 * demi;
    let options = Options {
        compilation: !matches!(calcul, Calcul::Evenementiel),
        ..Options::default()
    };
    let mut rapide = Simulateur::avec_options(&nl, options).expect("netlist valide");
    if matches!(calcul, Calcul::Compile) {
        rapide.compiler();
    }
    // Tôt dans l'essai : une sonde posée empêche ensuite le mode rapide.
    let compiler_a = alea.sous(nb_appels.min(30) as u64 + 1) as usize;
    let mut exact = Simulateur::avec_options(&nl, EXACT).expect("netlist valide");
    let contexte = |quoi: &str, i: usize| format!("graine {graine}, appel {i} ({quoi})");

    for e in 0..nl.entrees.len() {
        let v = valeur_aleatoire(&mut alea);
        rapide.fixer_entree(e, v);
        exact.fixer_entree(e, v);
    }
    // Une phase quelconque, d'où partent ensuite des avances d'une période ou plus.
    let decalage = alea.entre(1, periode - 1);
    rapide.avancer(decalage);
    exact.avancer(decalage);
    assert_eq!(vue(&rapide), vue(&exact), "{}", contexte("départ", 0));

    for i in 0..nb_appels {
        if let Calcul::Mixte = calcul {
            if i == compiler_a {
                rapide.compiler();
            }
            if alea.chance(1, 20) {
                rapide = rapide.clone();
            }
            // Retour au calcul événementiel, puis de nouveau au code compilé.
            if i > compiler_a && alea.chance(1, 10) {
                rapide.ne_plus_compiler();
            } else if i > compiler_a && alea.chance(1, 4) {
                rapide.compiler();
            }
        }
        if alea.chance(1, 3) {
            for e in 0..nl.entrees.len() {
                if alea.chance(1, 2) {
                    let v = valeur_aleatoire(&mut alea);
                    rapide.fixer_entree(e, v);
                    exact.fixer_entree(e, v);
                }
            }
            assert_eq!(vue(&rapide), vue(&exact), "{}", contexte("entrées", i));
        }
        match alea.sous(100) {
            0..=79 => {
                let n = alea.entre(1, 3) * periode;
                rapide.avancer(n);
                exact.avancer(n);
            }
            80..=91 => {
                let n = alea.entre(1, 2 * periode);
                rapide.avancer(n);
                exact.avancer(n);
            }
            92..=98 => {
                let budget = alea.entre(0, 2 * periode);
                assert_eq!(
                    rapide.stabiliser(budget),
                    exact.stabiliser(budget),
                    "{}",
                    contexte("repos", i)
                );
            }
            _ => {
                let net = alea.sous(nl.nb_nets as u64) as NetId;
                assert_eq!(rapide.sonder(net), exact.sonder(net));
            }
        }
        assert_eq!(vue(&rapide), vue(&exact), "{}", contexte("avance", i));
        assert_eq!(
            rapide.vider_journal(),
            exact.vider_journal(),
            "{}",
            contexte("journal", i)
        );
    }
    let stats = rapide.statistiques();
    (stats.cycles_rapides, stats.cycles_compiles)
}

/// Joue une série d'essais et vérifie que le mode rapide a vraiment travaillé : sans
/// cela, l'égalité ne prouverait rien.
fn serie(graines: std::ops::Range<u64>, nb_appels: usize, part_min: f64, calcul: Calcul) {
    let mut cycles = 0u64;
    let mut compiles = 0u64;
    let mut essais_rapides = 0u64;
    let mut essais_compiles = 0u64;
    for graine in graines.clone() {
        let (c, k) = essai(graine, nb_appels, calcul);
        cycles += c;
        compiles += k;
        essais_rapides += u64::from(c > 0);
        essais_compiles += u64::from(k > 0);
    }
    let nb = (graines.end - graines.start) as f64;
    let part = essais_rapides as f64 / nb;
    println!(
        "graines {graines:?} ({calcul:?}) : {cycles} périodes en mode rapide dont {compiles} \
         compilées, {:.0} % des circuits en mode rapide, {:.0} % en code compilé",
        100.0 * part,
        100.0 * essais_compiles as f64 / nb
    );
    assert!(
        part >= part_min,
        "le mode rapide s'est trop peu engagé pour que l'essai prouve quelque chose"
    );
    match calcul {
        Calcul::Evenementiel => assert_eq!(compiles, 0),
        // Le code compilé doit avoir calculé l'essentiel des périodes rapides, et dans
        // presque tous les circuits qui passent en mode rapide.
        Calcul::Compile => assert!(
            compiles == cycles && essais_compiles == essais_rapides,
            "{compiles} {cycles}"
        ),
        Calcul::Mixte => assert!(
            compiles * 4 >= cycles && essais_compiles * 2 >= essais_rapides && compiles < cycles,
            "le code compilé s'est trop peu engagé"
        ),
    }
}

#[test]
fn circuits_synchrones_aleatoires() {
    serie(0..3000, 60, 0.5, Calcul::Evenementiel);
}

#[test]
fn circuits_synchrones_aleatoires_longs() {
    serie(100_000..100_200, 600, 0.5, Calcul::Evenementiel);
}

#[test]
fn circuits_synchrones_aleatoires_compiles() {
    serie(0..1000, 60, 0.5, Calcul::Compile);
}

#[test]
fn circuits_synchrones_aleatoires_mixtes() {
    serie(3000..4000, 60, 0.5, Calcul::Mixte);
}

#[test]
fn circuits_synchrones_aleatoires_longs_compiles() {
    serie(100_000..100_200, 600, 0.5, Calcul::Mixte);
}

/// Un compteur de 8 bits fait de bascules et d'additionneurs : le mode rapide s'engage
/// et compte comme le calcul porte par porte, période après période.
#[test]
fn un_compteur_passe_en_mode_rapide() {
    let mut alea = Alea::nouveau(7);
    let mut nl = Netlist::nouvelle();
    let c = nl.net();
    nl.horloge(c, 32);
    let q: Vec<(NetId, NetId)> = (0..8).map(|_| (nl.net(), nl.net())).collect();
    // Incrémenteur : d_i = q_i xor retenue_i, retenue_{i+1} = q_i and retenue_i.
    let un = nl.net();
    nl.constante(un, Valeur::Un);
    let mut retenue = un;
    let mut d = Vec::new();
    for &(qi, _) in &q {
        let n1 = nl.net();
        nl.nand(qi, retenue, n1, 1);
        let (a, b, x) = (nl.net(), nl.net(), nl.net());
        nl.nand(qi, n1, a, 1);
        nl.nand(retenue, n1, b, 1);
        nl.nand(a, b, x, 1);
        d.push(x);
        let r = nl.net();
        nl.nand(n1, n1, r, 1);
        retenue = r;
    }
    // Remise à zéro : les bascules partent de X ; une entrée force d à 0 au début.
    let raz = nl.net();
    nl.entree(raz);
    for (i, &(qi, qni)) in q.iter().enumerate() {
        let (m, dd) = (nl.net(), nl.net());
        nl.nand(d[i], raz, m, 1);
        nl.nand(m, m, dd, 1);
        bascule(&mut alea, &mut nl, c, dd, qi, qni, [1; 9]);
    }
    let mut rapide = Simulateur::nouveau(&nl).unwrap();
    let mut exact = Simulateur::avec_options(&nl, EXACT).unwrap();
    for s in [&mut rapide, &mut exact] {
        s.fixer_entree(0, Valeur::Zero);
        s.avancer(16);
        s.avancer(64);
        s.fixer_entree(0, Valeur::Un);
    }
    for cycle in 0..300u64 {
        rapide.avancer(64);
        exact.avancer(64);
        assert_eq!(vue(&rapide), vue(&exact), "cycle {cycle}");
        let compte: u64 = q
            .iter()
            .enumerate()
            .map(|(i, &(qi, _))| u64::from(rapide.valeur(qi) == Valeur::Un) << i)
            .sum();
        assert_eq!(compte, (cycle + 1) % 256, "cycle {cycle}");
    }
    assert!(
        rapide.statistiques().cycles_rapides >= 290,
        "{:?}",
        rapide.statistiques()
    );
}

/// Le grand essai, à lancer à la main avant une livraison qui touche au mode rapide :
/// `cargo test --profile ci -p ce-moteur --test rapide -- --ignored`.
#[test]
#[ignore]
fn circuits_synchrones_aleatoires_nombreux() {
    serie(1_000_000..1_050_000, 100, 0.5, Calcul::Evenementiel);
    serie(2_000_000..2_020_000, 100, 0.5, Calcul::Compile);
    serie(3_000_000..3_020_000, 100, 0.5, Calcul::Mixte);
}
