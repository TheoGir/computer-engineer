//! La construction minimale : combien de portes NAND faut-il, au moins, pour un composant ?
//!
//! Trois niveaux de certitude, toujours dits tels quels à l'utilisateur :
//!
//! - **minimum prouvé** : une construction de cette taille existe, et aucune plus petite. La
//!   preuve d'absence vient d'un solveur SAT (`outils/minimum_nand.py`) : pour chaque taille
//!   inférieure, la formule « il existe un circuit de k NAND qui calcule ces sorties » est
//!   insatisfaisable. Elle est recoupée par l'énumération de tous les circuits jusqu'à 7 portes
//!   (`outils/minimum_nand_enumeration.py`) et, jusqu'à 6 portes, par le test Rust
//!   `crates/circuit/tests/minimums.rs`.
//! - **encadré** : la meilleure construction connue, et une borne basse prouvée plus petite.
//! - **référence** : seule la construction du corrigé est connue ; on ne dit pas qu'elle est
//!   la plus petite.
//!
//! Chaque construction plus petite que le corrigé est rangée ici sous forme de texte
//! (`g0=a0,a1;g1=g0,g0;s=g1`) et revérifiée par les tests sur toutes les combinaisons
//! d'entrées. Les entrées `a0`, `a1`… sont les bits d'entrée du composant dans l'ordre de
//! ses broches (première broche d'abord, bit 0 d'abord), comme pour
//! [`crate::analyse::TableDeVerite`] ; `s=` donne le signal de chaque bit de sortie.

use std::collections::BTreeMap;
use std::sync::OnceLock;

use ce_moteur::GenrePorte;

use crate::elaboration::Circuit;
use crate::modele::Bibliotheque;

/// Ce qu'on sait de la plus petite construction d'un composant.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Statut {
    /// Aucune construction plus petite n'existe : c'est démontré.
    Prouve,
    /// La meilleure construction connue ; il en faut au moins `borne_basse` (démontré).
    Encadre {
        /// Nombre de NAND dont on a prouvé qu'on ne peut pas descendre en dessous.
        borne_basse: u64,
    },
    /// La construction du corrigé, sans preuve qu'on ne peut pas faire mieux.
    Reference,
}

/// La plus petite construction connue d'un composant, en portes NAND.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Minimum {
    /// Nombre de NAND de la plus petite construction connue.
    pub nand: u64,
    /// Ce qu'on en sait.
    pub statut: Statut,
}

/// Une ligne de `minimums.txt` : ce que la recherche a établi pour un corrigé.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Etabli {
    /// Le composant corrigé.
    pub composant: String,
    /// Nombre de NAND de la meilleure construction connue.
    pub nand: u64,
    /// Borne basse démontrée (égale à `nand` : minimum prouvé).
    pub borne_basse: u64,
    /// La construction, si elle est plus petite que le corrigé.
    pub circuit: Option<String>,
}

/// Les résultats de la recherche, composant par composant.
pub fn etablis() -> &'static BTreeMap<String, Etabli> {
    static TABLE: OnceLock<BTreeMap<String, Etabli>> = OnceLock::new();
    TABLE.get_or_init(|| {
        let mut t = BTreeMap::new();
        for ligne in include_str!("minimums.txt").lines() {
            let ligne = ligne.trim();
            if ligne.is_empty() || ligne.starts_with('#') {
                continue;
            }
            let champs: Vec<&str> = ligne.split_whitespace().collect();
            let e = Etabli {
                composant: champs[0].to_owned(),
                nand: champs[1].parse().expect("minimums.txt : nombre de NAND"),
                borne_basse: champs[2].parse().expect("minimums.txt : borne basse"),
                circuit: (champs[3] != "-").then(|| champs[3].to_owned()),
            };
            t.insert(e.composant.clone(), e);
        }
        t
    })
}

/// Nombre de portes NAND d'un circuit mis à plat.
pub fn nb_nand(circuit: &Circuit) -> u64 {
    circuit
        .netlist
        .portes
        .iter()
        .filter(|p| p.genre == GenrePorte::Nand)
        .count() as u64
}

/// La plus petite construction connue du composant `nom` de `bib`, ou `None` s'il ne se met
/// pas à plat. Sans résultat établi, c'est le corrigé lui-même, au statut de référence.
pub fn minimum(bib: &Bibliotheque, nom: &str) -> Option<Minimum> {
    if let Some(e) = etablis().get(nom) {
        return Some(Minimum {
            nand: e.nand,
            statut: if e.borne_basse >= e.nand {
                Statut::Prouve
            } else {
                Statut::Encadre {
                    borne_basse: e.borne_basse,
                }
            },
        });
    }
    let circuit = Circuit::elaborer(bib, nom).ok()?;
    Some(Minimum {
        nand: nb_nand(&circuit),
        statut: Statut::Reference,
    })
}

