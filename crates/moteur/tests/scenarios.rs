//! Scénarios dont le résultat attendu est calculé à la main, instant par instant.

mod commun;

use ce_moteur::alea::{netlist_aleatoire, stimulus_aleatoire, Alea, ParamsNetlist};
use ce_moteur::{Netlist, Simulateur, Stabilite, Valeur};
use commun::derouler;
use Valeur::{Un, Zero, X, Z};

/// Inverseur (NAND aux entrées reliées) de délai `d` : rend (netlist, entrée, sortie).
fn inverseur(d: u16) -> (Netlist, u32, u32) {
    let mut nl = Netlist::nouvelle();
    let (a, s) = (nl.net(), nl.net());
    nl.entree(a);
    nl.nand(a, a, s, d);
    (nl, a, s)
}

/// Relève la valeur de `net` de l'instant courant à `fin` inclus, sous forme `01xz`.
fn releve(sim: &mut Simulateur, net: u32, fin: u64) -> String {
    let mut s = String::new();
    loop {
        s.push(sim.valeur(net).symbole());
        if sim.temps() >= fin {
            return s;
        }
        sim.avancer(1);
    }
}

#[test]
fn le_delai_de_propagation_est_respecte() {
    let (nl, _, s) = inverseur(3);
    let mut sim = Simulateur::nouveau(&nl).unwrap();
    assert_eq!(sim.valeur(s), X, "inconnu à la mise sous tension");
    sim.fixer_entree(0, Zero);
    // Entrée à 0 depuis t=0 : sortie à 1 à t=3 exactement.
    assert_eq!(releve(&mut sim, s, 5), "xxx111");
    sim.fixer_entree(0, Un); // t=5
    assert_eq!(releve(&mut sim, s, 9), "11100");
}

#[test]
fn une_impulsion_plus_courte_que_le_delai_est_filtree() {
    let (nl, _, s) = inverseur(3);
    let mut sim = Simulateur::nouveau(&nl).unwrap();
    sim.fixer_entree(0, Zero);
    sim.avancer(10);
    assert_eq!(sim.valeur(s), Un);
    // Impulsion de largeur 2 (< 3) : rien ne sort.
    sim.fixer_entree(0, Un);
    sim.avancer(2);
    sim.fixer_entree(0, Zero);
    assert_eq!(releve(&mut sim, s, 20), "111111111");
    assert_eq!(sim.statistiques().impulsions_filtrees, 1);
    // Impulsion de largeur 3 (= délai) : elle sort, décalée de 3, large de 3.
    sim.fixer_entree(0, Un); // t=20
    sim.avancer(3);
    sim.fixer_entree(0, Zero); // t=23
    assert_eq!(releve(&mut sim, s, 28), "000111");
}

#[test]
fn deux_changements_au_meme_instant_ne_creent_pas_d_impulsion() {
    let (nl, _, s) = inverseur(1);
    let mut sim = Simulateur::nouveau(&nl).unwrap();
    sim.fixer_entree(0, Zero);
    sim.avancer(5);
    let avant = sim.statistiques();
    sim.fixer_entree(0, Un);
    sim.fixer_entree(0, Zero);
    sim.avancer(5);
    assert_eq!(sim.valeur(s), Un);
    assert_eq!(
        sim.statistiques().tirs,
        avant.tirs,
        "aucun changement de sortie"
    );
}

/// Un X plus court que le délai ne doit pas masquer une impulsion possible.
///
/// Inverseur de délai 2, entrée `1 1 x 0 x 1 1 1`. Si les deux `x` valaient `0`,
/// l'entrée serait à 0 pendant trois instants et la sortie monterait à 1. Le
/// simulateur ne sait pas, donc il doit dire `x`, pas `0`. (Un simulateur Verilog
/// classique annonce `0` tout du long.)
#[test]
fn le_x_ne_cache_pas_une_impulsion() {
    let (nl, _, _) = inverseur(2);
    let flou = [Un, Un, X, Zero, X, Un, Un, Un, Un];
    let precis = [Un, Un, Zero, Zero, Zero, Un, Un, Un, Un];
    let vers_stimulus = |s: &[Valeur]| s.iter().map(|&v| vec![v]).collect::<Vec<_>>();
    let (nets_flous, _) = derouler(&nl, &vers_stimulus(&flou));
    let (nets_precis, _) = derouler(&nl, &vers_stimulus(&precis));
    let sortie = |nets: &[Vec<Valeur>]| nets.iter().map(|l| l[1].symbole()).collect::<String>();
    assert_eq!(sortie(&nets_precis), "xx0011100");
    assert_eq!(sortie(&nets_flous), "xx00xxx00");
}

