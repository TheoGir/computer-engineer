//! Analyse logique, pour comprendre un circuit sans mémoire :
//! - sa table de vérité complète, calculée en simulant toutes les combinaisons d'entrées ;
//! - l'expression simplifiée de chacune de ses sorties, en somme de produits minimale
//!   (méthode de Quine-McCluskey et de Petrick, bibliothèque `quine-mc_cluskey`) ;
//! - les indices des quêtes, révélés un par un.
//!
//! Notation des expressions : `a·b` pour ET, `a + b` pour OU, `¬a` pour NON.

use ce_circuit::analyse::{table_de_verite, table_du_circuit};
use ce_circuit::Sens;
use ce_moteur::Valeur;
use quine_mc_cluskey::{Bool, Term};

use crate::brique::Projet;
use crate::document::{Document, Genre};
use crate::pont::{Briques, Pont};
use crate::quete::{corriges, Epreuve, Fiche, Quete, PARCOURS};

/// Au-delà de ce nombre de bits d'entrée, la table n'est plus lisible à l'écran (256 lignes).
pub const MAX_BITS_TABLE: u32 = 8;
/// Au-delà de ce nombre de bits d'entrée, on ne simplifie pas : la méthode de Petrick peut
/// devenir très longue, et l'expression illisible.
pub const MAX_BITS_EXPRESSION: u32 = 6;

/// Au-delà de ce nombre d'essais, la recherche de la plus petite couverture s'arrête et garde
/// la meilleure trouvée : l'écran ne doit jamais se figer.
const BUDGET_COUVERTURE: u32 = 200_000;

/// Une expression simplifiée.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Simplifiee {
    /// Le texte, par exemple `a·¬b + ¬a·b`.
    pub texte: String,
    /// Vrai si l'on sait qu'aucune somme de produits n'a moins de produits.
    pub minimale: bool,
}

/// L'expression simplifiée d'une fonction de `noms.len()` variables, vraie exactement pour
/// les combinaisons de `vrais` (le bit `i` d'une combinaison est la variable `noms[i]`).
///
/// Méthode de Quine-McCluskey : la bibliothèque `quine-mc_cluskey` donne les implicants
/// premiers ; on cherche ensuite, par séparation et évaluation, le plus petit nombre d'entre
/// eux qui couvre tous les 1 (ce que fait la méthode de Petrick, mais avec un budget, car elle
/// peut exploser). Toute somme de produits minimale peut s'écrire avec des implicants premiers
/// (théorème de Quine) : la couverture minimale donne donc le moins de produits possible.
/// Le résultat est vérifié sur toutes les combinaisons ; s'il ne collait pas, la forme
/// développée (un produit par combinaison vraie) est rendue à la place.
pub fn simplifier(noms: &[String], vrais: &[u32]) -> Simplifiee {
    let n = noms.len() as u32;
    let toutes = 1u32 << n;
    if vrais.is_empty() {
        return Simplifiee {
            texte: "0".to_owned(),
            minimale: true,
        };
    }
    if vrais.len() as u32 == toutes {
        return Simplifiee {
            texte: "1".to_owned(),
            minimale: true,
        };
    }
    let mut vrais: Vec<u32> = vrais.to_vec();
    vrais.sort_unstable();
    vrais.dedup();
    let premiers =
        quine_mc_cluskey::essential_minterms(vrais.iter().map(|&m| Term::new(m)).collect())
            .essentials;
    let couvre = |p: &Term, m: u32| p.contains(&Term::new(m));
    let (choix, minimale) = couverture(&premiers, &vrais, couvre);
    let produits: Vec<Bool> = choix.iter().map(|&k| premiers[k].to_bool_expr(n)).collect();
    let expr = if produits.len() == 1 {
        produits.into_iter().next().unwrap_or(Bool::False)
    } else {
        Bool::Or(produits)
    };
    let juste = (0..toutes).all(|i| expr.eval(i) == vrais.binary_search(&i).is_ok());
    if !juste {
        let developpee = Bool::Or(
            vrais
                .iter()
                .map(|&m| Term::new(m).to_bool_expr(n))
                .collect(),
        );
        return Simplifiee {
            texte: ecrire(&developpee, noms, true),
            minimale: false,
        };
    }
    Simplifiee {
        texte: ecrire(&ordonner(expr), noms, true),
        minimale,
    }
}

