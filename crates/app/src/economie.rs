//! La construction minimale : à quel point une construction est-elle économe en portes NAND ?
//!
//! Pour chaque construction, on compare le nombre de NAND de l'utilisateur à la plus petite
//! construction connue, en disant toujours ce qu'on en sait vraiment (voir
//! [`ce_circuit::minimum`]) :
//!
//! - une **quête** se compare à son corrigé, ou à mieux quand la recherche a trouvé mieux ;
//!   un ordinateur ou un grand projet se compare sur ce que l'on ajoute à l'atelier ;
//! - une **brique gagnée** dans le parcours se compare au corrigé de sa quête ;
//! - **n'importe quel plan** sans mémoire, de 3 bits d'entrée au plus, se compare au minimum
//!   prouvé de chacune de ses sorties (toutes les fonctions de 3 entrées ont été résolues).
//!
//! Rien ici ne dépend d'egui : le panneau ne fait que dessiner un [`Ecart`].

use std::collections::BTreeMap;
use std::sync::{Mutex, OnceLock};

use ce_circuit::minimum::{self, Minimum, Statut};
use ce_moteur::Valeur;

use crate::brique::Projet;
use crate::document::{Document, Genre};
use crate::parcours::Parcours;
use crate::pont::{Briques, Pont};
use crate::quete::{corriges, Fiche, Quete};

/// À quoi l'on se compare.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Cible {
    /// Une construction connue, de ce nombre de portes.
    Connue(Minimum),
    /// Seulement une borne basse prouvée : il faut au moins ce nombre de portes. C'est le cas
    /// d'un plan à plusieurs sorties : chaque sortie seule a un minimum prouvé, mais pas
    /// l'ensemble, car des portes peuvent servir à plusieurs sorties.
    AuMoins(u64),
}

impl Cible {
    /// Le nombre de portes visé.
    pub fn nand(self) -> u64 {
        match self {
            Cible::Connue(m) => m.nand,
            Cible::AuMoins(n) => n,
        }
    }
}

/// D'où vient la comparaison : sert à la phrase affichée.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Origine {
    /// Le corrigé (ou mieux) de la quête en cours.
    Quete,
    /// Ce qu'on ajoute à l'atelier d'un ordinateur ou d'un grand projet.
    Ajouts,
    /// La brique gagnée dans le parcours, qui porte ce nom.
    Brique(String),
    /// La fonction calculée par le plan, reconnue par sa table de vérité.
    Fonction,
}

/// L'écart entre une construction et la plus petite connue.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Ecart {
    /// Les portes NAND de l'utilisateur.
    pub tiennes: u64,
    /// La cible.
    pub cible: Cible,
    /// D'où vient la cible.
    pub origine: Origine,
}

/// Au plus ce rapport (en pour cent) entre ses portes et la cible, deux étoiles.
pub const DEUX_ETOILES_POUR_CENT: u64 = 150;

impl Ecart {
    /// De 0 à 1 : la cible divisée par ses portes (1 quand on fait aussi bien ou mieux).
    pub fn proximite(&self) -> f32 {
        let cible = self.cible.nand();
        if self.tiennes == 0 || self.tiennes <= cible {
            return 1.0;
        }
        cible as f32 / self.tiennes as f32
    }

    /// Les étoiles d'une construction juste : 3 quand on atteint la cible, 2 jusqu'à
    /// [`DEUX_ETOILES_POUR_CENT`] %, 1 au-delà. Sans construction connue (borne basse seule),
    /// 3 étoiles seulement si l'on atteint la borne, qui est alors le minimum.
    pub fn etoiles(&self) -> u8 {
        let cible = self.cible.nand();
        if self.tiennes <= cible {
            3
        } else if matches!(self.cible, Cible::Connue(_))
            && self.tiennes * 100 <= cible * DEUX_ETOILES_POUR_CENT
        {
            2
        } else {
            1
        }
    }

    /// Portes de plus que la cible (0 si on l'atteint).
    pub fn en_trop(&self) -> u64 {
        self.tiennes.saturating_sub(self.cible.nand())
    }