/// Multiplexeur en NAND, `a = b = 1` : quand `sel` passe de 1 à 0 la sortie devrait
/// rester à 1, mais le chemin par l'inverseur est plus long d'une porte. Aléa
/// statique : la sortie tombe à 0 pendant une unité de temps.
#[test]
fn alea_statique_d_un_multiplexeur() {
    let mut nl = Netlist::nouvelle();
    let (a, b, sel) = (nl.net(), nl.net(), nl.net());
    let (n, p, q, s) = (nl.net(), nl.net(), nl.net(), nl.net());
    for e in [a, b, sel] {
        nl.entree(e);
    }
    nl.nand(sel, sel, n, 1);
    nl.nand(a, sel, p, 1);
    nl.nand(b, n, q, 1);
    nl.nand(p, q, s, 1);
    let mut sim = Simulateur::nouveau(&nl).unwrap();
    sim.fixer_entree(0, Un);
    sim.fixer_entree(1, Un);
    sim.fixer_entree(2, Un);
    // Au repos dès t=2 : la sortie y prend sa valeur, et la dernière porte marquée
    // (celle de la sortie, quand q passe à 1) n'a rien à changer.
    assert_eq!(sim.stabiliser(100), Stabilite::Stable { t: 2 });
    assert_eq!(sim.valeur(s), Un);
    sim.avancer(8); // t=10
    sim.fixer_entree(2, Zero);
    assert_eq!(releve(&mut sim, s, 15), "110111");
}

/// Oscillateur en anneau : NAND(validation, c) puis deux inverseurs.
fn anneau(d: u16) -> (Netlist, [u32; 3]) {
    let mut nl = Netlist::nouvelle();
    let (en, a, b, c) = (nl.net(), nl.net(), nl.net(), nl.net());
    nl.entree(en);
    nl.nand(en, c, a, d);
    nl.nand(a, a, b, d);
    nl.nand(b, b, c, d);
    (nl, [a, b, c])
}

#[test]
fn un_anneau_d_inverseurs_oscille() {
    let (nl, [a, _, _]) = anneau(1);
    let mut sim = Simulateur::nouveau(&nl).unwrap();
    sim.fixer_entree(0, Zero);
    assert!(sim.stabiliser(100).est_stable());
    assert_eq!(sim.valeur(a), Un);
    sim.avancer(10 - sim.temps());
    sim.fixer_entree(0, Un); // t=10
                             // Trois portes de délai 1 : période 6.
    assert_eq!(releve(&mut sim, a, 28), "1000111000111000111");
    assert_eq!(sim.stabiliser(10_000), Stabilite::Instable { t: 10_028 });
}

#[test]
fn la_periode_de_l_anneau_suit_les_delais() {
    let (nl, [a, _, _]) = anneau(4);
    let mut sim = Simulateur::nouveau(&nl).unwrap();
    let sonde = sim.sonder(a);
    sim.fixer_entree(0, Zero);
    sim.avancer(100);
    sim.fixer_entree(0, Un);
    sim.avancer(240);
    let fronts: Vec<u64> = sim
        .vider_journal()
        .iter()
        .filter(|c| c.sonde == sonde && c.valeur == Un && c.t > 100)
        .map(|c| c.t)
        .collect();
    assert!(fronts.len() >= 5);
    for paire in fronts.windows(2) {
        assert_eq!(paire[1] - paire[0], 24, "période = 2 × 3 portes × délai 4");
    }
}

#[test]
fn un_anneau_jamais_initialise_reste_inconnu() {
    let (nl, [a, b, c]) = anneau(1);
    let mut sim = Simulateur::nouveau(&nl).unwrap();
    sim.fixer_entree(0, Un);
    assert!(sim.stabiliser(100).est_stable());
    assert_eq!([sim.valeur(a), sim.valeur(b), sim.valeur(c)], [X, X, X]);
}

/// Verrou SR en NAND (entrées actives à 0) : rend (netlist, q, q_barre).
fn verrou_sr(d_q: u16, d_qn: u16) -> (Netlist, u32, u32) {
    let mut nl = Netlist::nouvelle();
    let (s_n, r_n, q, qn) = (nl.net(), nl.net(), nl.net(), nl.net());
    nl.entree(s_n);
    nl.entree(r_n);
    nl.nand(s_n, qn, q, d_q);
    nl.nand(r_n, q, qn, d_qn);
    (nl, q, qn)
}

