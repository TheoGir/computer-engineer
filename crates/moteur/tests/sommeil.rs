//! La mise en sommeil des régions ne change rien de ce qu'on peut observer.
//!
//! Chaque essai tire un circuit quelconque avec des horloges, un découpage en régions
//! quelconque et une suite d'appels quelconque (changer une entrée, avancer d'un ou de
//! plusieurs instants, attendre le repos, poser une sonde). La même suite est jouée
//! sur deux simulateurs, l'un avec le sommeil et l'autre sans. Après chaque appel,
//! tout ce que l'interface publique laisse voir doit être identique : le temps, la
//! valeur de chaque net, l'état complet de chaque porte (sortie, cible, échéance), le
//! repos, les courts-circuits, le journal des sondes. Les valeurs sont aussi comparées
//! à la spécification, instant par instant.
//!
//! C'est la vérification du théorème 5 de `docs/semantique.md` : la preuve y est
//! écrite, ces essais la confrontent à des millions de cas.

use ce_moteur::alea::{netlist_aleatoire, Alea, ParamsNetlist};
use ce_moteur::specification::simuler;
use ce_moteur::{Netlist, Options, Simulateur, StatistiquesSommeil, Valeur};

/// Découpe les portes en régions quelconques : des plages disjointes, de tailles
/// variées, séparées ou non par des portes hors région.
fn regions_aleatoires(alea: &mut Alea, nl: &mut Netlist, taille_max: u64) {
    let nb = nl.portes.len() as u32;
    let mut g = 0u32;
    while g < nb {
        if alea.chance(1, 4) {
            g += alea.entre(1, 3) as u32;
            continue;
        }
        let fin = (g + alea.entre(1, taille_max) as u32).min(nb);
        nl.region(g..fin);
        g = fin;
    }
}

fn valeur_aleatoire(alea: &mut Alea, pourcent_xz: u64) -> Valeur {
    if alea.chance(pourcent_xz, 100) {
        if alea.chance(2, 3) {
            Valeur::X
        } else {
            Valeur::Z
        }
    } else {
        Valeur::de_bool(alea.chance(1, 2))
    }
}

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

/// Réglages d'une famille d'essais.
#[derive(Clone, Copy)]
struct Famille {
    params: ParamsNetlist,
    /// Taille maximale d'une région.
    taille_region: u64,
    /// Nombre d'appels joués.
    nb_appels: usize,
    /// Probabilité (pour mille) de changer une entrée avant chaque avance.
    pour_mille_entree: u64,
    pourcent_xz: u64,
    /// Les avances font de 1 à `pas_max` instants.
    pas_max: u64,
    /// Probabilité (pour mille) d'attendre le repos au lieu d'avancer.
    pour_mille_stabiliser: u64,
    /// Probabilité (pour mille) de poser une sonde.
    pour_mille_sonde: u64,
    /// Durée au-delà de laquelle l'essai s'arrête (la spécification est lente).
    duree_max: u64,
}

/// Joue un essai et rend les compteurs de sommeil.
fn essai(graine: u64, f: Famille) -> StatistiquesSommeil {
    let mut alea = Alea::nouveau(graine);
    let mut nl = netlist_aleatoire(&mut alea, f.params);
    regions_aleatoires(&mut alea, &mut nl, f.taille_region);
    let mut avec = Simulateur::nouveau(&nl).expect("netlist valide");
    let mut sans = Simulateur::avec_options(
        &nl,
        Options {
            sommeil: false,
            rapide: false,
            compilation: false,
        },
    )
    .expect("valide");
    let nb_entrees = nl.entrees.len();

    // Valeur de chaque entrée à chaque instant, pour la spécification.
    let mut entrees = vec![Valeur::Z; nb_entrees];
    let mut stimulus: Vec<Vec<Valeur>> = Vec::new();
    // Instants où l'on s'est arrêté : les valeurs y sont comparées à la spécification.
    let mut arrets: Vec<(u64, Vec<u8>, Vec<Valeur>)> = Vec::new();

    let contexte = |quoi: &str, i: usize| format!("graine {graine}, appel {i} ({quoi})");
    for i in 0..f.nb_appels {
        if avec.temps() >= f.duree_max {
            break;
        }
        // Les entrées changent rarement : les régions ont le temps de s'endormir.
        for (e, valeur) in entrees.iter_mut().enumerate() {
            if i == 0 || alea.chance(f.pour_mille_entree, 1000) {
                let v = valeur_aleatoire(&mut alea, f.pourcent_xz);
                *valeur = v;
                avec.fixer_entree(e, v);
                sans.fixer_entree(e, v);
            }
        }
        assert_eq!(vue(&avec), vue(&sans), "{}", contexte("entrées", i));
        // Un seul relevé par instant : celui d'après les derniers changements d'entrées.
        if arrets.last().is_some_and(|a| a.0 == avec.temps()) {
            arrets.pop();
        }
        arrets.push((
            avec.temps(),
            avec.codes().to_vec(),
            (0..nl.portes.len()).map(|g| avec.sortie_porte(g)).collect(),
        ));

        if alea.chance(f.pour_mille_sonde, 1000) {
            let net = alea.sous(nl.nb_nets as u64) as u32;
            assert_eq!(avec.sonder(net), sans.sonder(net));
            assert_eq!(vue(&avec), vue(&sans), "{}", contexte("sonde", i));
        }

        let avant = avec.temps();
        if alea.chance(f.pour_mille_stabiliser, 1000) {
            let budget = alea.entre(0, 3 * f.pas_max);
            assert_eq!(
                avec.stabiliser(budget),
                sans.stabiliser(budget),
                "{}",
                contexte("stabiliser", i)
            );
        } else {
            let pas = if alea.chance(3, 4) {
                1
            } else {
                alea.entre(1, f.pas_max)
            };
            avec.avancer(pas);
            sans.avancer(pas);
        }
        assert_eq!(vue(&avec), vue(&sans), "{}", contexte("avance", i));
        assert_eq!(
            avec.vider_journal(),
            sans.vider_journal(),
            "{}",
            contexte("journal", i)
        );
        for _ in avant..avec.temps() {
            stimulus.push(entrees.clone());
        }
    }

    // Contre la spécification : mêmes valeurs à chaque instant où l'on s'est arrêté.
    stimulus.push(entrees.clone());
    let spec = simuler(&nl, stimulus.len() - 1, &|t, e| stimulus[t][e]);
    for (t, nets, sorties) in &arrets {
        let t = *t as usize;
        let attendus: Vec<u8> = spec.nets[t].iter().map(|v| v.code()).collect();
        assert!(
            *nets == attendus && *sorties == spec.sorties[t],
            "graine {graine}, instant {t} : le simulateur s'écarte de la spécification"
        );
    }
    avec.sommeil()
}

