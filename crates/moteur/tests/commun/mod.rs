//! Outils partagés par les tests d'intégration du moteur.
#![allow(dead_code)]

use ce_moteur::specification::{simuler, Trace};
use ce_moteur::{Netlist, Simulateur, Valeur};

/// Fait tourner le simulateur optimisé avec le stimulus donné et rend, pour chaque
/// instant, la valeur de tous les nets et la sortie de toutes les portes.
pub fn derouler(nl: &Netlist, stimulus: &[Vec<Valeur>]) -> (Vec<Vec<Valeur>>, Vec<Vec<Valeur>>) {
    let mut sim = Simulateur::nouveau(nl).expect("netlist valide");
    let mut nets = Vec::new();
    let mut sorties = Vec::new();
    for (t, ligne) in stimulus.iter().enumerate() {
        assert_eq!(sim.temps(), t as u64);
        for (i, &v) in ligne.iter().enumerate() {
            sim.fixer_entree(i, v);
        }
        nets.push((0..nl.nb_nets).map(|n| sim.valeur(n)).collect());
        sorties.push((0..nl.portes.len()).map(|g| sim.sortie_porte(g)).collect());
        sim.avancer(1);
    }
    (nets, sorties)
}

/// Fait tourner la spécification avec le même stimulus.
pub fn derouler_spec(nl: &Netlist, stimulus: &[Vec<Valeur>]) -> Trace {
    simuler(nl, stimulus.len() - 1, &|t, i| stimulus[t][i])
}

/// Écrit une ligne de valeurs sous forme de chaîne `01xz`.
pub fn chaine(v: &[Valeur]) -> String {
    v.iter().map(|x| x.symbole()).collect()
}