/// Un produit de littéraux : `(variable, vraie)`, `vraie = false` pour la variable inversée.
pub type Produit = Vec<(u32, bool)>;

/// La somme de produits la plus courte trouvée pour une fonction de `n` variables, vraie
/// exactement pour les combinaisons de `vrais` (même méthode que [`simplifier`]). Aucun produit :
/// la fonction vaut toujours 0 ; un produit vide : elle vaut toujours 1. Vérifiée sur toutes
/// les combinaisons ; sinon, la forme développée (un produit par combinaison vraie).
pub fn somme_de_produits(n: u32, vrais: &[u32]) -> Vec<Produit> {
    let toutes = 1u32 << n;
    let mut vrais: Vec<u32> = vrais.to_vec();
    vrais.sort_unstable();
    vrais.dedup();
    if vrais.is_empty() {
        return Vec::new();
    }
    if vrais.len() as u32 == toutes {
        return vec![Vec::new()];
    }
    let developpee = || -> Vec<Produit> {
        vrais
            .iter()
            .map(|&m| (0..n).map(|b| (b, m >> b & 1 == 1)).collect())
            .collect()
    };
    let premiers =
        quine_mc_cluskey::essential_minterms(vrais.iter().map(|&m| Term::new(m)).collect())
            .essentials;
    let couvre = |p: &Term, m: u32| p.contains(&Term::new(m));
    let (choix, _) = couverture(&premiers, &vrais, couvre);
    let mut produits = Vec::new();
    for &k in &choix {
        let litteraux = match premiers[k].to_bool_expr(n) {
            Bool::True => Vec::new(),
            Bool::And(v) => v,
            seul => vec![seul],
        };
        let mut produit = Vec::new();
        for l in litteraux {
            match l {
                Bool::Term(b) => produit.push((u32::from(b), true)),
                Bool::Not(x) => match *x {
                    Bool::Term(b) => produit.push((u32::from(b), false)),
                    _ => return developpee(),
                },
                _ => return developpee(),
            }
        }
        produits.push(produit);
    }
    let vaut = |m: u32| {
        produits
            .iter()
            .any(|p| p.iter().all(|&(b, v)| (m >> b & 1 == 1) == v))
    };
    if (0..toutes).all(|m| vaut(m) == vrais.binary_search(&m).is_ok()) {
        produits
    } else {
        developpee()
    }
}

/// Le texte seul de [`simplifier`].
pub fn expression(noms: &[String], vrais: &[u32]) -> String {
    simplifier(noms, vrais).texte
}