fn params(
    nb_entrees: usize,
    nb_portes: usize,
    delai_max: u16,
    pourcent_tri: u64,
    pourcent_partage: u64,
    nb_horloges: usize,
) -> ParamsNetlist {
    ParamsNetlist {
        nb_entrees,
        nb_portes,
        delai_max,
        pourcent_tri,
        pourcent_partage,
        nb_constantes: 2,
        nb_horloges,
    }
}

/// Joue une famille d'essais et vérifie qu'elle a vraiment fait dormir des régions :
/// sans cela, l'égalité ne prouverait rien.
fn famille(graines: std::ops::Range<u64>, f: Famille, sommeils_min: u64, reveils_min: u64) {
    let mut total = StatistiquesSommeil::default();
    for graine in graines.clone() {
        let s = essai(graine, f);
        total.endormissements += s.endormissements;
        total.reveils += s.reveils;
        total.observations_ratees += s.observations_ratees;
    }
    println!(
        "graines {graines:?} : {} mises en sommeil, {} réveils, {} observations sans suite",
        total.endormissements, total.reveils, total.observations_ratees
    );
    assert!(
        total.endormissements >= sommeils_min && total.reveils >= reveils_min,
        "trop peu de sommeils pour que l'essai prouve quelque chose : {total:?}"
    );
}

const BASE: Famille = Famille {
    params: ParamsNetlist {
        nb_entrees: 3,
        nb_portes: 16,
        delai_max: 1,
        pourcent_tri: 0,
        pourcent_partage: 0,
        nb_constantes: 2,
        nb_horloges: 2,
    },
    taille_region: 6,
    nb_appels: 400,
    pour_mille_entree: 15,
    pourcent_xz: 10,
    pas_max: 40,
    pour_mille_stabiliser: 0,
    pour_mille_sonde: 0,
    duree_max: 1500,
};

#[test]
fn nand_delai_unite_une_horloge() {
    famille(0..4000, BASE, 5000, 4000);
}

#[test]
fn delais_varies_deux_horloges() {
    let f = Famille {
        params: params(3, 18, 4, 0, 0, 3),
        ..BASE
    };
    famille(10_000..14_000, f, 7000, 5000);
}

#[test]
fn trois_etats_et_pilotes_multiples() {
    let f = Famille {
        params: params(4, 18, 3, 35, 25, 3),
        pourcent_xz: 20,
        ..BASE
    };
    famille(20_000..24_000, f, 7000, 5000);
}

#[test]
fn grandes_regions_et_grands_pas() {
    let f = Famille {
        params: params(4, 40, 2, 20, 10, 3),
        taille_region: 20,
        pas_max: 200,
        nb_appels: 300,
        duree_max: 2500,
        ..BASE
    };
    famille(30_000..31_500, f, 1500, 1000);
}

#[test]
fn attendre_le_repos() {
    let f = Famille {
        params: params(3, 16, 3, 20, 10, 2),
        pour_mille_stabiliser: 150,
        ..BASE
    };
    famille(40_000..44_000, f, 7000, 6000);
}

