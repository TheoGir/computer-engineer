//! Les bancs d'essai écrits par l'utilisateur : une petite table qui dit, ligne après ligne,
//! quelles valeurs donner aux interrupteurs et ce que les lampes doivent afficher. Le banc
//! joue chaque ligne sur le circuit mis à plat et dit lesquelles sont justes.
//!
//! ```text
//! # Le demi-additionneur
//! a b -> s c
//! 0 0 -> 0 0
//! 1 1 -> 0 1
//! ```
//!
//! - La première ligne utile nomme les interrupteurs, puis `->`, puis les lampes.
//! - Chaque ligne suivante donne une valeur par nom : en décimal, `0b101` en binaire, `0x1F`
//!   en hexadécimal. `-` laisse un interrupteur comme il était, ou n'attend rien d'une lampe.
//! - Les lignes se jouent dans l'ordre, sur le même circuit : sa mémoire est gardée d'une
//!   ligne à l'autre. Un interrupteur peut donc servir d'horloge (`0` puis `1`).
//! - `attendre 50` laisse passer 50 unités de temps (pour un circuit qui a une horloge).
//! - `#` commence un commentaire.

use crate::brique::Projet;
use crate::document::{Document, Genre, Id};
use crate::pont::{Briques, Pont};

/// Le temps laissé au circuit pour se calmer après chaque ligne.
const BUDGET: u64 = 10_000;

/// Ce qu'a donné une ligne du banc.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct LigneJouee {
    /// Son numéro dans le texte, à partir de 1.
    pub numero: usize,
    /// Le texte de la ligne.
    pub texte: String,
    /// Ce qui ne va pas : vide si la ligne est juste.
    pub ecarts: Vec<String>,
}

impl LigneJouee {
    /// Vrai si toutes les lampes valent ce qui était attendu.
    pub fn juste(&self) -> bool {
        self.ecarts.is_empty()
    }
}

/// Une valeur écrite dans le banc : un nombre, ou `-`.
type Case = Option<u64>;

/// Lit une valeur : décimal, `0b…`, `0x…`, ou `-` (et `x`) pour « rien ».
fn lire_case(mot: &str) -> Result<Case, String> {
    let m = mot.trim().to_lowercase().replace('_', "");
    if m == "-" || m == "x" {
        return Ok(None);
    }
    let lu = if let Some(b) = m.strip_prefix("0b") {
        u64::from_str_radix(b, 2)
    } else if let Some(h) = m.strip_prefix("0x") {
        u64::from_str_radix(h, 16)
    } else {
        m.parse::<u64>()
    };
    lu.map(Some)
        .map_err(|_| format!("« {mot} » n'est pas une valeur (0, 1, 12, 0b101, 0x1F ou -)"))
}

/// Les mots d'une ligne, séparés par des espaces ou des virgules.
fn mots(texte: &str) -> Vec<&str> {
    texte
        .split(|c: char| c.is_whitespace() || c == ',')
        .filter(|m| !m.is_empty())
        .collect()
}

/// Coupe une ligne autour de `->` (ou `→`).
fn deux_cotes(ligne: &str) -> Option<(&str, &str)> {
    ligne.split_once("->").or_else(|| ligne.split_once('→'))
}

/// Un élément du plan désigné par son nom dans l'en-tête.
#[derive(Clone, Debug)]
struct Borne {
    nom: String,
    id: Id,
    largeur: u32,
}

/// Trouve l'élément de ce nom parmi ceux du genre voulu.
fn borne(plan: &Document, nom: &str, genre: Genre, numero: usize) -> Result<Borne, String> {
    let quoi = if genre == Genre::Entree {
        "interrupteur"
    } else {
        "lampe"
    };
    let trouves: Vec<_> = plan
        .elements
        .iter()
        .filter(|e| e.genre == genre && e.nom == nom)
        .collect();
    match trouves.as_slice() {
        [e] => Ok(Borne {
            nom: nom.to_owned(),
            id: e.id,
            largeur: if genre == Genre::Entree {
                e.broches_s[0].largeur
            } else {
                e.broches_e[0].largeur
            },
        }),
        [] => Err(format!(
            "ligne {numero} : aucun {quoi} ne s'appelle « {nom} » sur ce plan"
        )),
        _ => Err(format!(
            "ligne {numero} : plusieurs {quoi}s s'appellent « {nom} » ; donne-leur des noms différents"
        )),
    }
}

/// Une valeur qui tient dans `largeur` bits.
fn tient(v: u64, largeur: u32) -> bool {
    largeur >= 64 || v < (1u64 << largeur)
}