    /// Le titre court : la cible et ce qu'on en sait.
    pub fn titre(&self) -> String {
        let n = self.cible.nand();
        match self.cible {
            Cible::Connue(m) => match m.statut {
                Statut::Prouve => tr!("Minimum prouvé : {n} {v}", n = n, v = portes(n)),
                Statut::Encadre { .. } => tr!("Meilleure connue : {n} {v}", n = n, v = portes(n)),
                Statut::Reference => tr!("Référence : {n} {v}", n = n, v = portes(n)),
            },
            Cible::AuMoins(_) => tr!("Au moins {n} {v}", n = n, v = portes(n)),
        }
    }

    /// La phrase qui dit où l'on en est, pour une construction juste.
    pub fn phrase(&self) -> String {
        let n = self.cible.nand();
        let d = self.en_trop();
        let mieux = self.cible.nand().saturating_sub(self.tiennes);
        match self.cible {
            Cible::Connue(m) => match m.statut {
                Statut::Prouve if d == 0 => {
                    tr!("Construction minimale : il est prouvé qu'on ne peut pas faire avec moins de portes.").to_owned()
                }
                Statut::Prouve => tr!("Il est prouvé qu'on peut le faire en {n} {v} : tu peux encore en économiser {d}.", n = n, d = d, v = portes(n)),
                Statut::Encadre { borne_basse } if mieux > 0 => tr!("Mieux que la meilleure construction connue ({n} {v}) ! On sait seulement qu'il en faut au moins {borne_basse}.", n = n, borne_basse = borne_basse, v = portes(n)),
                Statut::Encadre { borne_basse } if d == 0 => tr!("Aussi économe que la meilleure construction connue. Le vrai minimum est entre {borne_basse} et {n} portes : personne ne le connaît encore.", borne_basse = borne_basse, n = n),
                Statut::Encadre { borne_basse } => tr!("La meilleure construction connue en utilise {n} (il en faut au moins {borne_basse}) : tu en as {d} de plus.", n = n, borne_basse = borne_basse, d = d),
                Statut::Reference if mieux > 0 => tr!("Tu fais mieux que la construction de référence ({n} {v}) : bravo !", n = n, v = portes(n)),
                Statut::Reference if d == 0 => {
                    tr!("Autant de portes que la construction de référence. Personne n'a prouvé qu'on ne peut pas faire mieux : à toi de chercher !").to_owned()
                }
                Statut::Reference => tr!("La construction de référence en utilise {n} : tu en as {d} de plus.", n = n, d = d),
            },
            Cible::AuMoins(_) if d == 0 => {
                tr!("Construction minimale : chaque sortie demande à elle seule ce nombre de portes.").to_owned()
            }
            Cible::AuMoins(_) => tr!("Il en faut au moins {n} (le minimum prouvé de ta sortie la plus coûteuse) ; selon ce que les sorties partagent, on peut parfois s'en approcher.", n = n),
        }
    }
}

fn portes(n: u64) -> &'static str {
    if n > 1 {
        "portes"
    } else {
        "porte"
    }
}

/// La plus petite construction connue d'un corrigé, calculée une fois.
pub fn minimum_du_corrige(nom: &str) -> Option<Minimum> {
    static CACHE: OnceLock<Mutex<BTreeMap<String, Option<Minimum>>>> = OnceLock::new();
    let cache = CACHE.get_or_init(Default::default);
    if let Some(m) = cache.lock().ok().and_then(|c| c.get(nom).copied()) {
        return m;
    }
    let m = minimum::minimum(corriges(), nom);
    if let Ok(mut c) = cache.lock() {
        c.insert(nom.to_owned(), m);
    }
    m
}

/// Ce que la construction de référence ajoute à l'atelier d'un ordinateur ou d'un grand
/// projet : les briques posées en plus du câblage (le test
/// `les_ajouts_de_reference_sont_ceux_des_solutions` le vérifie sur les solutions).
pub const AJOUTS: &[(&str, &[&str])] = &[
    ("ordinateur", &["NON", "NON", "ET"]),
    ("ordinateur-majuscules", &["NON", "NON", "ET"]),
    (
        "ordinateur-a-toi",
        &["NON", "NON", "ET", "PROCESSEUR", "RAM16"],
    ),
    ("ordinateur-256", &["NON", "NON", "ET", "RAM256"]),
    ("messagerie", &["REGISTRE8", "NON"]),
    ("messagerie-chiffree", &["REGISTRE8", "NON"]),
];