#[test]
fn verrou_sr_memorise() {
    let (nl, q, qn) = verrou_sr(1, 1);
    let mut sim = Simulateur::nouveau(&nl).unwrap();
    let appliquer = |sim: &mut Simulateur, s_n, r_n| {
        sim.fixer_entree(0, s_n);
        sim.fixer_entree(1, r_n);
        assert!(sim.stabiliser(100).est_stable());
        (sim.valeur(q), sim.valeur(qn))
    };
    assert_eq!(
        appliquer(&mut sim, Un, Un),
        (X, X),
        "jamais écrit : inconnu"
    );
    assert_eq!(appliquer(&mut sim, Zero, Un), (Un, Zero), "mise à 1");
    assert_eq!(appliquer(&mut sim, Un, Un), (Un, Zero), "mémoire");
    assert_eq!(appliquer(&mut sim, Un, Zero), (Zero, Un), "mise à 0");
    assert_eq!(appliquer(&mut sim, Un, Un), (Zero, Un), "mémoire");
    assert_eq!(appliquer(&mut sim, Zero, Zero), (Un, Un), "état interdit");
}

#[test]
fn verrou_sr_relache_simultanement_oscille_si_les_delais_sont_egaux() {
    let (nl, q, _) = verrou_sr(1, 1);
    let mut sim = Simulateur::nouveau(&nl).unwrap();
    sim.fixer_entree(0, Zero);
    sim.fixer_entree(1, Zero);
    assert!(sim.stabiliser(100).est_stable());
    sim.fixer_entree(0, Un);
    sim.fixer_entree(1, Un);
    assert!(!sim.stabiliser(1000).est_stable());
    let t = sim.temps();
    assert_eq!(releve(&mut sim, q, t + 5).len(), 6);
    let v0 = sim.valeur(q);
    sim.avancer(1);
    assert_ne!(sim.valeur(q), v0, "q bascule à chaque instant");
}

#[test]
fn verrou_sr_relache_simultanement_la_porte_la_plus_rapide_gagne() {
    let (nl, q, qn) = verrou_sr(1, 2);
    let mut sim = Simulateur::nouveau(&nl).unwrap();
    sim.fixer_entree(0, Zero);
    sim.fixer_entree(1, Zero);
    assert!(sim.stabiliser(100).est_stable());
    sim.fixer_entree(0, Un);
    sim.fixer_entree(1, Un);
    assert!(sim.stabiliser(100).est_stable());
    assert_eq!((sim.valeur(q), sim.valeur(qn)), (Zero, Un));
}

/// Bus à deux pilotes trois états, lu par un inverseur.
fn bus() -> (Netlist, u32, u32) {
    let mut nl = Netlist::nouvelle();
    let (en_a, a, en_b, b, bus, lu) = (nl.net(), nl.net(), nl.net(), nl.net(), nl.net(), nl.net());
    for e in [en_a, a, en_b, b] {
        nl.entree(e);
    }
    nl.tri(en_a, a, bus, 1);
    nl.tri(en_b, b, bus, 1);
    nl.nand(bus, bus, lu, 1);
    (nl, bus, lu)
}

#[test]
fn bus_trois_etats() {
    let (nl, bus, lu) = bus();
    let mut sim = Simulateur::nouveau(&nl).unwrap();
    let appliquer = |sim: &mut Simulateur, v: [Valeur; 4]| {
        for (i, &x) in v.iter().enumerate() {
            sim.fixer_entree(i, x);
        }
        assert!(sim.stabiliser(100).est_stable());
        (sim.valeur(bus), sim.valeur(lu), sim.conflits().len())
    };
    assert_eq!(
        appliquer(&mut sim, [Un, Un, Zero, Zero]),
        (Un, Zero, 0),
        "A parle"
    );
    assert_eq!(
        appliquer(&mut sim, [Zero, Un, Un, Zero]),
        (Zero, Un, 0),
        "B parle"
    );
    assert_eq!(
        appliquer(&mut sim, [Zero, Un, Zero, Zero]),
        (Z, X, 0),
        "personne : bus flottant, lu inconnu"
    );
    assert_eq!(
        appliquer(&mut sim, [Un, Un, Un, Un]),
        (Un, Zero, 0),
        "deux pilotes d'accord"
    );
    assert_eq!(sim.premier_conflit(), None);
    assert_eq!(
        appliquer(&mut sim, [Un, Un, Un, Zero]),
        (X, X, 1),
        "court-circuit"
    );
    sim.avancer(1);
    let (t, net) = sim.premier_conflit().expect("conflit enregistré");
    assert_eq!(net, bus);
    assert!(t > 0);
    assert_eq!(
        appliquer(&mut sim, [Un, Un, X, Zero]),
        (X, X, 0),
        "validation inconnue : bus inconnu, pas de conflit franc"
    );
}

