//! L'avancement dans le parcours des quêtes : les quêtes réussies et les briques gagnées.
//!
//! Chaque quête réussie laisse sa solution, celle que l'utilisateur a construite, dans sa
//! collection de briques. Une nouvelle quête s'ouvre avec toute cette collection dans la
//! palette : l'additionneur se construit avec ses propres portes, la mémoire avec ses propres
//! bascules, et l'on monte ainsi jusqu'à l'ordinateur.

use std::collections::BTreeSet;

use serde::{Deserialize, Serialize};

use crate::brique::{self, Modele, Projet};
use crate::exemples;
use crate::format::{self, Lecture};
use crate::quete::{Epreuve, Fiche, Quete, PARCOURS};

/// Identifiant du format du fichier d'avancement.
pub const FORMAT: &str = "computer-engineer/parcours/4";

/// Ce que l'utilisateur a accompli dans le parcours.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Parcours {
    /// Format du fichier.
    pub format: String,
    /// Clés des quêtes réussies.
    pub faites: BTreeSet<String>,
    /// Les briques gagnées, et celles dont elles ont besoin.
    pub briques: Vec<Modele>,
    /// Augmente à chaque changement : sert à savoir quand enregistrer.
    #[serde(skip)]
    pub version: u64,
}

impl Default for Parcours {
    fn default() -> Self {
        Parcours {
            format: FORMAT.to_owned(),
            faites: BTreeSet::new(),
            briques: Vec::new(),
            version: 0,
        }
    }
}

impl Parcours {
    /// Vrai si la quête est réussie.
    pub fn faite(&self, cle: &str) -> bool {
        self.faites.contains(cle)
    }

    /// La quête qui suit celle-ci dans le parcours.
    pub fn suivante(cle: &str) -> Option<&'static Fiche> {
        let i = PARCOURS.iter().position(|f| f.cle == cle)?;
        PARCOURS.get(i + 1)
    }

    /// La première quête pas encore réussie.
    pub fn prochaine(&self) -> Option<&'static Fiche> {
        PARCOURS.iter().find(|f| !self.faite(f.cle))
    }

    /// Note la quête réussie et range sa solution, le plan principal du projet, parmi les
    /// briques gagnées, avec toutes les briques qu'elle utilise.
    pub fn gagner(&mut self, quete: &Quete, projet: &Projet) {
        self.faites.insert(quete.cle.to_owned());
        if !quete.epreuve.ordinateur() {
            let mut ranger = |m: Modele| match self.briques.iter_mut().find(|b| b.nom == m.nom) {
                Some(b) => *b = m,
                None => self.briques.push(m),
            };
            for m in &projet.modeles {
                ranger(m.clone());
            }
            // Une quête rejouée avec la brique qu'elle a déjà fait gagner : la brique reste
            // celle d'avant, sinon elle se contiendrait elle-même.
            let reutilise = projet
                .principal
                .elements
                .iter()
                .filter(|e| e.genre == crate::document::Genre::Brique)
                .any(|e| projet.utilise(&e.modele, quete.brique));
            if reutilise {
                self.version += 1;
                return;
            }
            let mut solution = brique::modele_du_plan(&projet.principal, quete.brique);
            solution.description = quete.titre.to_owned();
            ranger(solution);
        }
        self.version += 1;
    }

    /// Le projet où se joue une quête : son plan de départ, et les briques gagnées dans la
    /// palette. L'ordinateur complet part de ses grands composants déjà construits.
    pub fn projet(&self, fiche: &'static Fiche, nom: &str) -> Result<Projet, String> {
        let quete = Quete::de_fiche(fiche);
        let mut projet = match fiche.epreuve {
            Epreuve::Ordinateur {
                programme, complet, ..
            } => {
                let mut p = exemples::atelier_de_l_ordinateur(nom, programme, !complet)?;
                // Les briques gagnées, sauf celles que l'atelier fournit déjà sous ce nom.
                for m in &self.briques {
                    if p.modele(&m.nom).is_none() {
                        p.ajouter(m.clone());
                    }
                }
                p
            }
            Epreuve::Systeme { atelier, .. } => {
                let mut p = exemples::atelier_du_grand_projet(nom, atelier)?;
                for m in &self.briques {
                    if p.modele(&m.nom).is_none() {
                        p.ajouter(m.clone());
                    }
                }
                p
            }
            _ => {
                let depart = match crate::depannage::plan_en_panne(fiche) {
                    Some(plan) => plan?,
                    None => quete.plan_de_depart(),
                };
                let mut p = Projet::nouveau(nom, depart);
                for m in &self.briques {
                    p.ajouter(m.clone());
                }
                p
            }
        };
        projet.quete = Some(fiche.cle.to_owned());
        Ok(projet)
    }

    /// Le parcours écrit en JSON.
    pub fn en_json(&self) -> String {
        serde_json::to_string_pretty(self).expect("le parcours se sérialise toujours")
    }

    /// Relit un parcours.
    pub fn de_json(texte: &str) -> Result<Parcours, String> {
        Parcours::de_json_lu(texte).map(|(p, _)| p)
    }

    /// Comme [`Parcours::de_json`], et dit si le fichier a dû être migré.
    pub fn de_json_lu(texte: &str) -> Result<(Parcours, Lecture), String> {
        let (valeur, lecture) = format::PARCOURS.lire(texte)?;
        let mut p: Parcours =
            serde_json::from_value(valeur).map_err(|e| format!("fichier illisible : {e}"))?;
        for m in &mut p.briques {
            m.plan.reparer();
        }
        Ok((p, lecture))
    }
}
