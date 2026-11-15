//! Du plan dessiné au circuit que le moteur sait simuler.
//!
//! Le plan et ses briques sont traduits en une bibliothèque `ce-circuit`, puis mis à plat par
//! `ce-circuit` : c'est la même mise à plat, déjà prouvée, que celle des fichiers de
//! composants. Ce module ne simule rien et ne connaît du moteur que les numéros de nets.
//!
//! Les noms donnés aux composants sont fabriqués (`m3`, `g12`, `e4`) : l'utilisateur peut donc
//! nommer ses briques et ses broches comme il veut.

use std::collections::BTreeMap;

use ce_circuit::{Bibliotheque, Circuit, Composant};
use ce_moteur::NetId;

use crate::brique::Projet;
use crate::document::{Document, Element, Genre, Id, Source};

/// Nom du composant qui représente le plan affiché.
const RACINE: &str = "plan";

/// Les briques d'un projet, traduites une fois pour toutes tant qu'elles ne changent pas.
#[derive(Clone, Debug, Default)]
pub struct Briques {
    revision: Option<u64>,
    bib: Bibliotheque,
}

impl Briques {
    /// Traduit les modèles du projet, sauf si c'est déjà fait pour cette révision.
    pub fn suivre(&mut self, projet: &Projet) {
        if self.revision == Some(projet.revision) {
            return;
        }
        let mut bib = Bibliotheque::nouvelle();
        for (k, m) in projet.modeles.iter().enumerate() {
            bib.ajouter(&format!("m{k}"), composant(&m.plan, projet));
        }
        self.bib = bib;
        self.revision = Some(projet.revision);
    }
}

/// Un plan mis à plat, avec de quoi retrouver ses signaux.
pub struct Pont {
    /// Le circuit mis à plat.
    pub circuit: Circuit,
    /// Pour chaque sortie d'élément du plan, le net de chacun de ses bits (`None` : en l'air).
    /// Trié par source.
    pub prises: Vec<(Source, Vec<Option<NetId>>)>,
    /// Pour chaque interrupteur ou clavier : le numéro de sa première entrée dans le moteur et
    /// son nombre de bits.
    pub entrees: BTreeMap<Id, (usize, u32)>,
}

impl Pont {
    /// Met à plat le plan `plan` du projet. `briques` doit avoir suivi le projet.
    pub fn construire(plan: &Document, projet: &Projet, briques: &Briques) -> Result<Pont, String> {
        let mut bib = briques.bib.clone();
        bib.ajouter(RACINE, composant(plan, projet));
        let circuit = Circuit::elaborer(&bib, RACINE).map_err(|e| e.to_string())?;
        let mut entrees = BTreeMap::new();
        for e in &plan.elements {
            if matches!(e.genre, Genre::Entree | Genre::Clavier) {
                if let Some(b) = circuit.broche(&format!("e{}", e.id)) {
                    if let Some(premiere) = b.premiere_entree {
                        entrees.insert(e.id, (premiere, b.nets.len() as u32));
                    }
                }
            }
        }
        let prises = prises(&circuit, plan, projet, &[]);
        Ok(Pont {
            circuit,
            prises,
            entrees,
        })
    }
}

/// Les nets de chaque sortie d'élément de `plan`, quand ce plan est celui d'une brique posée
/// au bout du chemin `exemplaire` dans le circuit mis à plat (chemin vide : `plan` est le plan
/// mis à plat lui-même). C'est ainsi qu'on regarde l'intérieur d'une brique pendant que tout
/// l'ordinateur tourne : ses entrées sont les vrais signaux qui lui arrivent d'en haut.
/// Trié par source.
pub fn prises(
    circuit: &Circuit,
    plan: &Document,
    projet: &Projet,
    exemplaire: &[Id],
) -> Vec<(Source, Vec<Option<NetId>>)> {
    let prefixe: String = exemplaire.iter().map(|id| format!("b{id}.")).collect();
    let mut prises = Vec::new();
    for e in &plan.elements {
        for (j, b) in e.broches_s.iter().enumerate() {
            let source = Source::nouvelle(e.id, j);
            let nets = (0..b.largeur)
                .map(|k| {
                    let extremite = bit(plan, projet, source, k, 0)?;
                    circuit.net(&format!("{prefixe}{extremite}")).ok()
                })
                .collect();
            prises.push((source, nets));
        }
    }
    prises.sort_by_key(|p| p.0);
    prises
}

/// Les nets d'une seule sortie d'élément de `plan`, ce plan étant celui de la brique posée au
/// bout du chemin `exemplaire` (voir [`prises`]).
pub fn nets_de_sortie(
    circuit: &Circuit,
    plan: &Document,
    projet: &Projet,
    exemplaire: &[Id],
    source: Source,
) -> Vec<Option<NetId>> {
    let prefixe: String = exemplaire.iter().map(|id| format!("b{id}.")).collect();
    let largeur = plan.largeur_source(source).unwrap_or(0);
    (0..largeur)
        .map(|k| {
            let extremite = bit(plan, projet, source, k, 0)?;
            circuit.net(&format!("{prefixe}{extremite}")).ok()
        })
        .collect()
}