/// Le plus petit ensemble de `premiers` qui couvre tous les `vrais` : (rangs choisis, vrai si
/// la recherche est allée au bout). Les implicants indispensables d'abord, puis séparation :
/// on prend le 1 le moins couvert et on essaie chaque premier qui le couvre.
fn couverture(
    premiers: &[Term],
    vrais: &[u32],
    couvre: impl Fn(&Term, u32) -> bool,
) -> (Vec<usize>, bool) {
    let par_vrai: Vec<Vec<usize>> = vrais
        .iter()
        .map(|&m| {
            (0..premiers.len())
                .filter(|&k| couvre(&premiers[k], m))
                .collect()
        })
        .collect();
    struct Recherche<'a> {
        par_vrai: &'a [Vec<usize>],
        premiers: &'a [Term],
        vrais: &'a [u32],
        meilleure: Option<Vec<usize>>,
        essais: u32,
    }
    impl Recherche<'_> {
        fn chercher(&mut self, pris: &mut Vec<usize>, couverts: &mut Vec<bool>) {
            self.essais += 1;
            if self.essais > BUDGET_COUVERTURE {
                return;
            }
            if self
                .meilleure
                .as_ref()
                .is_some_and(|b| pris.len() >= b.len())
            {
                return;
            }
            // Le 1 encore à couvrir qui a le moins de candidats.
            let Some(i) = (0..self.vrais.len())
                .filter(|&i| !couverts[i])
                .min_by_key(|&i| self.par_vrai[i].len())
            else {
                self.meilleure = Some(pris.clone());
                return;
            };
            if self
                .meilleure
                .as_ref()
                .is_some_and(|b| pris.len() + 1 >= b.len())
            {
                return;
            }
            for &k in &self.par_vrai[i] {
                let avant = couverts.clone();
                for (j, &m) in self.vrais.iter().enumerate() {
                    if self.premiers[k].contains(&Term::new(m)) {
                        couverts[j] = true;
                    }
                }
                pris.push(k);
                self.chercher(pris, couverts);
                pris.pop();
                *couverts = avant;
            }
        }
    }
    let mut r = Recherche {
        par_vrai: &par_vrai,
        premiers,
        vrais,
        meilleure: None,
        essais: 0,
    };
    r.chercher(&mut Vec::new(), &mut vec![false; vrais.len()]);
    let complete = r.essais <= BUDGET_COUVERTURE;
    let choix = match r.meilleure {
        Some(c) => c,
        // Budget épuisé avant toute couverture (cela n'arrive pas en pratique) : tous les
        // premiers, ce qui couvre forcément.
        None => (0..premiers.len()).collect(),
    };
    (choix, complete)
}

/// Range les produits dans un ordre lisible et stable : d'après leurs variables, dans l'ordre
/// des entrées, la variable avant son inverse.
fn ordonner(e: Bool) -> Bool {
    let cle = |p: &Bool| -> Vec<(u8, bool)> {
        let lits: Vec<&Bool> = match p {
            Bool::And(v) => v.iter().collect(),
            autre => vec![autre],
        };
        lits.iter()
            .filter_map(|l| match l {
                Bool::Term(i) => Some((*i, false)),
                Bool::Not(x) => match **x {
                    Bool::Term(i) => Some((i, true)),
                    _ => None,
                },
                _ => None,
            })
            .collect()
    };
    match e {
        Bool::Or(mut v) => {
            v.sort_by_key(|p| cle(p));
            Bool::Or(v)
        }
        autre => autre,
    }
}

fn ecrire(e: &Bool, noms: &[String], dehors: bool) -> String {
    match e {
        Bool::True => "1".to_owned(),
        Bool::False => "0".to_owned(),
        Bool::Term(i) => noms.get(*i as usize).cloned().unwrap_or_else(|| "?".into()),
        Bool::Not(x) => match **x {
            Bool::Term(_) => format!("¬{}", ecrire(x, noms, false)),
            _ => format!("¬({})", ecrire(x, noms, true)),
        },
        Bool::And(v) => v
            .iter()
            .map(|x| match x {
                Bool::Or(_) => format!("({})", ecrire(x, noms, true)),
                _ => ecrire(x, noms, false),
            })
            .collect::<Vec<_>>()
            .join("·"),
        Bool::Or(v) => {
            let t = v
                .iter()
                .map(|x| ecrire(x, noms, true))
                .collect::<Vec<_>>()
                .join(" + ");
            if dehors {
                t
            } else {
                format!("({t})")
            }
        }
    }
}

/// Les noms des bits d'entrée : `a` pour un fil simple, `a0`, `a1`… pour un bus.
fn noms_des_bits(broches: &[(String, u32)]) -> Vec<String> {
    let mut noms = Vec::new();
    for (nom, largeur) in broches {
        if *largeur == 1 {
            noms.push(nom.clone());
        } else {
            noms.extend((0..*largeur).map(|k| format!("{nom}{k}")));
        }
    }
    noms
}

