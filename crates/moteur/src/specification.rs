//! Sémantique de référence, écrite sans aucune optimisation.
//!
//! Ce module **définit** ce que le simulateur doit calculer. Il garde toute
//! l'histoire en mémoire et recalcule tout à chaque instant : il est lent, mais
//! chaque ligne se lit directement contre la définition mathématique de
//! `docs/semantique.md`. Le simulateur optimisé ([`crate::simulateur`]) est testé
//! en le comparant à ce module, instant par instant et net par net.
//!
//! ## Définition
//!
//! Le temps est entier : `t = 0, 1, 2, …`
//!
//! 1. **Cible.** À chaque instant, la *cible* d'une porte est sa fonction logique
//!    appliquée aux valeurs présentes de ses nets d'entrée.
//! 2. **Délai inertiel sûr.** Soit `d ≥ 1` le délai de la porte et `F(t)` la
//!    *fenêtre* des `d` dernières cibles, aux instants `t-d, …, t-1` (les instants
//!    négatifs valent `X` : on ne sait rien du passé avant la mise sous tension).
//!    La sortie vaut `X` à `t = 0`, puis :
//!    - si toutes les cibles de `F(t)` valent une même valeur `v`, la sortie vaut `v` ;
//!    - sinon, si la sortie précédente figure dans `F(t)`, la sortie est conservée ;
//!    - sinon la sortie vaut `X`.
//! 3. **Résolution.** La valeur d'un net est la résolution de tous ses pilotes.
//!
//! Comme `d ≥ 1`, la sortie à `t` ne dépend que d'instants strictement antérieurs :
//! il n'y a aucune dépendance circulaire, même dans un circuit rebouclé, et le
//! résultat ne dépend d'aucun ordre d'évaluation.

use crate::netlist::{GenrePorte, Netlist};
use crate::valeur::{nand, resoudre, tri, Valeur};

/// Histoire complète d'une simulation de référence.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Trace {
    /// `nets[t][n]` : valeur du net `n` à l'instant `t`.
    pub nets: Vec<Vec<Valeur>>,
    /// `sorties[t][g]` : valeur pilotée par la porte `g` à l'instant `t`.
    pub sorties: Vec<Vec<Valeur>>,
    /// `cibles[t][g]` : cible de la porte `g` à l'instant `t`.
    pub cibles: Vec<Vec<Valeur>>,
}

/// Règle du délai inertiel sûr : nouvelle sortie à partir de la fenêtre des `d`
/// dernières cibles et de la sortie précédente.
pub fn sortie_suivante(fenetre: &[Valeur], precedente: Valeur) -> Valeur {
    let premiere = fenetre[0];
    if fenetre.iter().all(|&v| v == premiere) {
        premiere
    } else if fenetre.contains(&precedente) {
        precedente
    } else {
        Valeur::X
    }
}

/// Simule `netlist` de l'instant `0` à l'instant `duree` inclus.
///
/// `stimulus(t, i)` donne la valeur pilotée par l'entrée externe `i` à l'instant `t`.
pub fn simuler(
    netlist: &Netlist,
    duree: usize,
    stimulus: &dyn Fn(usize, usize) -> Valeur,
) -> Trace {
    netlist.valider().expect("netlist invalide");
    let nb_nets = netlist.nb_nets as usize;
    let nb_portes = netlist.portes.len();
    let mut trace = Trace {
        nets: Vec::with_capacity(duree + 1),
        sorties: Vec::with_capacity(duree + 1),
        cibles: Vec::with_capacity(duree + 1),
    };
    for t in 0..=duree {
        // 2. Sorties des portes : délai inertiel sûr.
        let mut sorties = vec![Valeur::X; nb_portes];
        if t > 0 {
            for (g, porte) in netlist.portes.iter().enumerate() {
                let d = porte.delai as usize;
                let fenetre: Vec<Valeur> = (0..d)
                    .map(|k| {
                        // Instant t - d + k, de t-d à t-1.
                        if t + k >= d {
                            trace.cibles[t + k - d][g]
                        } else {
                            Valeur::X
                        }
                    })
                    .collect();
                sorties[g] = sortie_suivante(&fenetre, trace.sorties[t - 1][g]);
            }
        }
        // 3. Résolution des nets.
        let mut nets = vec![Valeur::Z; nb_nets];
        for (g, porte) in netlist.portes.iter().enumerate() {
            let n = porte.s as usize;
            nets[n] = resoudre(nets[n], sorties[g]);
        }
        for &(net, v) in &netlist.constantes {
            nets[net as usize] = resoudre(nets[net as usize], v);
        }
        for (i, &net) in netlist.entrees.iter().enumerate() {
            nets[net as usize] = resoudre(nets[net as usize], stimulus(t, i));
        }
        for h in &netlist.horloges {
            nets[h.net as usize] = resoudre(nets[h.net as usize], h.niveau(t as u64));
        }
        // 1. Cibles des portes.
        let cibles: Vec<Valeur> = netlist
            .portes
            .iter()
            .map(|p| {
                let (a, b) = (nets[p.e0 as usize], nets[p.e1 as usize]);
                match p.genre {
                    GenrePorte::Nand => nand(a, b),
                    GenrePorte::Tri => tri(a, b),
                }
            })
            .collect();
        trace.nets.push(nets);
        trace.sorties.push(sorties);
        trace.cibles.push(cibles);
    }
    trace
}