/// Un circuit de NAND écrit en texte (`g0=a0,a1;g1=g0,g0;s=g1`) : les deux signaux lus par
/// chaque porte, puis le signal de chaque bit de sortie. Un signal est `aI` (bit d'entrée I)
/// ou `gJ` (porte J, forcément plus ancienne).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Ecrit {
    /// Nombre de bits d'entrée lus.
    pub entrees: usize,
    /// Les portes, chacune avec ses deux signaux.
    pub portes: Vec<(Signal, Signal)>,
    /// Le signal de chaque bit de sortie.
    pub sorties: Vec<Signal>,
}

/// Un signal d'un [`Ecrit`].
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Signal {
    /// Un bit d'entrée.
    Entree(usize),
    /// La sortie d'une porte.
    Porte(usize),
}

impl Ecrit {
    /// Lit un circuit écrit en texte. Rend `None` si le texte est mal formé ou si une porte
    /// lit une porte qui n'est pas plus ancienne.
    pub fn lire(texte: &str) -> Option<Ecrit> {
        let mut portes = Vec::new();
        let mut sorties = Vec::new();
        let mut entrees = 0;
        let mut signal = |s: &str, portes: usize| -> Option<Signal> {
            let (genre, n) = s.split_at(1);
            let n: usize = n.parse().ok()?;
            match genre {
                "a" => {
                    entrees = entrees.max(n + 1);
                    Some(Signal::Entree(n))
                }
                "g" if n < portes => Some(Signal::Porte(n)),
                _ => None,
            }
        };
        for morceau in texte.split(';') {
            let (nom, valeur) = morceau.split_once('=')?;
            if nom == "s" {
                for s in valeur.split(',') {
                    sorties.push(signal(s, portes.len())?);
                }
            } else {
                if nom != format!("g{}", portes.len()) {
                    return None;
                }
                let (a, b) = valeur.split_once(',')?;
                let a = signal(a, portes.len())?;
                let b = signal(b, portes.len())?;
                portes.push((a, b));
            }
        }
        Some(Ecrit {
            entrees,
            portes,
            sorties,
        })
    }

    /// Les bits de sortie quand les bits d'entrée valent ceux de `ligne` (bit 0 = `a0`).
    pub fn calculer(&self, ligne: u64) -> Vec<bool> {
        let mut g = Vec::with_capacity(self.portes.len());
        let lire = |s: Signal, g: &[bool]| match s {
            Signal::Entree(i) => (ligne >> i) & 1 == 1,
            Signal::Porte(j) => g[j],
        };
        for &(a, b) in &self.portes {
            let v = !(lire(a, &g) && lire(b, &g));
            g.push(v);
        }
        self.sorties.iter().map(|&s| lire(s, &g)).collect()
    }
}

/// Le nombre minimal de NAND de chacune des 256 fonctions de 3 entrées (bit `r` de la
/// fonction = sa valeur sur la ligne `r`), avec une construction de cette taille.
pub fn minimums_a_trois_entrees() -> &'static [(u64, Ecrit); 256] {
    static TABLE: OnceLock<[(u64, Ecrit); 256]> = OnceLock::new();
    TABLE.get_or_init(|| {
        let mut t: Vec<(u64, Ecrit)> = Vec::with_capacity(256);
        for ligne in include_str!("minimums3.txt").lines() {
            let ligne = ligne.trim();
            if ligne.is_empty() || ligne.starts_with('#') {
                continue;
            }
            let champs: Vec<&str> = ligne.split_whitespace().collect();
            let f: usize = champs[0].parse().expect("minimums3.txt : fonction");
            assert_eq!(f, t.len(), "minimums3.txt : fonctions dans l'ordre");
            let k: u64 = champs[1].parse().expect("minimums3.txt : taille");
            let e = Ecrit::lire(champs[2]).expect("minimums3.txt : circuit");
            t.push((k, e));
        }
        t.try_into()
            .unwrap_or_else(|_| panic!("minimums3.txt : 256 fonctions attendues"))
    })
}

/// Nombre minimal (prouvé) de NAND pour calculer la fonction de `n` ≤ 3 entrées dont la table
/// est `table` (bit `r` = valeur sur la ligne `r`, `2^n` lignes). Une fonction de moins de 3
/// entrées est vue comme une fonction de 3 entrées qui ignore les dernières : son minimum est
/// le même, car dans un circuit qui la calcule on peut brancher les entrées ignorées sur `a0`.
pub fn minimum_d_une_fonction(n: u32, table: u64) -> Option<u64> {
    if n > 3 {
        return None;
    }
    let lignes = 1u64 << n;
    let mut f = 0usize;
    for r in 0..8u64 {
        if (table >> (r % lignes)) & 1 == 1 {
            f |= 1 << r;
        }
    }
    Some(minimums_a_trois_entrees()[f].0)
}

