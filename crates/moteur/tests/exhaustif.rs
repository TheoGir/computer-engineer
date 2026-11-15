//! Preuve par exploration exhaustive : pour un délai donné, la porte du
//! simulateur événementiel et la règle de la spécification sont deux automates
//! finis. On explore **tous** les états atteignables de leur produit, avec les
//! quatre valeurs de cible possibles à chaque instant, et on vérifie qu'ils
//! donnent toujours la même sortie.
//!
//! Contrairement à un test aléatoire, c'est une démonstration complète de
//! l'équivalence des deux automates pour chaque délai exploré : toute suite de
//! cibles, de n'importe quelle longueur, passe par des états déjà vérifiés.

use ce_moteur::specification::sortie_suivante;
use ce_moteur::{Netlist, Simulateur, Valeur};
use std::collections::{HashSet, VecDeque};

/// Entrées (validation, donnée) d'un tampon trois états qui donnent la cible voulue.
fn entrees_pour(cible: Valeur) -> (Valeur, Valeur) {
    match cible {
        Valeur::Zero => (Valeur::Un, Valeur::Zero),
        Valeur::Un => (Valeur::Un, Valeur::Un),
        Valeur::Z => (Valeur::Zero, Valeur::Zero),
        Valeur::X => (Valeur::X, Valeur::X),
    }
}

/// Explore le produit des deux automates pour le délai `d`. Rend le nombre
/// d'états atteignables et le nombre de transitions vérifiées.
fn explorer(d: u16) -> (usize, usize) {
    let mut nl = Netlist::nouvelle();
    let (e, a, s) = (nl.net(), nl.net(), nl.net());
    nl.entree(e);
    nl.entree(a);
    nl.tri(e, a, s, d);

    // État de la spécification à l'instant t : sortie(t) et fenêtre des d cibles
    // précédentes (instants t-d à t-1). Avant l'instant 0, tout vaut X.
    type Cle = (Valeur, Vec<Valeur>, [u32; 6]);
    let sim0 = Simulateur::nouveau(&nl).unwrap();
    let fenetre0 = vec![Valeur::X; d as usize];
    let mut vus: HashSet<Cle> = HashSet::new();
    let mut file: VecDeque<(Simulateur, Valeur, Vec<Valeur>)> = VecDeque::new();
    vus.insert((Valeur::X, fenetre0.clone(), sim0.etat_relatif_porte(0)));
    file.push_back((sim0, Valeur::X, fenetre0));
    let mut transitions = 0;
    while let Some((sim, sortie, fenetre)) = file.pop_front() {
        assert_eq!(sim.sortie_porte(0), sortie);
        assert_eq!(sim.valeur(s), sortie);
        for cible in Valeur::TOUTES {
            let mut suite = sim.clone();
            let (ve, va) = entrees_pour(cible);
            suite.fixer_entree(0, ve);
            suite.fixer_entree(1, va);
            suite.avancer(1);
            let mut fenetre_suivante = fenetre[1..].to_vec();
            fenetre_suivante.push(cible);
            let sortie_suivante = sortie_suivante(&fenetre_suivante, sortie);
            assert_eq!(
                suite.sortie_porte(0),
                sortie_suivante,
                "délai {d}, sortie {sortie}, fenêtre {fenetre_suivante:?}"
            );
            transitions += 1;
            let cle = (
                sortie_suivante,
                fenetre_suivante.clone(),
                suite.etat_relatif_porte(0),
            );
            if vus.insert(cle) {
                file.push_back((suite, sortie_suivante, fenetre_suivante));
            }
        }
    }
    (vus.len(), transitions)
}

#[test]
fn la_porte_evenementielle_equivaut_a_la_specification_pour_tout_delai_jusqu_a_8() {
    let mut total = 0;
    for d in 1..=8 {
        let (etats, transitions) = explorer(d);
        println!("délai {d} : {etats} états atteignables, {transitions} transitions vérifiées");
        assert_eq!(transitions, 4 * etats);
        // Au moins un état par couple (sortie, fenêtre) atteignable.
        assert!(etats >= 4usize.pow(d as u32));
        total += etats;
    }
    assert!(total > 10_000);
}