/// Les portes déjà posées dans le plan de départ d'une quête, calculées une fois.
pub fn fournies(fiche: &'static Fiche) -> u64 {
    if !fiche.epreuve.ordinateur() {
        return 0;
    }
    static CACHE: OnceLock<Mutex<BTreeMap<&'static str, u64>>> = OnceLock::new();
    let cache = CACHE.get_or_init(Default::default);
    if let Some(n) = cache.lock().ok().and_then(|c| c.get(fiche.cle).copied()) {
        return n;
    }
    let n = Parcours::default()
        .projet(fiche, "depart")
        .map_or(0, |p| p.nb_portes(&p.principal));
    if let Ok(mut c) = cache.lock() {
        c.insert(fiche.cle, n);
    }
    n
}

/// La cible d'une quête : son corrigé (ou mieux), ou, pour un ordinateur ou un grand projet,
/// ce qu'ajoute la construction de référence.
pub fn cible_de_la_quete(fiche: &'static Fiche) -> Option<(Cible, Origine)> {
    if fiche.epreuve.ordinateur() {
        let (_, ajouts) = AJOUTS.iter().find(|(cle, _)| *cle == fiche.cle)?;
        let mut n = 0;
        for nom in *ajouts {
            n += minimum_du_corrige(nom)?.nand;
        }
        let statut = if n == 0 {
            Statut::Prouve
        } else {
            Statut::Reference
        };
        return Some((Cible::Connue(Minimum { nand: n, statut }), Origine::Ajouts));
    }
    let m = minimum_du_corrige(fiche.corrige)?;
    Some((Cible::Connue(m), Origine::Quete))
}

/// L'écart de la quête en cours, jouée sur `plan` (le plan principal du projet).
pub fn de_la_quete(quete: &Quete, projet: &Projet, plan: &Document) -> Option<Ecart> {
    let fiche = Quete::fiche(quete.cle)?;
    let (cible, origine) = cible_de_la_quete(fiche)?;
    let tiennes = projet.nb_portes(plan).saturating_sub(fournies(fiche));
    Some(Ecart {
        tiennes,
        cible,
        origine,
    })
}

/// L'écart de la brique gagnée d'une quête réussie, d'après la collection du parcours.
pub fn de_la_brique_gagnee(parcours: &Parcours, fiche: &'static Fiche) -> Option<Ecart> {
    if fiche.epreuve.ordinateur() || !parcours.faite(fiche.cle) {
        return None;
    }
    let modele = parcours.briques.iter().find(|m| m.nom == fiche.brique)?;
    let mut projet = Projet::nouveau("", Document::nouveau());
    projet.modeles = parcours.briques.clone();
    let tiennes = projet.nb_portes(&modele.plan);
    let (cible, _) = cible_de_la_quete(fiche)?;
    Some(Ecart {
        tiennes,
        cible,
        origine: Origine::Brique(fiche.brique.to_owned()),
    })
}

/// Les étoiles de chaque quête du parcours, dans l'ordre de [`crate::quete::PARCOURS`] :
/// `None` pour une quête pas encore réussie, ou sans brique gagnée (un ordinateur).
pub fn etoiles_du_parcours(parcours: &Parcours) -> Vec<Option<u8>> {
    let mut projet = Projet::nouveau("", Document::nouveau());
    projet.modeles = parcours.briques.clone();
    crate::quete::PARCOURS
        .iter()
        .map(|f| {
            if f.epreuve.ordinateur() || !parcours.faite(f.cle) {
                return None;
            }
            let modele = projet.modele(f.brique)?;
            let (cible, _) = cible_de_la_quete(f)?;
            let e = Ecart {
                tiennes: projet.nb_portes(&modele.plan),
                cible,
                origine: Origine::Quete,
            };
            Some(e.etoiles())
        })
        .collect()
}

/// Bits d'entrée au-delà desquels on ne cherche pas le minimum d'une fonction.
pub const BITS_MAX_FONCTION: u32 = 3;