/// Le plan écrit comme un composant `ce-circuit`.
fn composant(plan: &Document, projet: &Projet) -> Composant {
    let mut c = Composant::nouveau("");
    for e in &plan.elements {
        c = match e.genre {
            Genre::Entree | Genre::Clavier => {
                c.entree(&format!("e{}", e.id), e.broches_s[0].largeur)
            }
            Genre::Sortie => c.sortie(&format!("s{}", e.id), e.broches_e[0].largeur),
            Genre::Nand => c.inst(&format!("g{}", e.id), "NAND"),
            Genre::Horloge => c.horloge(&format!("h{}", e.id), e.param.max(1)),
            Genre::Brique => match projet.index(&e.modele) {
                Some(k) => c.inst(&format!("b{}", e.id), &format!("m{k}")),
                None => c,
            },
            Genre::Ecran | Genre::Decoupe | Genre::Jonction => c,
        };
    }
    // Un fil par bit de source : la source d'abord, puis tout ce qui la lit.
    let mut rang: BTreeMap<String, usize> = BTreeMap::new();
    let mut fils: Vec<Vec<String>> = Vec::new();
    for e in &plan.elements {
        for (i, src) in e.entrees.iter().enumerate() {
            let (Some(src), Some(base)) = (src, arrivee(e, i, projet)) else {
                continue;
            };
            let largeur = e.broches_e[i].largeur;
            for k in 0..largeur {
                let Some(depart) = bit(plan, projet, *src, k, 0) else {
                    continue;
                };
                let lecteur = if largeur == 1 {
                    base.clone()
                } else {
                    format!("{base}[{k}]")
                };
                let n = *rang.entry(depart.clone()).or_insert_with(|| {
                    fils.push(vec![depart]);
                    fils.len() - 1
                });
                fils[n].push(lecteur);
            }
        }
    }
    for f in &fils {
        c = c.fil(f);
    }
    c
}

/// Le nom, dans le composant, de l'entrée `i` d'un élément ; `None` si elle ne lit rien pour
/// le moteur (écran, découpeur, jonction, brique dont le modèle a disparu).
fn arrivee(e: &Element, i: usize, projet: &Projet) -> Option<String> {
    match e.genre {
        Genre::Nand => Some(format!("g{}.{}", e.id, if i == 0 { "a" } else { "b" })),
        Genre::Sortie => Some(format!("s{}", e.id)),
        Genre::Brique => {
            let b = &e.broches_e[i];
            let m = projet.modele(&e.modele)?;
            let dedans = m.plan.element(b.origine)?;
            (dedans.genre == Genre::Entree && dedans.broches_s[0].largeur == b.largeur)
                .then(|| format!("b{}.e{}", e.id, b.origine))
        }
        _ => None,
    }
}

/// Le nom, dans le composant, du bit `k` d'une sortie d'élément, en traversant découpeurs et
/// jonctions. `None` si ce bit n'est piloté par rien.
fn bit(
    plan: &Document,
    projet: &Projet,
    source: Source,
    k: u32,
    profondeur: u32,
) -> Option<String> {
    // Des découpeurs et des jonctions bouclés sur eux-mêmes ne pilotent rien.
    if profondeur > 64 {
        return None;
    }
    let e = plan.element(source.id)?;
    let b = e.broches_s.get(source.sortie)?;
    if k >= b.largeur {
        return None;
    }
    let indexe = |base: String| {
        if b.largeur == 1 {
            base
        } else {
            format!("{base}[{k}]")
        }
    };
    match e.genre {
        Genre::Entree | Genre::Clavier => Some(indexe(format!("e{}", e.id))),
        Genre::Nand => Some(format!("g{}.s", e.id)),
        Genre::Horloge => Some(format!("h{}.s", e.id)),
        Genre::Brique => {
            let m = projet.modele(&e.modele)?;
            let dedans = m.plan.element(b.origine)?;
            (dedans.genre == Genre::Sortie && dedans.broches_e[0].largeur == b.largeur)
                .then(|| indexe(format!("b{}.s{}", e.id, b.origine)))
        }
        Genre::Decoupe => {
            let (premier, dernier) = *e.plages.get(source.sortie)?;
            let amont = if premier <= dernier {
                premier + k
            } else {
                premier.checked_sub(k)?
            };
            bit(plan, projet, e.entrees[0]?, amont, profondeur + 1)
        }
        Genre::Jonction => {
            let mut debut = 0;
            for (i, be) in e.broches_e.iter().enumerate() {
                if k < debut + be.largeur {
                    return bit(plan, projet, e.entrees[i]?, k - debut, profondeur + 1);
                }
                debut += be.largeur;
            }
            None
        }
        Genre::Sortie | Genre::Ecran => None,
    }
}