#[test]
fn deux_sorties_de_portes_reliees_en_court_circuit() {
    let mut nl = Netlist::nouvelle();
    let (a, s) = (nl.net(), nl.net());
    nl.entree(a);
    let n = nl.net();
    nl.nand(a, a, n, 1);
    nl.nand(a, a, s, 1); // pilote s avec non(a)
    nl.nand(n, n, s, 1); // pilote s avec a
    let mut sim = Simulateur::nouveau(&nl).unwrap();
    sim.fixer_entree(0, Un);
    assert!(sim.stabiliser(100).est_stable());
    assert_eq!(sim.valeur(s), X);
    assert_eq!(sim.conflits(), vec![s]);
}

#[test]
fn horloge_et_journal() {
    let mut nl = Netlist::nouvelle();
    let (clk, s) = (nl.net(), nl.net());
    nl.horloge(clk, 5);
    nl.nand(clk, clk, s, 2);
    let mut sim = Simulateur::nouveau(&nl).unwrap();
    let sc = sim.sonder(clk);
    let ss = sim.sonder(s);
    sim.avancer(21);
    let journal = sim.vider_journal();
    let de = |sonde: u32| -> Vec<(u64, char)> {
        journal
            .iter()
            .filter(|c| c.sonde == sonde)
            .map(|c| (c.t, c.valeur.symbole()))
            .collect()
    };
    assert_eq!(
        de(sc),
        [(0, '0'), (5, '1'), (10, '0'), (15, '1'), (20, '0')]
    );
    assert_eq!(de(ss), [(0, 'x'), (2, '1'), (7, '0'), (12, '1'), (17, '0')]);
    // À t=21 l'inverseur doit encore réagir au front de t=20 : repos à t=22,
    // en attendant le front suivant.
    assert_eq!(sim.stabiliser(1000), Stabilite::Stable { t: 22 });
}

#[test]
fn stabiliser_n_avance_pas_un_circuit_au_repos() {
    let (nl, _, s) = inverseur(4);
    let mut sim = Simulateur::nouveau(&nl).unwrap();
    sim.fixer_entree(0, Un);
    assert_eq!(sim.stabiliser(100), Stabilite::Stable { t: 4 });
    assert_eq!(sim.stabiliser(100), Stabilite::Stable { t: 4 });
    sim.avancer(0);
    assert_eq!(sim.temps(), 4);
    assert_eq!(sim.valeur(s), Zero);
    assert!(sim.est_stable());
    sim.avancer(1_000_000_000_000);
    assert_eq!(
        sim.temps(),
        1_000_000_000_004,
        "le temps saute quand rien ne se passe"
    );
}

#[test]
fn netlist_invalide_refusee() {
    let mut nl = Netlist::nouvelle();
    let a = nl.net();
    nl.nand(a, a, 7, 1);
    assert!(Simulateur::nouveau(&nl).is_err());
    let mut nl = Netlist::nouvelle();
    let a = nl.net();
    nl.nand(a, a, a, 0);
    assert!(Simulateur::nouveau(&nl).is_err());
}

/// Le résultat ne dépend pas de l'ordre dans lequel les portes sont rangées.
#[test]
fn le_resultat_ne_depend_pas_de_l_ordre_des_portes() {
    for graine in 0..200u64 {
        let mut alea = Alea::nouveau(777 + graine);
        let p = ParamsNetlist {
            nb_entrees: 3,
            nb_portes: 20,
            delai_max: 4,
            pourcent_tri: 20,
            pourcent_partage: 15,
            nb_constantes: 1,
            nb_horloges: 1,
        };
        let nl = netlist_aleatoire(&mut alea, p);
        let stimulus = stimulus_aleatoire(&mut alea, p.nb_entrees, 80, 25, 15);
        let mut melange = nl.clone();
        for i in (1..melange.portes.len()).rev() {
            let j = alea.sous(i as u64 + 1) as usize;
            melange.portes.swap(i, j);
        }
        assert_eq!(
            derouler(&nl, &stimulus).0,
            derouler(&melange, &stimulus).0,
            "graine {graine}"
        );
    }
}
