//! Sur de vrais circuits, le mode rapide ne change rien : l'ordinateur miniature et un
//! compteur simulés période par période passent par les mêmes états, fil par fil et
//! porte par porte, que simulés porte par porte. Et le mode rapide s'engage bien,
//! qu'il calcule les périodes événement par événement ou par le code compilé du
//! circuit.

use ce_circuit::ordinateur::{assembler, programmes, Machine, Reglages};
use ce_moteur::alea::Alea;
use ce_moteur::{Options, Simulateur};

const EXACT: Options = Options {
    sommeil: false,
    rapide: false,
    compilation: false,
};

/// Vérifie que les deux simulateurs sont dans le même état : chaque fil, chaque porte
/// (sortie, cible, échéance), le repos et les courts-circuits.
fn memes_etats(rapide: &Simulateur, exact: &Simulateur, contexte: &str) {
    assert_eq!(rapide.temps(), exact.temps(), "temps, {contexte}");
    assert!(
        rapide.codes() == exact.codes(),
        "valeurs des fils, {contexte}"
    );
    for g in 0..rapide.nb_portes() {
        assert_eq!(
            rapide.etat_relatif_porte(g),
            exact.etat_relatif_porte(g),
            "porte {g}, {contexte}"
        );
    }
    assert_eq!(rapide.est_stable(), exact.est_stable(), "repos, {contexte}");
    assert_eq!(rapide.conflits(), exact.conflits(), "{contexte}");
}

#[test]
fn l_ordinateur_passe_par_les_memes_etats_en_mode_rapide() {
    les_memes_etats(false);
}

#[test]
fn l_ordinateur_passe_par_les_memes_etats_en_code_compile() {
    les_memes_etats(true);
}

fn les_memes_etats(compiler: bool) {
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
        let options = Options {
            compilation: compiler,
            ..Options::default()
        };
        let mut rapide = Machine::avec_options(&programme, reglages, options).unwrap();
        if compiler {
            assert!(rapide.banc.sim.compiler(), "le circuit se compile");
        }
        let mut exact = Machine::avec_options(&programme, reglages, EXACT).unwrap();
        let touches = touches.as_bytes();
        let mut alea = Alea::nouveau(7);
        for cycle in 0..400 {
            // Chaque touche reste enfoncée 12 cycles, puis est relâchée 12 cycles.
            let touche = if cycle % 24 < 12 {
                touches.get(cycle / 24).copied().unwrap_or(0)
            } else {
                0
            };
            assert_eq!(rapide.cycle(touche), exact.cycle(touche), "{nom}, écran");
            memes_etats(
                &rapide.banc.sim,
                &exact.banc.sim,
                &format!("{nom}, cycle {cycle}"),
            );
            if alea.chance(1, 16) {
                // On regarde aussi en plein cycle : le mode rapide s'interrompt.
                let pas = alea.entre(1, 2 * reglages.demi_periode - 1);
                for banc in [&mut rapide.banc, &mut exact.banc] {
                    banc.avancer(pas);
                }
                memes_etats(
                    &rapide.banc.sim,
                    &exact.banc.sim,
                    &format!("{nom}, en plein cycle {cycle}"),
                );
                for banc in [&mut rapide.banc, &mut exact.banc] {
                    banc.avancer(2 * reglages.demi_periode - pas);
                }
                memes_etats(
                    &rapide.banc.sim,
                    &exact.banc.sim,
                    &format!("{nom}, après le cycle {cycle}"),
                );
            }
        }
        assert_eq!(rapide.ecran, exact.ecran, "{nom}");
        let stats = rapide.banc.sim.statistiques();
        let (cycles_rapides, cycles_compiles) = (stats.cycles_rapides, stats.cycles_compiles);
        println!(
            "{nom} : {cycles_rapides} cycles sur 400 en mode rapide, {cycles_compiles} compilés"
        );
        assert!(
            cycles_rapides >= 250,
            "{nom} : le mode rapide doit faire l'essentiel du travail ({cycles_rapides} cycles)"
        );
        assert_eq!(cycles_compiles, if compiler { cycles_rapides } else { 0 });
    }
}

/// La remise à zéro, qui change une entrée en plein cycle, puis un programme entier.
#[test]
fn l_ordinateur_execute_un_programme_en_mode_rapide() {
    let programme = assembler(programmes::FIBONACCI).unwrap();
    let mut rapide = Machine::nouvelle(&programme, Reglages::default()).unwrap();
    rapide.banc.sim.compiler();
    let mut exact = Machine::avec_options(&programme, Reglages::default(), EXACT).unwrap();
    for _ in 0..3 {
        let e1 = rapide.executer(2000, |_| 0);
        let e2 = exact.executer(2000, |_| 0);
        assert_eq!(e1, e2);
        memes_etats(&rapide.banc.sim, &exact.banc.sim, "fin du programme");
        rapide.remettre_a_zero();
        exact.remettre_a_zero();
        memes_etats(&rapide.banc.sim, &exact.banc.sim, "remise à zéro");
    }
    assert!(rapide.banc.sim.statistiques().cycles_compiles > 0);
}
