//! Bibliothèque de composants et format de sauvegarde.
//!
//! Une bibliothèque est un ensemble de composants nommés. Un composant a des
//! broches, des instances d'autres composants (ou de primitives) et des fils.
//! Il se sauvegarde en JSON lisible :
//!
//! ```json
//! {
//!   "format": "computer-engineer/bibliotheque/1",
//!   "composants": {
//!     "NON": {
//!       "description": "Inverseur",
//!       "broches": [
//!         { "nom": "a", "sens": "entree" },
//!         { "nom": "s", "sens": "sortie" }
//!       ],
//!       "instances": [ { "nom": "n", "type": "NAND" } ],
//!       "fils": [ ["a", "n.a", "n.b"], ["n.s", "s"] ]
//!     }
//!   }
//! }
//! ```
//!
//! ## Primitives
//!
//! | Type | Broches | Paramètres |
//! |---|---|---|
//! | `NAND` | entrées `a`, `b` ; sortie `s` | `delai` (1 par défaut) |
//! | `TRI` | validation `e`, donnée `a` ; sortie `s` | `delai` (1 par défaut) |
//! | `ZERO`, `UN` | sortie `s` | aucun |
//! | `HORLOGE` | sortie `s` | `demi_periode` (obligatoire) |
//!
//! ## Fils
//!
//! Un fil est la liste de ses extrémités. Une extrémité s'écrit `broche` pour une
//! broche du composant lui-même, ou `instance.broche` pour une broche d'une
//! instance. Pour un bus, `broche` désigne tous les bits (bit 0 d'abord),
//! `broche[3]` un seul bit et `broche[0:3]` les bits 0, 1, 2, 3 dans cet ordre
//! (`broche[3:0]` les donne dans l'ordre inverse). Toutes les extrémités d'un fil
//! doivent avoir le même nombre de bits : elles sont reliées bit à bit.

use crate::erreur::Erreur;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

/// Identifiant du format de fichier lu et écrit par cette version.
pub const FORMAT: &str = "computer-engineer/bibliotheque/1";

/// Noms des primitives : aucun composant ne peut les porter.
pub const PRIMITIVES: [&str; 5] = ["NAND", "TRI", "ZERO", "UN", "HORLOGE"];

/// Un ensemble de composants nommés.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Bibliotheque {
    /// Identifiant du format, toujours [`FORMAT`].
    pub format: String,
    /// Les composants, par nom.
    #[serde(default)]
    pub composants: BTreeMap<String, Composant>,
}

/// Un composant : un circuit réutilisable, vu de l'extérieur par ses broches.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Composant {
    /// Texte libre.
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub description: String,
    /// Broches, dans l'ordre d'affichage.
    #[serde(default)]
    pub broches: Vec<Broche>,
    /// Instances de primitives ou d'autres composants.
    #[serde(default)]
    pub instances: Vec<Instance>,
    /// Fils : chaque fil est la liste de ses extrémités.
    #[serde(default)]
    pub fils: Vec<Vec<String>>,
    /// Données réservées à l'éditeur graphique (positions, tracé des fils).
    /// Le moteur ne les lit pas et les conserve telles quelles.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub editeur: Option<serde_json::Value>,
}

/// Sens d'une broche.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Sens {
    /// Le composant lit cette broche.
    Entree,
    /// Le composant pilote cette broche.
    Sortie,
}

fn largeur_un() -> u32 {
    1
}

fn est_un(n: &u32) -> bool {
    *n == 1
}

/// Une broche d'un composant, éventuellement un bus de plusieurs bits.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Broche {
    /// Nom de la broche.
    pub nom: String,
    /// Entrée ou sortie.
    pub sens: Sens,
    /// Nombre de bits (1 par défaut).
    #[serde(default = "largeur_un", skip_serializing_if = "est_un")]
    pub largeur: u32,
}