/// Joue le banc `texte` sur le plan `plan` du projet. `Err` si le texte ne se lit pas ou si
/// le plan ne se simule pas ; sinon, une ligne jouée par ligne de valeurs.
pub fn jouer(projet: &Projet, plan: &Document, texte: &str) -> Result<Vec<LigneJouee>, String> {
    let mut entete: Option<(Vec<Borne>, Vec<Borne>)> = None;
    let mut banc: Option<ce_circuit::Banc> = None;
    let mut jouees = Vec::new();
    for (i, brut) in texte.lines().enumerate() {
        let numero = i + 1;
        let ligne = brut.split('#').next().unwrap_or("").trim();
        if ligne.is_empty() {
            continue;
        }
        let Some((entrees, sorties)) = &entete else {
            let (g, d) = deux_cotes(ligne).ok_or_else(|| {
                format!("ligne {numero} : la première ligne nomme les interrupteurs, puis « -> », puis les lampes (par exemple « a b -> s »)")
            })?;
            let entrees = mots(g)
                .into_iter()
                .map(|n| borne(plan, n, Genre::Entree, numero))
                .collect::<Result<Vec<_>, _>>()?;
            let sorties = mots(d)
                .into_iter()
                .map(|n| borne(plan, n, Genre::Sortie, numero))
                .collect::<Result<Vec<_>, _>>()?;
            if sorties.is_empty() {
                return Err(format!(
                    "ligne {numero} : nomme au moins une lampe après « -> »"
                ));
            }
            entete = Some((entrees, sorties));
            continue;
        };
        // Le circuit n'est mis à plat qu'une fois l'en-tête lu, et une seule fois.
        if banc.is_none() {
            let mut briques = Briques::default();
            briques.suivre(projet);
            let pont = Pont::construire(plan, projet, &briques)?;
            let mut b = ce_circuit::Banc::de_circuit(pont.circuit).map_err(|e| e.to_string())?;
            for e in plan.elements.iter().filter(|e| e.genre == Genre::Entree) {
                let _ = b.ecrire(&format!("e{}", e.id), 0);
            }
            b.stabiliser(BUDGET);
            banc = Some(b);
        }
        let Some(b) = banc.as_mut() else {
            continue;
        };
        let premiers = mots(ligne);
        if let Some(&"attendre") = premiers.first() {
            let duree: u64 = premiers
                .get(1)
                .and_then(|d| d.parse().ok())
                .filter(|_| premiers.len() == 2)
                .ok_or_else(|| format!("ligne {numero} : écris « attendre » suivi d'un nombre"))?;
            b.avancer(duree);
            continue;
        }
        let (g, d) = deux_cotes(ligne).ok_or_else(|| {
            format!("ligne {numero} : il manque « -> » entre les interrupteurs et les lampes")
        })?;
        let (valeurs_e, valeurs_s) = (mots(g), mots(d));
        if valeurs_e.len() != entrees.len() || valeurs_s.len() != sorties.len() {
            return Err(format!(
                "ligne {numero} : {} valeur{} avant « -> » et {} après, comme dans l'en-tête",
                entrees.len(),
                if entrees.len() > 1 { "s" } else { "" },
                sorties.len()
            ));
        }
        for (borne, mot) in entrees.iter().zip(&valeurs_e) {
            if let Some(v) = lire_case(mot).map_err(|e| format!("ligne {numero} : {e}"))? {
                if !tient(v, borne.largeur) {
                    return Err(format!(
                        "ligne {numero} : {v} ne tient pas sur les {} bit{} de « {} »",
                        borne.largeur,
                        if borne.largeur > 1 { "s" } else { "" },
                        borne.nom
                    ));
                }
                b.ecrire(&format!("e{}", borne.id), v)
                    .map_err(|e| e.to_string())?;
            }
        }
        b.stabiliser(BUDGET);
        let mut ecarts = Vec::new();
        for (borne, mot) in sorties.iter().zip(&valeurs_s) {
            let Some(attendue) = lire_case(mot).map_err(|e| format!("ligne {numero} : {e}"))?
            else {
                continue;
            };
            let lue = b
                .lire(&format!("s{}", borne.id))
                .map_err(|e| e.to_string())?;
            if lue != Some(attendue) {
                let obtenue = match lue {
                    Some(v) => v.to_string(),
                    None => b
                        .lire_texte(&format!("s{}", borne.id))
                        .unwrap_or_else(|_| "?".to_owned())
                        .to_uppercase(),
                };
                ecarts.push(format!(
                    "{} : attendu {attendue}, obtenu {obtenue}",
                    borne.nom
                ));
            }
        }
        jouees.push(LigneJouee {
            numero,
            texte: ligne.to_owned(),
            ecarts,
        });
    }
    if entete.is_none() {
        return Err("Écris d'abord l'en-tête : les interrupteurs, « -> », puis les lampes (par exemple « a b -> s »).".to_owned());
    }
    Ok(jouees)
}

