//! Les quêtes de dépannage : un circuit tout fait, mais une de ses portes est mal branchée.
//! Il faut trouver laquelle, en essayant les entrées et en suivant les fils, puis la réparer.
//!
//! Le circuit de départ est le corrigé de la quête mis à plat en portes NAND, disposé en
//! colonnes, avec une panne choisie une fois pour toutes : la première entrée de porte qui,
//! rebranchée ailleurs sans créer de boucle, rend la table de vérité fausse. La quête se
//! vérifie ensuite comme toute quête combinatoire, contre le même corrigé.

use std::collections::BTreeMap;

use ce_circuit::{Bibliotheque, Circuit, Sens};
use ce_moteur::GenrePorte;

use crate::brique::Projet;
use crate::document::{Document, Element, Fil, Id, Point, Source};
use crate::quete::Fiche;

/// Les quêtes de dépannage : leur clé, et à partir de quelle porte (comptée depuis la fin du
/// circuit) on cherche la panne, pour qu'elles ne tombent pas toutes au même endroit.
pub const PANNES: &[(&str, usize)] = &[
    ("panne-ou-x", 0),
    ("panne-demi-additionneur", 2),
    ("panne-multiplexeur", 1),
    ("panne-additionneur-complet", 3),
];

/// Vrai pour une quête de dépannage.
pub fn est_un_depannage(cle: &str) -> bool {
    PANNES.iter().any(|(c, _)| *c == cle)
}

/// Le composant `nom` de `bib` mis à plat en portes NAND, dans un plan disposé en colonnes :
/// un interrupteur par broche d'entrée, une lampe par broche de sortie (d'un bit chacune).
pub fn plan_a_plat(bib: &Bibliotheque, nom: &str) -> Result<Document, String> {
    let circuit = Circuit::elaborer(bib, nom).map_err(|e| e.to_string())?;
    let net = &circuit.netlist;
    if !net.constantes.is_empty() || !net.horloges.is_empty() {
        return Err(tr!("« {nom} » a des constantes ou une horloge", nom = nom));
    }
    if net.portes.iter().any(|p| p.genre != GenrePorte::Nand) {
        return Err(tr!("« {nom} » n'est pas fait que de NAND", nom = nom));
    }
    if circuit.broches.iter().any(|b| b.nets.len() != 1) {
        return Err(tr!("« {nom} » a des broches de plusieurs bits", nom = nom));
    }
    let mut plan = Document::nouveau();
    let mut source_de: BTreeMap<u32, Source> = BTreeMap::new();
    let (mut entrees, mut sorties) = (Vec::new(), Vec::new());
    for b in circuit.broches.iter().filter(|b| b.sens == Sens::Entree) {
        let id = plan.poser(Element::entree(&b.nom, 1), Point::default());
        source_de.insert(b.nets[0], Source::de(id));
        entrees.push(id);
    }
    let portes: Vec<Id> = (0..net.portes.len())
        .map(|k| {
            let id = plan.poser(Element::nand(&format!("g{}", k + 1)), Point::default());
            source_de.insert(net.portes[k].s, Source::de(id));
            id
        })
        .collect();
    for (k, p) in net.portes.iter().enumerate() {
        for (i, e) in [p.e0, p.e1].into_iter().enumerate() {
            let source = source_de.get(&e).ok_or_else(|| {
                tr!("une entrée de porte de « {nom} » n'est pilotée par rien", nom = nom)
            })?;
            plan.relier(
                *source,
                Fil {
                    puits: portes[k],
                    entree: i,
                },
            );
        }
    }
    for b in circuit.broches.iter().filter(|b| b.sens == Sens::Sortie) {
        let id = plan.poser(Element::sortie(&b.nom, &b.nom, 1), Point::default());
        let source = source_de
            .get(&b.nets[0])
            .ok_or_else(|| tr!("la sortie {nom2} de « {nom} » n'est pilotée par rien", nom = nom, nom2 = b.nom))?;
        plan.relier(
            *source,
            Fil {
                puits: id,
                entree: 0,
            },
        );
        sorties.push(id);
    }
    crate::importer::disposer(&mut plan, &entrees, &sorties);
    Ok(plan)
}

/// La table de vérité d'un plan sans brique, ou `None` s'il a une boucle.
fn table(plan: &Document) -> Option<Vec<Vec<ce_moteur::Valeur>>> {
    let projet = Projet::nouveau("", plan.clone());
    crate::logique::analyser(&projet, plan)
        .ok()
        .map(|a| a.lignes)
}

/// Le plan de départ d'une quête de dépannage : son corrigé mis à plat, avec une panne.
/// `None` si ce n'est pas une quête de dépannage.
pub fn plan_en_panne(fiche: &Fiche) -> Option<Result<Document, String>> {
    let &(_, depart) = PANNES.iter().find(|(c, _)| *c == fiche.cle)?;
    Some(plan_en_panne_de(
        crate::quete::corriges(),
        fiche.corrige,
        depart,
    ))
}

fn plan_en_panne_de(bib: &Bibliotheque, corrige: &str, depart: usize) -> Result<Document, String> {
    let juste = plan_a_plat(bib, corrige)?;
    let attendue = table(&juste).ok_or(tr!("le corrigé a une boucle"))?;
    let portes: Vec<Id> = juste.portes().map(|e| e.id).collect();
    let sources: Vec<Source> = juste
        .elements
        .iter()
        .filter(|e| e.est_source())
        .map(|e| Source::de(e.id))
        .collect();
    let n = portes.len();
    for rang in 0..n {
        // De la dernière porte vers la première, en commençant à `depart`.
        let porte = portes[n - 1 - (depart + rang) % n];
        for entree in 0..2 {
            let fil = Fil {
                puits: porte,
                entree,
            };
            let actuelle = juste.source_de(fil);
            for &autre in &sources {
                if Some(autre) == actuelle || autre.id == porte {
                    continue;
                }
                let mut essai = juste.clone();
                essai.relier(autre, fil);
                if table(&essai).is_some_and(|t| t != attendue) {
                    return Ok(essai);
                }
            }
        }
    }
    Err(tr!("aucune panne trouvée pour « {corrige} »", corrige = corrige))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::quete::{Quete, PARCOURS};

    /// Chaque quête de dépannage : le corrigé mis à plat est juste, le plan de départ est
    /// faux, et il ne diffère du juste que par une seule entrée de porte.
    #[test]
    fn chaque_panne_tient_a_un_seul_fil() {
        for (cle, depart) in PANNES {
            let fiche = PARCOURS.iter().find(|f| f.cle == *cle).expect("fiche");
            let bib = crate::quete::corriges();
            let juste = plan_a_plat(bib, fiche.corrige).unwrap();
            let mut quete = Quete::de_fiche(fiche);
            assert!(
                quete.verifier(&Projet::nouveau("", juste.clone())).reussi,
                "{cle} : le corrigé à plat réussit"
            );
            let panne = plan_en_panne_de(bib, fiche.corrige, *depart).unwrap();
            assert!(
                !quete.verifier(&Projet::nouveau("", panne.clone())).reussi,
                "{cle} : le circuit en panne échoue"
            );
            let differences: Vec<_> = juste
                .elements
                .iter()
                .zip(&panne.elements)
                .filter(|(a, b)| a.entrees != b.entrees)
                .collect();
            assert_eq!(differences.len(), 1, "{cle}");
            assert_eq!(juste.elements.len(), panne.elements.len());
        }
    }
}