/// L'écart d'un plan quelconque (le plan principal ou celui d'une brique) : la brique du
/// parcours de même nom si c'en est une, sinon la fonction qu'il calcule, s'il est sans
/// mémoire et a au plus [`BITS_MAX_FONCTION`] bits d'entrée.
pub fn du_plan(
    projet: &Projet,
    plan: &Document,
    nom_de_brique: Option<&str>,
    parcours: &Parcours,
) -> Option<Ecart> {
    let tiennes = projet.nb_portes(plan);
    if let Some(nom) = nom_de_brique {
        let gagnee = parcours.briques.iter().any(|m| m.nom == nom);
        let fiche = crate::quete::PARCOURS
            .iter()
            .find(|f| f.brique == nom && !f.epreuve.ordinateur());
        if let (true, Some(f)) = (gagnee, fiche) {
            let (cible, _) = cible_de_la_quete(f)?;
            return Some(Ecart {
                tiennes,
                cible,
                origine: Origine::Brique(nom.to_owned()),
            });
        }
    }
    let cible = cible_d_une_fonction(projet, plan)?;
    Some(Ecart {
        tiennes,
        cible,
        origine: Origine::Fonction,
    })
}

/// La cible de la fonction calculée par un plan sans mémoire, de 3 bits d'entrée au plus,
/// dont toutes les sorties sont branchées.
pub fn cible_d_une_fonction(projet: &Projet, plan: &Document) -> Option<Cible> {
    let simple = plan.elements.iter().all(|e| match e.genre {
        Genre::Entree => e.param == 0,
        Genre::Sortie | Genre::Nand | Genre::Brique | Genre::Decoupe | Genre::Jonction => true,
        Genre::Horloge | Genre::Ecran | Genre::Clavier => false,
    });
    let (e, s) = plan.interface();
    let bits: u32 = e.iter().map(|b| b.largeur).sum();
    if !simple || s.is_empty() || bits > BITS_MAX_FONCTION {
        return None;
    }
    let mut briques = Briques::default();
    briques.suivre(projet);
    let circuit = Pont::construire(plan, projet, &briques).ok()?.circuit;
    let table = ce_circuit::analyse::table_du_circuit(circuit).ok()?;
    let nb_sorties = table.lignes.first()?.len();
    let mut minima = Vec::with_capacity(nb_sorties);
    let mut fonctions = Vec::with_capacity(nb_sorties);
    for o in 0..nb_sorties {
        let mut f = 0u64;
        for (r, ligne) in table.lignes.iter().enumerate() {
            match ligne[o] {
                Valeur::Un => f |= 1 << r,
                Valeur::Zero => {}
                // Une sortie en l'air ou indéterminée : pas de fonction à comparer.
                _ => return None,
            }
        }
        minima.push(minimum::minimum_d_une_fonction(bits, f)?);
        fonctions.push(f);
    }
    // Les mêmes sorties qu'un composant dont le minimum est établi (un demi-additionneur, un
    // décodeur…), à l'ordre près des entrées et des sorties : son minimum est le leur.
    if let Some(m) = composant_reconnu(bits, &fonctions) {
        return Some(Cible::Connue(m));
    }
    let haut = minima.iter().copied().max()?;
    if nb_sorties == 1 {
        Some(Cible::Connue(Minimum {
            nand: haut,
            statut: Statut::Prouve,
        }))
    } else {
        Some(Cible::AuMoins(haut))
    }
}

/// La table de `n` entrées `f` (bit `r` = valeur sur la ligne `r`), une fois ses entrées
/// renumérotées : l'entrée `i` devient l'entrée `ordre[i]`.
fn permuter(n: u32, f: u64, ordre: &[usize]) -> u64 {
    let mut g = 0u64;
    for r in 0..(1u64 << n) {
        let mut r2 = 0u64;
        for (i, &j) in ordre.iter().enumerate() {
            if (r >> i) & 1 == 1 {
                r2 |= 1 << j;
            }
        }
        if (f >> r) & 1 == 1 {
            g |= 1 << r2;
        }
    }
    g
}