/// Ce qu'on sait d'un circuit sans mémoire.
#[derive(Clone, Debug, PartialEq)]
pub struct Analyse {
    /// Les interrupteurs : nom et nombre de bits, dans l'ordre des colonnes.
    pub entrees: Vec<(String, u32)>,
    /// Les lampes : nom et nombre de bits.
    pub sorties: Vec<(String, u32)>,
    /// Pour chaque combinaison d'entrées (bit 0 de la combinaison : premier bit de la
    /// première entrée), la valeur de chaque bit de sortie.
    pub lignes: Vec<Vec<Valeur>>,
    /// Pour chaque bit de sortie : son nom et son expression simplifiée, ou pourquoi elle
    /// n'est pas donnée.
    pub expressions: Vec<(String, Result<String, String>)>,
    /// Nombre d'étapes de portes, dans le pire cas, entre une entrée et une sortie.
    pub profondeur: u64,
    /// Nombre de portes NAND, briques comprises.
    pub portes: u64,
}

impl Analyse {
    /// Nombre total de bits d'entrée.
    pub fn bits_d_entree(&self) -> u32 {
        self.entrees.iter().map(|(_, l)| l).sum()
    }

    /// Valeur du bit `bit` de la combinaison `ligne`.
    pub fn bit_d_entree(ligne: usize, bit: u32) -> bool {
        (ligne >> bit) & 1 == 1
    }
}

/// Les expressions de chaque bit de sortie d'une table.
fn expressions_de(
    entrees: &[(String, u32)],
    sorties: &[(String, u32)],
    lignes: &[Vec<Valeur>],
) -> Vec<(String, Result<String, String>)> {
    let bits: u32 = entrees.iter().map(|(_, l)| l).sum();
    let noms = noms_des_bits(entrees);
    noms_des_bits(sorties)
        .into_iter()
        .enumerate()
        .map(|(k, nom)| {
            if bits == 0 {
                return (nom, Err(tr!("Pas d'interrupteur : rien dont dépendre.").to_owned()));
            }
            if bits > MAX_BITS_EXPRESSION {
                return (
                    nom,
                    Err(tr!("{bits} bits d'entrée : trop pour une expression lisible (au plus {MAX_BITS_EXPRESSION}).", bits = bits, MAX_BITS_EXPRESSION = MAX_BITS_EXPRESSION)),
                );
            }
            let mut vrais = Vec::new();
            for (i, l) in lignes.iter().enumerate() {
                match l.get(k) {
                    Some(Valeur::Un) => vrais.push(i as u32),
                    Some(Valeur::Zero) => {}
                    _ => {
                        return (
                            nom,
                            Err(tr!("Elle n'a pas toujours une valeur nette (X ou Z) : un fil est en l'air ou en conflit.").to_owned()),
                        )
                    }
                }
            }
            let s = simplifier(&noms, &vrais);
            let texte = if s.minimale {
                s.texte
            } else {
                tr!("{texte}   (simplifiée, peut-être pas la plus courte)", texte = s.texte)
            };
            (nom, Ok(texte))
        })
        .collect()
}