/// Une instance d'un composant ou d'une primitive à l'intérieur d'un composant.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Instance {
    /// Nom de l'instance, unique dans le composant.
    pub nom: String,
    /// Nom du composant ou de la primitive instanciée.
    #[serde(rename = "type")]
    pub genre: String,
    /// Délai de propagation (primitives `NAND` et `TRI` seulement).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub delai: Option<u16>,
    /// Demi-période (primitive `HORLOGE` seulement).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub demi_periode: Option<u64>,
}

impl Default for Bibliotheque {
    fn default() -> Self {
        Self::nouvelle()
    }
}

impl Bibliotheque {
    /// Bibliothèque vide.
    pub fn nouvelle() -> Self {
        Bibliotheque {
            format: FORMAT.to_string(),
            composants: BTreeMap::new(),
        }
    }

    /// Ajoute ou remplace un composant.
    pub fn ajouter(&mut self, nom: &str, composant: Composant) {
        self.composants.insert(nom.to_string(), composant);
    }

    /// Ajoute tous les composants d'une autre bibliothèque (ceux de même nom sont remplacés).
    pub fn fusionner(&mut self, autre: Bibliotheque) {
        self.composants.extend(autre.composants);
    }

    /// Lit une bibliothèque au format JSON.
    pub fn depuis_json(texte: &str) -> Result<Self, Erreur> {
        let bib: Bibliotheque =
            serde_json::from_str(texte).map_err(|e| Erreur::Json(e.to_string()))?;
        if bib.format != FORMAT {
            return Err(Erreur::FormatInconnu(bib.format));
        }
        Ok(bib)
    }

    /// Écrit la bibliothèque en JSON indenté.
    pub fn vers_json(&self) -> String {
        serde_json::to_string_pretty(self).expect("sérialisation sans erreur possible")
    }

    /// Vérifie tous les composants : noms, fils, largeurs, dépendances.
    pub fn valider(&self) -> Result<(), Erreur> {
        for nom in self.composants.keys() {
            crate::elaboration::compiler(self, nom)?;
        }
        Ok(())
    }
}

impl Composant {
    /// Composant vide portant une description.
    pub fn nouveau(description: &str) -> Self {
        Composant {
            description: description.to_string(),
            ..Default::default()
        }
    }

    /// Ajoute une broche d'entrée de `largeur` bits.
    pub fn entree(mut self, nom: &str, largeur: u32) -> Self {
        self.broches.push(Broche {
            nom: nom.to_string(),
            sens: Sens::Entree,
            largeur,
        });
        self
    }

    /// Ajoute une broche de sortie de `largeur` bits.
    pub fn sortie(mut self, nom: &str, largeur: u32) -> Self {
        self.broches.push(Broche {
            nom: nom.to_string(),
            sens: Sens::Sortie,
            largeur,
        });
        self
    }

    /// Ajoute une instance de type `genre`.
    pub fn inst(mut self, nom: &str, genre: &str) -> Self {
        self.instances.push(Instance {
            nom: nom.to_string(),
            genre: genre.to_string(),
            delai: None,
            demi_periode: None,
        });
        self
    }

    /// Ajoute une porte `NAND` ou `TRI` de délai donné.
    pub fn inst_delai(mut self, nom: &str, genre: &str, delai: u16) -> Self {
        self.instances.push(Instance {
            nom: nom.to_string(),
            genre: genre.to_string(),
            delai: Some(delai),
            demi_periode: None,
        });
        self
    }

    /// Ajoute une horloge.
    pub fn horloge(mut self, nom: &str, demi_periode: u64) -> Self {
        self.instances.push(Instance {
            nom: nom.to_string(),
            genre: "HORLOGE".to_string(),
            delai: None,
            demi_periode: Some(demi_periode),
        });
        self
    }

    /// Ajoute un fil reliant les extrémités données.
    pub fn fil<S: AsRef<str>>(mut self, points: &[S]) -> Self {
        self.fils
            .push(points.iter().map(|p| p.as_ref().to_string()).collect());
        self
    }
}
