//! Le simulateur optimisé doit donner exactement les valeurs de la spécification,
//! à chaque instant, sur chaque net et pour chaque sortie de porte.

mod commun;

use ce_moteur::alea::{netlist_aleatoire, stimulus_aleatoire, Alea, ParamsNetlist};
use commun::{chaine, derouler, derouler_spec};

fn comparer(
    graine: u64,
    p: ParamsNetlist,
    duree: usize,
    pourcent_changement: u64,
    pourcent_xz: u64,
) {
    let mut alea = Alea::nouveau(graine);
    let nl = netlist_aleatoire(&mut alea, p);
    let stimulus = stimulus_aleatoire(
        &mut alea,
        p.nb_entrees,
        duree,
        pourcent_changement,
        pourcent_xz,
    );
    let (nets, sorties) = derouler(&nl, &stimulus);
    let spec = derouler_spec(&nl, &stimulus);
    for t in 0..=duree {
        assert!(
            nets[t] == spec.nets[t] && sorties[t] == spec.sorties[t],
            "graine {graine}, instant {t}\n  nets simulateur     {}\n  nets spécification  {}\n  sorties simulateur    {}\n  sorties spécification {}\n{nl:#?}",
            chaine(&nets[t]),
            chaine(&spec.nets[t]),
            chaine(&sorties[t]),
            chaine(&spec.sorties[t]),
        );
    }
}

fn params(
    nb_entrees: usize,
    nb_portes: usize,
    delai_max: u16,
    pourcent_tri: u64,
    pourcent_partage: u64,
    nb_constantes: usize,
    nb_horloges: usize,
) -> ParamsNetlist {
    ParamsNetlist {
        nb_entrees,
        nb_portes,
        delai_max,
        pourcent_tri,
        pourcent_partage,
        nb_constantes,
        nb_horloges,
    }
}

#[test]
fn nand_seules_delai_unite() {
    for graine in 0..400 {
        comparer(graine, params(3, 12, 1, 0, 0, 1, 0), 60, 20, 10);
    }
}

#[test]
fn nand_seules_delais_varies() {
    for graine in 1000..1600 {
        comparer(graine, params(3, 14, 5, 0, 0, 1, 0), 120, 30, 15);
    }
}

#[test]
fn trois_etats_et_pilotes_multiples() {
    for graine in 2000..2600 {
        comparer(graine, params(4, 14, 4, 35, 25, 2, 0), 120, 30, 20);
    }
}

#[test]
fn avec_horloges() {
    for graine in 3000..3300 {
        comparer(graine, params(2, 16, 6, 15, 10, 1, 2), 150, 10, 10);
    }
}

#[test]
fn grands_delais_et_entrees_agitees() {
    for graine in 4000..4300 {
        comparer(graine, params(3, 10, 17, 20, 15, 1, 1), 300, 45, 30);
    }
}

#[test]
fn circuits_plus_gros() {
    for graine in 5000..5040 {
        comparer(graine, params(8, 300, 7, 10, 5, 2, 1), 200, 15, 5);
    }
}