/// Analyse le plan `plan` du projet : sa table de vérité, puis l'expression de chaque lampe.
/// Échoue, avec une phrase qui dit pourquoi, si le circuit a une mémoire, une horloge, un
/// clavier ou un écran, ou trop d'entrées pour une table lisible.
pub fn analyser(projet: &Projet, plan: &Document) -> Result<Analyse, String> {
    let nom = |id: &str| -> String {
        id.get(1..)
            .and_then(|n| n.parse::<crate::document::Id>().ok())
            .and_then(|n| plan.elements.iter().find(|e| e.id == n))
            .map_or_else(|| id.to_owned(), |e| e.nom.clone())
    };
    if plan
        .elements
        .iter()
        .any(|e| matches!(e.genre, Genre::Horloge | Genre::Clavier | Genre::Ecran))
    {
        return Err(tr!("Ce plan a une horloge, un clavier ou un écran : ce qu'il fait dépend du temps qui passe. La table de vérité ne vaut que pour un circuit sans mémoire ; regarde plutôt le chronogramme.").to_owned());
    }
    if !plan.elements.iter().any(|e| e.genre == Genre::Sortie) {
        return Err(tr!("Pose au moins une lampe : c'est elle qu'on analyse.").to_owned());
    }
    let bits: u32 = plan
        .elements
        .iter()
        .filter(|e| e.genre == Genre::Entree)
        .map(|e| e.broches_s[0].largeur)
        .sum();
    if bits > MAX_BITS_TABLE {
        return Err(tr!("Ce plan a {bits} bits d'entrée, soit {v} combinaisons : trop pour une table lisible (au plus {MAX_BITS_TABLE} bits). Entre dans une de ses briques pour l'analyser seule.", bits = bits, v = 1u64 << bits.min(63), MAX_BITS_TABLE = MAX_BITS_TABLE));
    }
    let mut briques = Briques::default();
    briques.suivre(projet);
    let circuit = Pont::construire(plan, projet, &briques)?.circuit;
    let table = table_du_circuit(circuit).map_err(|e| {
        let texte = e.to_string();
        if texte.contains("boucle") {
            tr!("Ce circuit a une boucle : il garde une mémoire, et ses lampes dépendent du passé, pas seulement des interrupteurs. La table de vérité ne vaut que pour un circuit sans mémoire ; regarde plutôt le chronogramme.").to_owned()
        } else {
            tr!("Ce circuit ne peut pas être analysé : {texte}.", texte = texte)
        }
    })?;
    let entrees: Vec<(String, u32)> = table.entrees.iter().map(|(n, l)| (nom(n), *l)).collect();
    let sorties: Vec<(String, u32)> = table.sorties.iter().map(|(n, l)| (nom(n), *l)).collect();
    let expressions = expressions_de(&entrees, &sorties, &table.lignes);
    Ok(Analyse {
        entrees,
        sorties,
        lignes: table.lignes,
        expressions,
        profondeur: table.chemin_critique,
        portes: projet.nb_portes(plan),
    })
}

/// Un indice de quête.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Indice {
    /// Ce qu'il apporte, en quelques mots.
    pub titre: &'static str,
    /// Le texte.
    pub texte: String,
}

/// Comment s'appelle, pour le joueur, le composant `genre` d'un corrigé.
fn nom_pour_le_joueur(genre: &str) -> String {
    if genre == "NAND" {
        return tr!("porte NAND").to_owned();
    }
    match PARCOURS.iter().find(|f| f.corrige == genre) {
        Some(f) => tr!("« {brique} » (quête « {titre} »)", brique = f.brique, titre = crate::langue::traduire(f.titre)),
        None => genre.to_lowercase().replace('_', " "),
    }
}

fn par_ou_commencer(f: &Fiche) -> String {
    if crate::depannage::est_un_depannage(f.cle) {
        return tr!("Ne démonte rien : le circuit est presque juste. Clique sur les entrées pour essayer chaque cas, ou ouvre la table de vérité (Ctrl+E), et note les cas faux. Puis pars de la lampe qui se trompe et remonte les fils : pour chaque porte, vérifie que sa sortie est bien le NAND de ses deux entrées et que ses entrées viennent du bon endroit. Le fil fautif trouvé, rebranche-le.").to_owned();
    }
    match f.epreuve {
        Epreuve::Combinatoire => tr!("Commence par la table de vérité : pour chaque combinaison d'entrées, que doit valoir chaque lampe ? Une méthode marche toujours : chaque ligne où la lampe vaut 1 devient un ET de ses entrées (inversées quand elles valent 0), et la lampe est le OU de ces ET. On simplifie ensuite. Souvent, une brique déjà gagnée fait la moitié du travail.").to_owned(),
        Epreuve::Sequentielle => tr!("Ce composant se souvient : sa sortie dépend de ce qui s'est passé avant. Il lui faut une mémoire, une boucle ou une bascule gagnée plus tôt. Demande-toi ce qui doit être retenu, et à quel moment (au front montant de l'horloge, le plus souvent) cela doit changer. Le chronogramme montre tout, instant par instant.").to_owned(),
        Epreuve::Ordinateur { .. } | Epreuve::Systeme { .. } => tr!("Un grand assemblage se construit et se vérifie morceau par morceau. Pose d'abord les grandes briques, relie les bus en suivant leurs noms, puis teste chaque brique seule : entre dedans (double-clic), et Ctrl+E l'analyse quand elle n'a pas de mémoire. Le chronogramme montre où le signal s'arrête.").to_owned(),
    }
}