/// Un banc tout prêt pour ce plan : l'en-tête avec ses interrupteurs et ses lampes, et une
/// première ligne à compléter.
pub fn modele_de_banc(plan: &Document) -> String {
    let noms = |genre: Genre| -> Vec<String> {
        plan.elements
            .iter()
            .filter(|e| e.genre == genre && !e.nom.is_empty())
            .map(|e| e.nom.clone())
            .collect()
    };
    let (entrees, sorties) = (noms(Genre::Entree), noms(Genre::Sortie));
    let zeros = |n: usize| vec!["0"; n].join(" ");
    let tirets = |n: usize| vec!["-"; n].join(" ");
    format!(
        "# Une ligne par essai : les valeurs des interrupteurs, « -> », ce que les lampes\n# doivent afficher (- : peu importe).\n{} -> {}\n{} -> {}\n",
        entrees.join(" "),
        sorties.join(" "),
        zeros(entrees.len()),
        tirets(sorties.len())
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    fn demi() -> (Projet, Document) {
        let plan = Document::demi_additionneur();
        (Projet::nouveau("", plan.clone()), plan)
    }

    #[test]
    fn un_banc_juste_et_un_banc_faux() {
        let (projet, plan) = demi();
        let juste =
            "# demi-additionneur\na b -> s c\n0 0 -> 0 0\n0 1 -> 1 0\n1 0 -> 1 -\n1 1 -> 0 1\n";
        let lignes = jouer(&projet, &plan, juste).unwrap();
        assert_eq!(lignes.len(), 4);
        assert!(lignes.iter().all(LigneJouee::juste), "{lignes:?}");
        assert_eq!(lignes[0].numero, 3);
        let faux = "a, b -> s, c\n1 1 -> 1 0x1\n";
        let lignes = jouer(&projet, &plan, faux).unwrap();
        assert_eq!(lignes[0].ecarts, ["s : attendu 1, obtenu 0"]);
    }

    #[test]
    fn le_tiret_garde_l_interrupteur_comme_il_etait() {
        let (projet, plan) = demi();
        let lignes = jouer(&projet, &plan, "a b -> c\n1 1 -> 1\n- 0 -> 0\n- 1 -> 1\n").unwrap();
        assert!(lignes.iter().all(LigneJouee::juste), "{lignes:?}");
    }

    #[test]
    fn les_erreurs_disent_la_ligne() {
        let (projet, plan) = demi();
        let erreur = |t: &str| jouer(&projet, &plan, t).unwrap_err();
        assert!(erreur("a b s c").contains("ligne 1"));
        assert!(erreur("a z -> s").contains("« z »"));
        assert!(erreur("a b -> s\n1 -> 1").contains("ligne 2"));
        assert!(erreur("a b -> s\n2 0 -> 1").contains("ne tient pas"));
        assert!(erreur("a b -> s\n1 0 -> deux").contains("« deux »"));
        assert!(erreur("# rien\n").contains("en-tête"));
    }

    #[test]
    fn la_memoire_est_gardee_d_une_ligne_a_l_autre() {
        // Un verrou fait de deux NAND : r (actif à 0) remet à 0, m (actif à 0) met à 1.
        let mut plan = Document::nouveau();
        let m = plan.poser(
            crate::document::Element::entree("m", 1),
            crate::document::Point::default(),
        );
        let r = plan.poser(
            crate::document::Element::entree("r", 1),
            crate::document::Point::default(),
        );
        let g1 = plan.poser(
            crate::document::Element::nand("g1"),
            crate::document::Point::default(),
        );
        let g2 = plan.poser(
            crate::document::Element::nand("g2"),
            crate::document::Point::default(),
        );
        let q = plan.poser(
            crate::document::Element::sortie("q", "q", 1),
            crate::document::Point::default(),
        );
        use crate::document::{Fil, Source};
        plan.relier(
            Source::de(m),
            Fil {
                puits: g1,
                entree: 0,
            },
        );
        plan.relier(
            Source::de(g2),
            Fil {
                puits: g1,
                entree: 1,
            },
        );
        plan.relier(
            Source::de(r),
            Fil {
                puits: g2,
                entree: 0,
            },
        );
        plan.relier(
            Source::de(g1),
            Fil {
                puits: g2,
                entree: 1,
            },
        );
        plan.relier(
            Source::de(g1),
            Fil {
                puits: q,
                entree: 0,
            },
        );
        let projet = Projet::nouveau("", plan.clone());
        let texte = "m r -> q\n1 0 -> 0\n1 1 -> 0\n0 1 -> 1\n1 1 -> 1\n1 0 -> 0\n";
        let lignes = jouer(&projet, &plan, texte).unwrap();
        assert!(lignes.iter().all(LigneJouee::juste), "{lignes:?}");
    }

    #[test]
    fn le_modele_de_banc_se_joue_tel_quel() {
        let (projet, plan) = demi();
        let texte = modele_de_banc(&plan);
        assert!(texte.contains("a b -> s c"), "{texte}");
        let lignes = jouer(&projet, &plan, &texte).unwrap();
        assert_eq!(lignes.len(), 1);
        assert!(lignes[0].juste());
    }
}
