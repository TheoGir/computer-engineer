//! Sur de vrais circuits, la mise en sommeil des registres ne change rien : le même
//! circuit simulé avec et sans sommeil passe par les mêmes états, fil par fil et porte
//! par porte, et le sommeil fait bien l'économie attendue.

use ce_circuit::grands::MemoireEnMarche;
use ce_circuit::ordinateur::{assembler, programmes, Machine, Reglages};
use ce_moteur::alea::Alea;
use ce_moteur::{Options, Simulateur};

const SANS: Options = Options {
    sommeil: false,
    rapide: false,
    compilation: false,
};

/// Le sommeil seul : le mode rapide, qui calcule tout le circuit période par période,
/// est essayé à part (`tests/rapide.rs`).
const AVEC: Options = Options {
    sommeil: true,
    rapide: false,
    compilation: false,
};

/// Vérifie que les deux simulateurs sont dans le même état : chaque fil, chaque porte
/// (sortie, cible, échéance), le repos et les courts-circuits.
fn memes_etats(avec: &Simulateur, sans: &Simulateur, contexte: &str) {
    assert_eq!(avec.temps(), sans.temps(), "temps, {contexte}");
    assert!(avec.codes() == sans.codes(), "valeurs des fils, {contexte}");
    for g in 0..avec.nb_portes() {
        assert_eq!(
            avec.etat_relatif_porte(g),
            sans.etat_relatif_porte(g),
            "porte {g}, {contexte}"
        );
    }
    assert_eq!(avec.est_stable(), sans.est_stable(), "repos, {contexte}");
    assert_eq!(
        avec.conflits(),
        sans.conflits(),
        "courts-circuits, {contexte}"
    );
    assert_eq!(avec.premier_conflit(), sans.premier_conflit(), "{contexte}");
}

#[test]
fn l_ordinateur_passe_par_les_memes_etats_avec_et_sans_sommeil() {
    for (nom, source, touches, bits_ram) in [
        ("fibonacci", programmes::FIBONACCI.to_string(), "", 4),
        (
            "calculatrice",
            programmes::CALCULATRICE.to_string(),
            "67",
            4,
        ),
        ("majuscules", programmes::MAJUSCULES.to_string(), "nand", 6),
    ] {
        let programme = assembler(&source).unwrap();
        let reglages = Reglages {
            bits_ram,
            ..Reglages::default()
        };
        let mut avec = Machine::avec_options(&programme, reglages, AVEC).unwrap();
        let mut sans = Machine::avec_options(&programme, reglages, SANS).unwrap();
        let touches = touches.as_bytes();
        let mut alea = Alea::nouveau(7);
        for cycle in 0..400 {
            // Chaque touche reste enfoncée 12 cycles, puis est relâchée 12 cycles.
            let touche = if cycle % 24 < 12 {
                touches.get(cycle / 24).copied().unwrap_or(0)
            } else {
                0
            };
            assert_eq!(avec.cycle(touche), sans.cycle(touche), "{nom}, écran");
            memes_etats(
                &avec.banc.sim,
                &sans.banc.sim,
                &format!("{nom}, cycle {cycle}"),
            );
            if alea.chance(1, 8) {
                // On s'arrête aussi en plein cycle, là où les registres basculent.
                let pas = alea.entre(1, 2 * reglages.demi_periode - 1);
                for banc in [&mut avec.banc, &mut sans.banc] {
                    banc.avancer(pas);
                }
                memes_etats(
                    &avec.banc.sim,
                    &sans.banc.sim,
                    &format!("{nom}, en plein cycle {cycle}"),
                );
                for banc in [&mut avec.banc, &mut sans.banc] {
                    banc.avancer(2 * reglages.demi_periode - pas);
                }
                memes_etats(
                    &avec.banc.sim,
                    &sans.banc.sim,
                    &format!("{nom}, après le cycle {cycle}"),
                );
            }
        }
        assert_eq!(avec.ecran, sans.ecran, "{nom}");
        let (s, travail_avec) = (avec.banc.sim.sommeil(), avec.banc.sim.statistiques());
        let travail_sans = sans.banc.sim.statistiques();
        assert!(s.endormissements > 0 && s.reveils > 0, "{nom} : {s:?}");
        assert!(
            travail_avec.evaluations < travail_sans.evaluations,
            "{nom} : le sommeil doit économiser des évaluations"
        );
    }
}

#[test]
fn la_memoire_passe_par_les_memes_etats_avec_et_sans_sommeil() {
    let bits = 7;
    let mut avec = MemoireEnMarche::avec_options(bits, 64, AVEC).unwrap();
    let mut sans = MemoireEnMarche::avec_options(bits, 64, SANS).unwrap();
    let mut alea = Alea::nouveau(11);
    for cycle in 0..1500 {
        let adresse = alea.sous(1 << bits) as usize;
        let (d, ecrire) = (alea.u64() as u8, alea.chance(1, 4));
        let lu = avec.cycle(adresse, d, ecrire);
        assert_eq!(lu, sans.cycle(adresse, d, ecrire));
        assert_eq!(lu, avec.modele[adresse], "cycle {cycle}");
        if cycle % 50 == 0 {
            memes_etats(&avec.banc.sim, &sans.banc.sim, &format!("cycle {cycle}"));
        }
        if alea.chance(1, 20) {
            let pas = alea.entre(1, 127);
            for banc in [&mut avec.banc, &mut sans.banc] {
                banc.avancer(pas);
            }
            memes_etats(
                &avec.banc.sim,
                &sans.banc.sim,
                &format!("en plein cycle {cycle}"),
            );
            for banc in [&mut avec.banc, &mut sans.banc] {
                banc.avancer(128 - pas);
            }
        }
    }
    memes_etats(&avec.banc.sim, &sans.banc.sim, "fin");
    // La plupart des mots dorment, et le simulateur travaille plusieurs fois moins.
    let s = avec.banc.sim.sommeil();
    println!("{s:?}");
    assert!(
        s.endormies * 2 >= s.regions,
        "au moins un mot sur deux doit dormir : {s:?}"
    );
    let (a, b) = (
        avec.banc.sim.statistiques().evaluations,
        sans.banc.sim.statistiques().evaluations,
    );
    println!("{a} évaluations avec sommeil, {b} sans");
    assert!(a * 4 < b, "{a} évaluations avec sommeil, {b} sans");
}

/// La mise à plat signale les registres au simulateur, et rien d'autre ne change.
#[test]
fn la_mise_a_plat_signale_des_regions_disjointes() {
    let programme = assembler(programmes::FIBONACCI).unwrap();
    let m = Machine::nouvelle(&programme, Reglages::default()).unwrap();
    let nl = &m.banc.circuit.netlist;
    assert!(
        nl.regions.len() >= 16,
        "au moins les 16 registres de la mémoire vive"
    );
    let mut fin = 0;
    for r in &nl.regions {
        assert!(r.start >= fin && r.start < r.end && r.end as usize <= nl.portes.len());
        assert!((8..=128).contains(&(r.end - r.start)));
        fin = r.end;
    }
}