fn briques_du_corrige(f: &'static Fiche) -> Option<String> {
    let c = corriges().composants.get(f.corrige)?;
    let mut compte: Vec<(String, usize)> = Vec::new();
    for i in &c.instances {
        match compte.iter_mut().find(|(g, _)| *g == i.genre) {
            Some((_, n)) => *n += 1,
            None => compte.push((i.genre.clone(), 1)),
        }
    }
    if compte.is_empty() {
        return None;
    }
    let liste = compte
        .iter()
        .map(|(g, n)| {
            let nom = nom_pour_le_joueur(g);
            if *n > 1 {
                format!("{n} × {nom}")
            } else {
                nom
            }
        })
        .collect::<Vec<_>>()
        .join(", ");
    Some(tr!("Le corrigé assemble : {liste}. Ce n'est pas la seule façon de faire, mais ces briques suffisent.", liste = liste))
}

fn expression_attendue(f: &'static Fiche) -> Option<String> {
    if f.epreuve != Epreuve::Combinatoire {
        return None;
    }
    let quete = Quete::de_fiche(f);
    // Compter les bits d'entrée avant de calculer la table : elle double à chaque bit.
    if quete.entrees.iter().map(|b| b.largeur).sum::<u32>() > MAX_BITS_EXPRESSION {
        return None;
    }
    let table = table_de_verite(corriges(), f.corrige).ok()?;
    // Les noms que le joueur voit sur son plan.
    let nom_du_plan = |broche: &str, sens: Sens| -> String {
        let liste = match sens {
            Sens::Entree => &quete.entrees,
            Sens::Sortie => &quete.sorties,
        };
        liste
            .iter()
            .find(|a| a.corrige == broche)
            .map_or_else(|| broche.to_owned(), |a| a.nom.clone())
    };
    let entrees: Vec<(String, u32)> = table
        .entrees
        .iter()
        .map(|(n, l)| (nom_du_plan(n, Sens::Entree), *l))
        .collect();
    let sorties: Vec<(String, u32)> = table
        .sorties
        .iter()
        .map(|(n, l)| (nom_du_plan(n, Sens::Sortie), *l))
        .collect();
    let lignes = expressions_de(&entrees, &sorties, &table.lignes)
        .into_iter()
        .filter_map(|(nom, e)| e.ok().map(|e| format!("{nom} = {e}")))
        .collect::<Vec<_>>();
    if lignes.is_empty() {
        return None;
    }
    Some(tr!("{v}. Lis · comme ET, + comme OU et ¬ comme NON : tu sais fabriquer chacune de ces portes avec des NAND.", v = lignes.join(" ; ")))
}