#[test]
fn sondes_posees_en_route() {
    let f = Famille {
        params: params(3, 16, 2, 20, 10, 2),
        pour_mille_sonde: 20,
        ..BASE
    };
    famille(50_000..54_000, f, 6000, 5000);
}

/// Applique la même action aux deux simulateurs et vérifie qu'ils restent identiques.
fn les_deux(
    sim: &mut Simulateur,
    temoin: &mut Simulateur,
    f: &mut dyn FnMut(&mut Simulateur),
) -> StatistiquesSommeil {
    f(sim);
    f(temoin);
    assert_eq!(vue(sim), vue(temoin));
    sim.sommeil()
}

/// Une région endormie est réveillée par l'entrée qu'elle surveille, et pas par une
/// entrée masquée : un registre à chargement, construit à la main.
#[test]
fn un_registre_dort_tant_qu_on_ne_le_charge_pas() {
    let mut nl = Netlist::nouvelle();
    let (d, charge, clk) = (nl.net(), nl.net(), nl.net());
    nl.entree(d);
    nl.entree(charge);
    nl.horloge(clk, 20);
    // Hors région : l'inverse de l'horloge.
    let nclk = nl.net();
    nl.nand(clk, clk, nclk, 1);
    let debut = nl.portes.len() as u32;
    // Multiplexeur : x = charge ? d : q.
    let (ncharge, m0, m1, x, q, nq) = (nl.net(), nl.net(), nl.net(), nl.net(), nl.net(), nl.net());
    nl.nand(charge, charge, ncharge, 1);
    nl.nand(d, charge, m0, 1);
    nl.nand(q, ncharge, m1, 1);
    nl.nand(m0, m1, x, 1);
    // Bascule maître-esclave en verrous NAND.
    let (nx, s1, r1, a, na) = (nl.net(), nl.net(), nl.net(), nl.net(), nl.net());
    nl.nand(x, x, nx, 1);
    nl.nand(x, nclk, s1, 1);
    nl.nand(nx, nclk, r1, 1);
    nl.nand(s1, na, a, 1);
    nl.nand(r1, a, na, 1);
    let (s2, r2) = (nl.net(), nl.net());
    nl.nand(a, clk, s2, 1);
    nl.nand(na, clk, r2, 1);
    nl.nand(s2, nq, q, 1);
    nl.nand(r2, q, nq, 1);
    nl.region(debut..nl.portes.len() as u32);
    // Hors région : une porte qui lit la sortie.
    let lu = nl.net();
    nl.nand(q, q, lu, 1);

    let mut sim = Simulateur::nouveau(&nl).unwrap();
    let mut temoin = Simulateur::avec_options(
        &nl,
        Options {
            sommeil: false,
            rapide: false,
            compilation: false,
        },
    )
    .unwrap();
    // Chargement de 1, puis repos : le registre s'endort.
    les_deux(&mut sim, &mut temoin, &mut |s| {
        s.fixer_entree(0, Valeur::Un);
        s.fixer_entree(1, Valeur::Un);
        s.avancer(50);
        s.fixer_entree(1, Valeur::Zero);
        s.avancer(400);
    });
    let stats = les_deux(&mut sim, &mut temoin, &mut |s| s.avancer(1));
    assert_eq!(stats.endormies, 1, "le registre au repos doit dormir");
    // La donnée bouge, le chargement reste à 0 : la donnée est masquée, il dort toujours.
    let reveils = stats.reveils;
    for k in 0..40 {
        let stats = les_deux(&mut sim, &mut temoin, &mut |s| {
            s.fixer_entree(0, Valeur::de_bool(k % 2 == 0));
            s.avancer(7);
        });
        assert_eq!((stats.endormies, stats.reveils), (1, reveils));
    }
    // Chargement de 0 : il se réveille, prend la valeur, puis se rendort.
    let stats = les_deux(&mut sim, &mut temoin, &mut |s| {
        s.fixer_entree(0, Valeur::Zero);
        s.fixer_entree(1, Valeur::Un);
    });
    assert_eq!((stats.endormies, stats.reveils), (0, reveils + 1));
    les_deux(&mut sim, &mut temoin, &mut |s| s.avancer(40));
    assert_eq!(sim.valeur(q), Valeur::Zero);
    let stats = les_deux(&mut sim, &mut temoin, &mut |s| {
        s.fixer_entree(1, Valeur::Zero);
        s.avancer(2000);
    });
    assert_eq!(stats.endormies, 1);
    assert_eq!(sim.valeur(q), Valeur::Zero);
    // Endormi, le simulateur ne travaille presque plus : seul l'inverseur de l'horloge bat.
    let avant = sim.statistiques().evaluations;
    les_deux(&mut sim, &mut temoin, &mut |s| s.avancer(4000));
    let par_periode = (sim.statistiques().evaluations - avant) / 100;
    assert!(
        par_periode <= 2,
        "{par_periode} évaluations par période pour un registre endormi"
    );
}