/// La construction rangée de `nom`, écrite comme un composant de mêmes broches que son corrigé
/// (`corrige`), ou `None` si aucune construction plus petite que le corrigé n'est rangée.
pub fn composant_minimal(
    nom: &str,
    corrige: &crate::modele::Composant,
) -> Option<crate::modele::Composant> {
    use crate::modele::{Composant, Sens};
    let ecrit = Ecrit::lire(etablis().get(nom)?.circuit.as_deref()?)?;
    // Le nom de chaque bit d'entrée puis de sortie, dans l'ordre des broches.
    let bits = |sens: Sens| -> Vec<String> {
        corrige
            .broches
            .iter()
            .filter(|b| b.sens == sens)
            .flat_map(|b| (0..b.largeur).map(move |i| format!("{}[{i}]", b.nom)))
            .collect()
    };
    let (entrees, sorties) = (bits(Sens::Entree), bits(Sens::Sortie));
    if ecrit.entrees > entrees.len() || ecrit.sorties.len() != sorties.len() {
        return None;
    }
    let mut c = Composant::nouveau(&format!(
        "{} (plus petite construction connue : {} NAND)",
        corrige.description,
        ecrit.portes.len()
    ));
    for b in &corrige.broches {
        c = match b.sens {
            Sens::Entree => c.entree(&b.nom, b.largeur),
            Sens::Sortie => c.sortie(&b.nom, b.largeur),
        };
    }
    let signal = |s: Signal| match s {
        Signal::Entree(i) => entrees[i].clone(),
        Signal::Porte(j) => format!("g{j}.s"),
    };
    for (j, &(a, b)) in ecrit.portes.iter().enumerate() {
        c = c
            .inst(&format!("g{j}"), "NAND")
            .fil(&[signal(a), format!("g{j}.a")])
            .fil(&[signal(b), format!("g{j}.b")]);
    }
    for (k, &s) in ecrit.sorties.iter().enumerate() {
        c = c.fil(&[signal(s), sorties[k].clone()]);
    }
    Some(c)
}

/// La bibliothèque des solutions : les corrigés, chacun remplacé par sa plus petite
/// construction connue quand elle est plus petite. Les composants qui en utilisent un autre
/// (une mémoire et son décodeur) en profitent aussi. Les quêtes restent vérifiées contre les
/// corrigés ; le test `chaque_solution_reussit_sa_quete` de l'application vérifie que chaque
/// solution de cette bibliothèque réussit la sienne.
pub fn bibliotheque_des_solutions() -> &'static Bibliotheque {
    static BIB: OnceLock<Bibliotheque> = OnceLock::new();
    BIB.get_or_init(|| {
        let mut bib = crate::classiques::bibliotheque();
        let remplacements: Vec<(String, crate::modele::Composant)> = etablis()
            .keys()
            .filter_map(|nom| {
                let corrige = bib.composants.get(nom)?;
                Some((nom.clone(), composant_minimal(nom, corrige)?))
            })
            .collect();
        for (nom, c) in remplacements {
            bib.ajouter(&nom, c);
        }
        if bib.composants.contains_key("COMPTEUR4") {
            bib.ajouter("COMPTEUR4", compteur_sans_constante());
        }
        bib
    })
}

/// Le compteur 4 bits sans la constante 1 du corrigé, que la palette n'a pas : ajouter 1 au
/// bit 0, c'est l'inverser, et sa retenue est le bit lui-même ; le dernier bit n'a pas besoin
/// de retenue, un OU exclusif suffit. Remise à zéro synchrone par un ET avec `raz` inversé.
fn compteur_sans_constante() -> crate::modele::Composant {
    let mut c = crate::modele::Composant::nouveau(
        "Compteur 4 bits : à chaque front montant de clk, q devient 0 si raz = 1, q + 1 sinon",
    )
    .entree("raz", 1)
    .entree("clk", 1)
    .sortie("q", 4)
    .inst("nraz", "NON")
    .inst("inc0", "NON")
    .inst("inc1", "DEMI_ADD")
    .inst("inc2", "DEMI_ADD")
    .inst("inc3", "OU_X")
    .fil(&["raz", "nraz.a"])
    .fil(&["inc0.s", "et0.a"])
    .fil(&["b0.q", "inc0.a", "inc1.b", "q[0]"])
    .fil(&["b1.q", "inc1.a", "q[1]"])
    .fil(&["inc1.s", "et1.a"])
    .fil(&["inc1.r", "inc2.b"])
    .fil(&["b2.q", "inc2.a", "q[2]"])
    .fil(&["inc2.s", "et2.a"])
    .fil(&["inc2.r", "inc3.b"])
    .fil(&["b3.q", "inc3.a", "q[3]"])
    .fil(&["inc3.s", "et3.a"])
    .fil(&["nraz.s", "et0.b", "et1.b", "et2.b", "et3.b"])
    .fil(&["clk", "b0.clk", "b1.clk", "b2.clk", "b3.clk"]);
    for i in 0..4 {
        c = c
            .inst(&format!("et{i}"), "ET")
            .inst(&format!("b{i}"), "BASCULE_D")
            .fil(&[format!("et{i}.s"), format!("b{i}.d")]);
    }
    c
}