/// Les tables des composants combinatoires de 3 bits d'entrée au plus dont le minimum est
/// établi : bits d'entrée, une table par bit de sortie, minimum.
fn composants_etablis() -> &'static [(u32, Vec<u64>, Minimum)] {
    static TABLES: OnceLock<Vec<(u32, Vec<u64>, Minimum)>> = OnceLock::new();
    TABLES.get_or_init(|| {
        let mut v = Vec::new();
        for nom in minimum::etablis().keys() {
            let Ok(t) = ce_circuit::analyse::table_de_verite(corriges(), nom) else {
                continue;
            };
            let n: u32 = t.entrees.iter().map(|(_, l)| l).sum();
            let Some(m) = minimum_du_corrige(nom) else {
                continue;
            };
            if n > BITS_MAX_FONCTION {
                continue;
            }
            let sorties = t.lignes.first().map_or(0, Vec::len);
            let tables = (0..sorties)
                .map(|o| {
                    t.lignes
                        .iter()
                        .enumerate()
                        .filter(|(_, l)| l[o] == Valeur::Un)
                        .fold(0u64, |f, (r, _)| f | 1 << r)
                })
                .collect();
            v.push((n, tables, m));
        }
        v
    })
}

/// Le minimum établi d'un composant qui calcule exactement ces sorties, entrées et sorties
/// prises dans n'importe quel ordre.
fn composant_reconnu(n: u32, fonctions: &[u64]) -> Option<Minimum> {
    let mut voulu = fonctions.to_vec();
    voulu.sort_unstable();
    let ordres: Vec<Vec<usize>> = match n {
        0 | 1 => vec![(0..n as usize).collect()],
        2 => vec![vec![0, 1], vec![1, 0]],
        3 => vec![
            vec![0, 1, 2],
            vec![0, 2, 1],
            vec![1, 0, 2],
            vec![1, 2, 0],
            vec![2, 0, 1],
            vec![2, 1, 0],
        ],
        _ => return None,
    };
    composants_etablis()
        .iter()
        .filter(|(m, tables, _)| *m == n && tables.len() == fonctions.len())
        .find(|(_, tables, _)| {
            ordres.iter().any(|ordre| {
                let mut t: Vec<u64> = tables.iter().map(|&f| permuter(n, f, ordre)).collect();
                t.sort_unstable();
                t == voulu
            })
        })
        .map(|(_, _, m)| *m)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::document::{Element, Fil, Point, Source};

    /// Un plan : deux interrupteurs a et b, une lampe s, et un OU exclusif fait de NAND, en
    /// `portes` portes (4 : la construction minimale ; 5 : une porte inutile en plus).
    fn ou_exclusif(portes: usize) -> Document {
        let mut d = Document::nouveau();
        let a = d.poser(Element::entree("a", 1), Point::nouveau(0.0, 0.0));
        let b = d.poser(Element::entree("b", 1), Point::nouveau(0.0, 100.0));
        let s = d.poser(Element::sortie("s", "s", 1), Point::nouveau(500.0, 50.0));
        let mut g = Vec::new();
        for i in 0..portes {
            g.push(d.poser(
                Element::nand(&format!("g{i}")),
                Point::nouveau(200.0, 60.0 * i as f32),
            ));
        }
        let mut relier = |source: Id, puits: Id, entree: usize| {
            assert!(d.relier(Source::de(source), Fil { puits, entree }));
        };
        relier(a, g[0], 0);
        relier(b, g[0], 1);
        relier(a, g[1], 0);
        relier(g[0], g[1], 1);
        relier(b, g[2], 0);
        relier(g[0], g[2], 1);
        relier(g[1], g[3], 0);
        relier(g[2], g[3], 1);
        relier(g[3], s, 0);
        if portes > 4 {
            relier(a, g[4], 0);
            relier(b, g[4], 1);
        }
        d
    }

    use crate::document::Id;

    #[test]
    fn un_plan_de_trois_entrees_au_plus_se_compare_a_son_minimum_prouve() {
        let parcours = Parcours::default();
        let plan = ou_exclusif(4);
        let projet = Projet::nouveau("essai", plan.clone());
        let e = du_plan(&projet, &plan, None, &parcours).unwrap();
        assert_eq!(e.tiennes, 4);
        assert_eq!(
            e.cible,
            Cible::Connue(Minimum {
                nand: 4,
                statut: Statut::Prouve
            })
        );
        assert_eq!(e.etoiles(), 3);
        assert_eq!(e.proximite(), 1.0);
        assert!(e.phrase().starts_with("Construction minimale"));

        let plan = ou_exclusif(5);
        let projet = Projet::nouveau("essai", plan.clone());
        let e = du_plan(&projet, &plan, None, &parcours).unwrap();
        assert_eq!((e.tiennes, e.en_trop(), e.etoiles()), (5, 1, 2));
        assert!((e.proximite() - 0.8).abs() < 1e-6);
        assert!(e.phrase().contains("encore en économiser 1"));
    }

    #[test]
    fn un_demi_additionneur_dessine_a_la_main_est_reconnu() {
        // Le OU exclusif de 4 portes, plus une lampe r = ET(a, b) = NON(g0) : 5 portes, le
        // minimum prouvé du demi-additionneur, même si les lampes sont dans l'autre ordre.
        let mut plan = ou_exclusif(5);
        let ids: Vec<Id> = plan.elements.iter().map(|e| e.id).collect();
        let g0 = plan.nomme("g0").unwrap().id;
        let g4 = plan.nomme("g4").unwrap().id;
        // g4 devient NON(g0) : ses deux entrées sur g0.
        assert!(plan.relier(
            Source::de(g0),
            Fil {
                puits: g4,
                entree: 0
            }
        ));
        assert!(plan.relier(
            Source::de(g0),
            Fil {
                puits: g4,
                entree: 1
            }
        ));
        let r = plan.poser(Element::sortie("r", "r", 1), Point::nouveau(500.0, -100.0));
        assert!(plan.relier(
            Source::de(g4),
            Fil {
                puits: r,
                entree: 0
            }
        ));
        assert!(!ids.is_empty());
        let projet = Projet::nouveau("essai", plan.clone());
        let e = du_plan(&projet, &plan, None, &Parcours::default()).unwrap();
        assert_eq!(
            e.cible,
            Cible::Connue(Minimum {
                nand: 5,
                statut: Statut::Prouve
            })
        );
        assert_eq!(e.etoiles(), 3);
    }

    #[test]
    fn permuter_les_entrees_garde_la_fonction() {
        // a0 ET NON a1 (lignes : seule la ligne 1 vaut 1), entrées échangées : ligne 2.
        assert_eq!(permuter(2, 0b0010, &[1, 0]), 0b0100);
        assert_eq!(permuter(3, 0b1110_1000, &[2, 0, 1]), 0b1110_1000);
    }

    #[test]
    fn une_sortie_en_l_air_ne_donne_pas_de_fonction() {
        let mut plan = ou_exclusif(4);
        plan.poser(Element::sortie("t", "t", 1), Point::nouveau(500.0, 300.0));
        let projet = Projet::nouveau("essai", plan.clone());
        assert_eq!(du_plan(&projet, &plan, None, &Parcours::default()), None);
    }

    #[test]
    fn les_etoiles_suivent_l_ecart() {
        let cible = |nand, statut| Cible::Connue(Minimum { nand, statut });
        let e = |tiennes, cible| Ecart {
            tiennes,
            cible,
            origine: Origine::Quete,
        };
        let prouve = cible(10, Statut::Prouve);
        assert_eq!(e(10, prouve).etoiles(), 3);
        assert_eq!(e(15, prouve).etoiles(), 2);
        assert_eq!(e(16, prouve).etoiles(), 1);
        let reference = cible(10, Statut::Reference);
        assert_eq!(e(8, reference).etoiles(), 3);
        assert!(e(8, reference)
            .phrase()
            .contains("mieux que la construction de référence"));
        let encadre = cible(10, Statut::Encadre { borne_basse: 7 });
        assert!(e(10, encadre).phrase().contains("entre 7 et 10"));
        // Une borne basse seule : pas de deuxième étoile, faute de construction connue.
        assert_eq!(e(3, Cible::AuMoins(3)).etoiles(), 3);
        assert_eq!(e(4, Cible::AuMoins(3)).etoiles(), 1);
    }

    #[test]
    fn chaque_quete_a_une_cible() {
        for f in crate::quete::PARCOURS {
            let (cible, _) = cible_de_la_quete(f).unwrap_or_else(|| panic!("{}", f.cle));
            if !f.epreuve.ordinateur() {
                assert!(cible.nand() > 0, "{}", f.cle);
            }
        }
        // Le corrigé du nombre premier fait 115 portes ; la recherche en a trouvé bien moins.
        let premier = crate::quete::PARCOURS
            .iter()
            .find(|f| f.corrige == "PREMIER")
            .unwrap();
        assert!(cible_de_la_quete(premier).unwrap().0.nand() < 115);
    }
}