/// Les indices de la quête `cle`, du plus léger au plus précis.
pub fn indices(cle: &str) -> Vec<Indice> {
    let Some(f) = Quete::fiche(cle) else {
        return Vec::new();
    };
    let mut v = vec![Indice {
        titre: tr!("Par où commencer"),
        texte: par_ou_commencer(f),
    }];
    if let Some(texte) = briques_du_corrige(f) {
        v.push(Indice {
            titre: tr!("Les briques du corrigé"),
            texte,
        });
    }
    if let Some(texte) = expression_attendue(f) {
        v.push(Indice {
            titre: tr!("L'expression attendue"),
            texte,
        });
    }
    v
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::document::{Element, Point};

    fn noms(n: usize) -> Vec<String> {
        ["a", "b", "c", "d", "e", "f"][..n]
            .iter()
            .map(|s| (*s).to_owned())
            .collect()
    }

    #[test]
    fn les_portes_classiques_ont_leur_expression_attendue() {
        // Bit 0 de la combinaison : a ; bit 1 : b.
        assert_eq!(expression(&noms(2), &[3]), "a·b");
        assert_eq!(expression(&noms(2), &[0, 1, 2]), "¬a + ¬b");
        let ou_x = expression(&noms(2), &[1, 2]);
        assert!(ou_x == "a·¬b + ¬a·b" || ou_x == "¬a·b + a·¬b", "{ou_x}");
        assert_eq!(expression(&noms(1), &[0]), "¬a");
        assert_eq!(expression(&noms(2), &[]), "0");
        assert_eq!(expression(&noms(2), &[0, 1, 2, 3]), "1");
        // La majorité : trois produits de deux variables.
        let maj = expression(&noms(3), &[3, 5, 6, 7]);
        assert_eq!(maj.split(" + ").count(), 3, "{maj}");
        assert!(maj.split(" + ").all(|p| p.split('·').count() == 2), "{maj}");
    }

    /// Toute fonction de 1 à 4 variables, et un échantillon de fonctions de 5 et 6 variables :
    /// l'expression rendue, relue, redonne exactement la fonction, et elle n'a jamais plus de
    /// produits que la forme développée.
    #[test]
    fn l_expression_rendue_est_toujours_juste() {
        let lire = |texte: &str, n: u32, i: u32| -> bool {
            if texte == "1" {
                return true;
            }
            if texte == "0" {
                return false;
            }
            texte.split(" + ").any(|produit| {
                produit.split('·').all(|lit| {
                    let (non, var) = match lit.strip_prefix('¬') {
                        Some(v) => (true, v),
                        None => (false, lit),
                    };
                    let k = noms(n as usize).iter().position(|x| x == var).unwrap() as u32;
                    ((i >> k) & 1 == 1) != non
                })
            })
        };
        let essayer = |n: u32, f: u64| {
            let vrais: Vec<u32> = (0..(1u32 << n)).filter(|i| (f >> i) & 1 == 1).collect();
            let e = expression(&noms(n as usize), &vrais);
            for i in 0..(1u32 << n) {
                assert_eq!(lire(&e, n, i), vrais.contains(&i), "n={n} f={f:#x} : {e}");
            }
            if !vrais.is_empty() && vrais.len() < (1 << n) {
                assert!(e.split(" + ").count() <= vrais.len(), "{e}");
            }
        };
        for n in 1..=3u32 {
            for f in 0..(1u64 << (1 << n)) {
                essayer(n, f);
            }
        }
        let mut x = 0x9E37_79B9_7F4A_7C15u64;
        for n in [4u32, 5, 6] {
            for _ in 0..60 {
                x ^= x << 13;
                x ^= x >> 7;
                x ^= x << 17;
                let masque = if n == 6 {
                    u64::MAX
                } else {
                    (1u64 << (1 << n)) - 1
                };
                essayer(n, x & masque);
            }
        }
    }

    /// Pour chacune des 256 fonctions de 3 variables, aucune somme de produits n'a moins de
    /// produits que l'expression rendue : on essaie toutes les familles de 1, 2, 3… produits
    /// (un produit fixe chaque variable à 0, à 1, ou ne la contient pas : 27 produits).
    #[test]
    fn l_expression_a_le_moins_de_produits_possible_pour_trois_variables() {
        let produits: Vec<u8> = (0..27u32)
            .map(|p| {
                // Chiffre k en base 3 : 0 absente, 1 vaut 1, 2 vaut 0.
                let mut couvre = 0u8;
                for i in 0..8u32 {
                    let mut q = p;
                    let mut ok = true;
                    for k in 0..3 {
                        let bit = (i >> k) & 1;
                        match q % 3 {
                            1 if bit == 0 => ok = false,
                            2 if bit == 1 => ok = false,
                            _ => {}
                        }
                        q /= 3;
                    }
                    if ok {
                        couvre |= 1 << i;
                    }
                }
                couvre
            })
            .collect();
        for f in 1..255u32 {
            let f = f as u8;
            // Les produits qui ne débordent pas de la fonction.
            let dedans: Vec<u8> = produits.iter().copied().filter(|p| p & !f == 0).collect();
            let mut minimum = 0;
            'taille: for taille in 1..=4usize {
                let mut pile = vec![(0usize, 0u8, 0usize)];
                while let Some((debut, union, pris)) = pile.pop() {
                    if pris == taille {
                        if union == f {
                            minimum = taille;
                            break 'taille;
                        }
                        continue;
                    }
                    for (j, p) in dedans.iter().enumerate().skip(debut) {
                        pile.push((j + 1, union | p, pris + 1));
                    }
                }
            }
            assert!(minimum > 0, "{f:#010b}");
            let vrais: Vec<u32> = (0..8).filter(|i| (f >> i) & 1 == 1).collect();
            let e = expression(&noms(3), &vrais);
            assert_eq!(e.split(" + ").count(), minimum, "{f:#010b} : {e}");
        }
    }

    #[test]
    fn le_pire_cas_a_six_entrees_reste_rapide() {
        // La parité de 6 bits n'a aucun regroupement possible : 32 produits. Et une fonction
        // « en damier » décalée donne beaucoup d'implicants premiers.
        let debut = std::time::Instant::now();
        let parite: Vec<u32> = (0..64u32).filter(|i| i.count_ones() % 2 == 1).collect();
        let e = expression(&noms(6), &parite);
        assert_eq!(e.split(" + ").count(), 32);
        let x: Vec<u32> = (0..64u32).filter(|i| (i * 7 + 3) % 5 < 3).collect();
        let s = simplifier(&noms(6), &x);
        assert!(!s.texte.is_empty());
        // Toutes les fonctions de 6 variables qui ont 4 à 60 combinaisons vraies, prises en
        // grand nombre au hasard : jamais plus d'une seconde chacune.
        let mut graine = 7u64;
        for _ in 0..200 {
            graine ^= graine << 13;
            graine ^= graine >> 7;
            graine ^= graine << 17;
            let vrais: Vec<u32> = (0..64u32).filter(|i| (graine >> i) & 1 == 1).collect();
            let t = std::time::Instant::now();
            let s = simplifier(&noms(6), &vrais);
            assert!(
                t.elapsed().as_secs_f32() < 1.0,
                "{vrais:?} : {:?}",
                t.elapsed()
            );
            // La recherche va au bout : l'expression est bien la plus courte.
            assert!(s.minimale, "{vrais:?}");
        }
        assert!(debut.elapsed().as_secs() < 30, "{:?}", debut.elapsed());
    }

    #[test]
    fn le_plan_du_demi_additionneur_s_analyse() {
        let doc = Document::demi_additionneur();
        let projet = Projet::nouveau("essai", doc.clone());
        let an = analyser(&projet, &doc).expect("sans mémoire");
        assert_eq!(an.bits_d_entree(), 2);
        assert_eq!(an.lignes.len(), 4);
        let s = an.expressions.iter().find(|(n, _)| n == "s").unwrap();
        let s = s.1.as_ref().unwrap();
        assert!(s == "a·¬b + ¬a·b" || s == "¬a·b + a·¬b", "{s}");
        let c = an.expressions.iter().find(|(n, _)| n == "c").unwrap();
        assert_eq!(c.1.as_deref(), Ok("a·b"));
        assert_eq!(an.portes, 5);
    }

    #[test]
    fn un_circuit_a_memoire_ou_a_horloge_est_refuse_avec_une_raison() {
        let mut doc = Document::default();
        doc.poser(Element::horloge("h", 4), Point::nouveau(0.0, 0.0));
        doc.poser(Element::sortie("s", "", 1), Point::nouveau(100.0, 0.0));
        let projet = Projet::nouveau("essai", doc.clone());
        let e = analyser(&projet, &doc).unwrap_err();
        assert!(e.contains("horloge"), "{e}");
        let vide = Document::default();
        assert!(analyser(&projet, &vide).unwrap_err().contains("lampe"));
    }

    #[test]
    fn chaque_quete_a_au_moins_deux_indices_et_les_portes_ont_leur_expression() {
        for f in PARCOURS {
            let v = indices(f.cle);
            assert!(!v.is_empty(), "{}", f.cle);
            if f.epreuve == Epreuve::Combinatoire || f.epreuve == Epreuve::Sequentielle {
                assert!(v.len() >= 2, "{} : {v:?}", f.cle);
            }
        }
        let et = indices("et");
        let derniere = et.last().unwrap();
        assert_eq!(derniere.titre, "L'expression attendue");
        assert!(derniere.texte.starts_with("s = a·b"), "{}", derniere.texte);
    }
}
