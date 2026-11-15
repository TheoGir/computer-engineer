//! L'éditeur d'automates : on décrit des états et leurs transitions en texte, et
//! l'application fabrique la brique, en portes NAND et en bascules D, qui fait la même chose.
//!
//! ```text
//! # Un feu pour piétons
//! entrées : bouton
//! sorties : vert, rouge
//! état attente : vert
//!   si bouton -> passage
//! état passage : rouge
//!   -> attente
//! ```
//!
//! - Le premier état est celui du départ : la brique y revient quand `raz` vaut 1.
//! - Chaque état dit quelles sorties valent 1 pendant qu'on y est (les autres valent 0).
//! - Au front montant de `clk`, la première transition dont la condition est vraie est
//!   suivie ; si aucune ne l'est, on reste dans l'état. Une condition : `bouton`, `non bouton`,
//!   `bouton = 0`, plusieurs reliées par `et`. `-> cible` seul : toujours.
//!
//! La brique a les entrées de l'automate, puis `raz` et `clk`, et les sorties de l'automate,
//! puis `etat` (le numéro de l'état, en binaire). Les états sont numérotés dans l'ordre du
//! texte ; la logique de l'état suivant et des sorties est simplifiée par la méthode de
//! Quine-McCluskey ([`crate::logique::somme_de_produits`]).

use ce_circuit::{Bibliotheque, Composant};

use crate::brique::Modele;
use crate::logique::{somme_de_produits, Produit};

/// Au plus tant de bits entre l'état et les entrées : la table de l'état suivant a 2^n lignes.
pub const MAX_BITS: u32 = 8;

/// Une transition : la condition (entrée, valeur attendue), et l'état d'arrivée.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Transition {
    /// Toutes ces entrées doivent avoir ces valeurs ; vide : toujours.
    pub condition: Vec<(usize, bool)>,
    /// Le rang de l'état d'arrivée.
    pub cible: usize,
}

/// Un état.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Etat {
    /// Son nom.
    pub nom: String,
    /// La valeur de chaque sortie pendant qu'on y est.
    pub sorties: Vec<bool>,
    /// Ses transitions, dans l'ordre où elles sont essayées.
    pub transitions: Vec<Transition>,
}

/// Un automate lu dans le texte.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Automate {
    /// Les noms des entrées.
    pub entrees: Vec<String>,
    /// Les noms des sorties.
    pub sorties: Vec<String>,
    /// Les états ; le premier est celui du départ.
    pub etats: Vec<Etat>,
}

/// Les mots d'une liste, séparés par des virgules ou des espaces.
fn liste(texte: &str) -> Vec<String> {
    texte
        .split(|c: char| c == ',' || c.is_whitespace())
        .filter(|m| !m.is_empty())
        .map(str::to_owned)
        .collect()
}

/// Sans accents ni majuscules, pour reconnaître les mots-clés.
fn plier(mot: &str) -> String {
    crate::fenetres::plier(mot)
}

/// Un nom de broche ou d'état acceptable : lettres, chiffres et `_`, sans commencer par un
/// chiffre.
fn nom_valable(nom: &str) -> bool {
    let mut c = nom.chars();
    c.next().is_some_and(|p| p.is_alphabetic() || p == '_')
        && nom.chars().all(|x| x.is_alphanumeric() || x == '_')
}

impl Automate {
    /// Lit un automate. Les erreurs disent la ligne.
    pub fn lire(texte: &str) -> Result<Automate, String> {
        let mut entrees: Vec<String> = Vec::new();
        let mut sorties: Vec<String> = Vec::new();
        // Les transitions sont relues une fois tous les états connus.
        let mut brutes: Vec<(usize, usize, String)> = Vec::new();
        let mut etats: Vec<(String, Vec<String>)> = Vec::new();
        for (i, brut) in texte.lines().enumerate() {
            let numero = i + 1;
            let ligne = brut.split('#').next().unwrap_or("").trim();
            if ligne.is_empty() {
                continue;
            }
            let (tete, reste) = ligne.split_once(':').unwrap_or((ligne, ""));
            let tete_pliee = plier(tete.trim());
            if tete_pliee == "entrees" {
                entrees.extend(liste(reste));
            } else if tete_pliee == "sorties" {
                sorties.extend(liste(reste));
            } else if let Some(nom) = tete_pliee.strip_prefix("etat ") {
                let nom = tete.trim()[tete.trim().len() - nom.len()..]
                    .trim()
                    .to_owned();
                if !nom_valable(&nom) {
                    return Err(tr!("ligne {numero} : « {nom} » n'est pas un nom d'état (lettres, chiffres, _)", numero = numero, nom = nom));
                }
                if etats.iter().any(|(n, _)| *n == nom) {
                    return Err(tr!("ligne {numero} : l'état « {nom} » existe déjà", numero = numero, nom = nom));
                }
                // Les sorties à 1 : « vert », ou « vert = 1 » ; « = 0 » est accepté.
                let mut a_un = Vec::new();
                for morceau in reste.split(',') {
                    let morceau = morceau.trim();
                    if morceau.is_empty() {
                        continue;
                    }
                    let (s, v) = match morceau.split_once('=') {
                        Some((s, v)) => (s.trim(), v.trim()),
                        None => (morceau, "1"),
                    };
                    match v {
                        "1" => a_un.push(s.to_owned()),
                        "0" => {}
                        _ => {
                            return Err(tr!("ligne {numero} : « {morceau} » : une sortie vaut 0 ou 1", numero = numero, morceau = morceau))
                        }
                    }
                }
                etats.push((nom, a_un));
            } else if ligne.contains("->") || ligne.contains('→') {
                let Some(rang) = etats.len().checked_sub(1) else {
                    return Err(tr!("ligne {numero} : une transition vient après son état (« état nom : … »)", numero = numero));
                };
                brutes.push((numero, rang, ligne.to_owned()));
            } else {
                return Err(tr!("ligne {numero} : je ne comprends pas « {ligne} ». Une ligne commence par « entrées : », « sorties : », « état nom : », ou c'est une transition « si condition -> état ».", numero = numero, ligne = ligne));
            }
        }
        if etats.is_empty() {
            return Err(tr!("Écris au moins un état : « état nom : sorties à 1 ».").to_owned());
        }
        for nom in entrees.iter().chain(&sorties) {
            if !nom_valable(nom) {
                return Err(tr!("« {nom} » n'est pas un nom de broche (lettres, chiffres, _)", nom = nom));
            }
            if ["raz", "clk", "etat"].contains(&nom.as_str()) {
                return Err(tr!("« {nom} » est réservé : la brique a déjà raz, clk et etat", nom = nom));
            }
        }
        let tous: Vec<&String> = entrees.iter().chain(&sorties).collect();
        for (k, nom) in tous.iter().enumerate() {
            if tous[..k].contains(nom) {
                return Err(tr!("« {nom} » est nommé deux fois", nom = nom));
            }
        }
        let mut automate = Automate {
            sorties: sorties.clone(),
            entrees,
            etats: Vec::new(),
        };
        for (nom, a_un) in &etats {
            for s in a_un {
                if !sorties.contains(s) {
                    return Err(tr!("l'état « {nom} » allume « {s} », qui n'est pas dans « sorties : »", nom = nom, s = s));
                }
            }
            automate.etats.push(Etat {
                nom: nom.clone(),
                sorties: sorties.iter().map(|s| a_un.contains(s)).collect(),
                transitions: Vec::new(),
            });
        }
        for (numero, rang, ligne) in brutes {
            let t = automate.transition(&ligne, numero)?;
            automate.etats[rang].transitions.push(t);
        }
        let bits = automate.bits_d_etat() + automate.entrees.len() as u32;
        if bits > MAX_BITS {
            return Err(tr!("{len} états et {len2} entrées font {bits} bits : au plus {MAX_BITS}. Découpe l'automate en plusieurs briques.", bits = bits, len = automate.etats.len(), len2 = automate.entrees.len(), MAX_BITS = MAX_BITS));
        }
        Ok(automate)
    }

    /// Lit une transition : `si a et non b -> cible`, ou `-> cible`.
    fn transition(&self, ligne: &str, numero: usize) -> Result<Transition, String> {
        let (gauche, cible) = ligne
            .split_once("->")
            .or_else(|| ligne.split_once('→'))
            .unwrap_or((ligne, ""));
        let cible = cible.trim();
        let rang = self
            .etats
            .iter()
            .position(|e| e.nom == cible)
            .ok_or_else(|| tr!("ligne {numero} : l'état « {cible} » n'existe pas", numero = numero, cible = cible))?;
        let gauche = gauche.trim();
        let condition_texte = match plier(gauche).as_str() {
            "" => "",
            "si" => "",
            g if g.starts_with("si ") => gauche[2..].trim(),
            _ => {
                return Err(tr!("ligne {numero} : une condition commence par « si » (« si a et non b -> {cible} »)", numero = numero, cible = cible))
            }
        };
        let mut condition = Vec::new();
        for terme in condition_texte
            .split(',')
            .flat_map(|m| m.split(" et "))
            .map(str::trim)
            .filter(|t| !t.is_empty())
        {
            let (nom, valeur) = if let Some((n, v)) = terme.split_once('=') {
                let v = match v.trim() {
                    "1" => true,
                    "0" => false,
                    autre => {
                        return Err(tr!("ligne {numero} : « {autre} » : une entrée vaut 0 ou 1", numero = numero, autre = autre))
                    }
                };
                (n.trim(), v)
            } else if let Some(n) = terme.strip_prefix("non ") {
                (n.trim(), false)
            } else if let Some(n) = terme.strip_prefix('!') {
                (n.trim(), false)
            } else {
                (terme, true)
            };
            let k = self.entrees.iter().position(|e| e == nom).ok_or_else(|| {
                tr!("ligne {numero} : « {nom} » n'est pas dans « entrées : »", numero = numero, nom = nom)
            })?;
            condition.push((k, valeur));
        }
        Ok(Transition {
            condition,
            cible: rang,
        })
    }

    /// Le nombre de bits qui numérotent les états (au moins 1).
    pub fn bits_d_etat(&self) -> u32 {
        let n = self.etats.len().max(2) as u32;
        u32::BITS - (n - 1).leading_zeros()
    }

    /// L'état qui suit `etat` quand les entrées valent `entrees` (bit `k` = entrée `k`). Un
    /// numéro qui n'est pas un état ramène au départ.
    pub fn suivant(&self, etat: usize, entrees: u32) -> usize {
        let Some(e) = self.etats.get(etat) else {
            return 0;
        };
        e.transitions
            .iter()
            .find(|t| {
                t.condition
                    .iter()
                    .all(|&(k, v)| (entrees >> k & 1 == 1) == v)
            })
            .map_or(etat, |t| t.cible)
    }

    /// Les sommes de produits de chaque bit de l'état suivant (variables : les bits d'état,
    /// puis les entrées), et de chaque sortie (variables : les bits d'état).
    fn logique(&self) -> (Vec<Vec<Produit>>, Vec<Vec<Produit>>) {
        let k = self.bits_d_etat();
        let n = k + self.entrees.len() as u32;
        let suivants: Vec<Vec<Produit>> = (0..k)
            .map(|b| {
                let vrais: Vec<u32> = (0..1u32 << n)
                    .filter(|&m| {
                        let etat = (m & ((1 << k) - 1)) as usize;
                        self.suivant(etat, m >> k) >> b & 1 == 1
                    })
                    .collect();
                somme_de_produits(n, &vrais)
            })
            .collect();
        let sorties: Vec<Vec<Produit>> = (0..self.sorties.len())
            .map(|j| {
                let vrais: Vec<u32> = (0..1u32 << k)
                    .filter(|&m| self.etats.get(m as usize).is_some_and(|e| e.sorties[j]))
                    .collect();
                somme_de_produits(k, &vrais)
            })
            .collect();
        (suivants, sorties)
    }

    /// Le composant qui réalise l'automate, en NAND et en bascules D (`BASCULE_D`).
    pub fn composant(&self) -> Composant {
        let k = self.bits_d_etat();
        let mut c = Fabrique::default();
        let description = tr!("automate de {len} état{v} : {v2}", len = self.etats.len(), v = if self.etats.len() > 1 { "s" } else { "" }, v2 = self.etats
                .iter()
                .map(|e| e.nom.as_str())
                .collect::<Vec<_>>()
                .join(", "));
        let mut comp = Composant::nouveau(&description);
        for e in &self.entrees {
            comp = comp.entree(e, 1);
        }
        comp = comp.entree("raz", 1).entree("clk", 1);
        for s in &self.sorties {
            comp = comp.sortie(s, 1);
        }
        comp = comp.sortie("etat", k);
        c.c = Some(comp);
        // Les variables : bits d'état (sorties des bascules), puis entrées.
        let mut variables: Vec<String> = (0..k).map(|b| format!("b{b}.q")).collect();
        variables.extend(self.entrees.iter().cloned());
        let (suivants, sorties) = self.logique();
        let nraz = c.non("raz");
        for (b, somme) in suivants.iter().enumerate() {
            c.inst(&format!("b{b}"), "BASCULE_D");
            c.fil(&["clk", &format!("b{b}.clk")]);
            let f = c.somme(somme, &variables, "raz");
            // Au front de clk avec raz = 1, retour à l'état 0.
            let d = c.et(&f, &nraz);
            c.fil(&[d.as_str(), &format!("b{b}.d")]);
            c.fil(&[&format!("b{b}.q"), &format!("etat[{b}]")]);
        }
        for (j, somme) in sorties.iter().enumerate() {
            let s = c.somme(somme, &variables, "raz");
            c.fil(&[s.as_str(), self.sorties[j].as_str()]);
        }
        c.c.take().unwrap_or_else(|| Composant::nouveau(""))
    }

    /// Les briques de l'automate, sous le nom `nom` : la sienne en dernier, précédée de
    /// celles qu'elle utilise (la bascule D et ce qui la compose).
    pub fn briques(&self, nom: &str) -> Result<Vec<Modele>, String> {
        let mut bib: Bibliotheque = crate::quete::corriges().clone();
        bib.ajouter(nom, self.composant());
        crate::importer::importer(&bib, nom)
    }
}

/// Construit un composant porte par porte, avec des noms d'instances fabriqués.
#[derive(Default)]
struct Fabrique {
    c: Option<Composant>,
    n: u32,
}

impl Fabrique {
    fn changer(&mut self, f: impl FnOnce(Composant) -> Composant) {
        let c = self.c.take().unwrap_or_else(|| Composant::nouveau(""));
        self.c = Some(f(c));
    }

    fn inst(&mut self, nom: &str, genre: &str) {
        self.changer(|c| c.inst(nom, genre));
    }

    fn fil(&mut self, points: &[&str]) {
        self.changer(|c| c.fil(points));
    }

    fn nand(&mut self, a: &str, b: &str) -> String {
        self.n += 1;
        let nom = format!("n{}", self.n);
        self.inst(&nom, "NAND");
        self.fil(&[a, &format!("{nom}.a")]);
        self.fil(&[b, &format!("{nom}.b")]);
        format!("{nom}.s")
    }

    fn non(&mut self, a: &str) -> String {
        self.nand(a, a)
    }

    fn et(&mut self, a: &str, b: &str) -> String {
        let n = self.nand(a, b);
        self.non(&n)
    }

    /// Une somme de produits ; `x` sert à fabriquer 0 ou 1 quand la fonction est constante.
    fn somme(&mut self, produits: &[Produit], variables: &[String], x: &str) -> String {
        if produits.is_empty() {
            // 0 = NON(NAND(x, NON x)).
            let nx = self.non(x);
            let un = self.nand(x, &nx);
            return self.non(&un);
        }
        // NAND-NAND : chaque produit donne NON(produit), puis le NAND de tous donne la somme.
        let mut inverses = Vec::new();
        for p in produits {
            if p.is_empty() {
                // Un produit vide vaut 1 : la somme aussi.
                let nx = self.non(x);
                return self.nand(x, &nx);
            }
            let litteraux: Vec<String> = p
                .iter()
                .map(|&(b, v)| {
                    let nom = &variables[b as usize];
                    if v {
                        nom.clone()
                    } else {
                        self.non(nom)
                    }
                })
                .collect();
            let inverse = match litteraux.as_slice() {
                [seul] => self.non(seul),
                _ => {
                    let mut acc = litteraux[0].clone();
                    for l in &litteraux[1..litteraux.len() - 1] {
                        acc = self.et(&acc, l);
                    }
                    self.nand(&acc, &litteraux[litteraux.len() - 1])
                }
            };
            inverses.push(inverse);
        }
        // NAND de tous les NON(produit) = OU des produits : on les réunit deux à deux par OU.
        let mut sommes: Vec<String> = inverses.iter().map(|i| self.non(i)).collect();
        while sommes.len() > 1 {
            let b = sommes.pop().unwrap_or_default();
            let a = sommes.pop().unwrap_or_default();
            let (na, nb) = (self.non(&a), self.non(&b));
            sommes.push(self.nand(&na, &nb));
        }
        sommes.pop().unwrap_or_default()
    }
}

/// Un automate tout prêt, pour commencer.
pub const EXEMPLE: &str = "# Un feu pour piétons : vert pour les voitures, jusqu'à ce qu'on appuie.
entrées : bouton
sorties : voitures, pietons
état attente : voitures
  si bouton -> orange
état orange :
  -> traversee
état traversee : pietons
  si non bouton -> attente
";

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn l_exemple_se_lit() {
        let a = Automate::lire(EXEMPLE).unwrap();
        assert_eq!(a.entrees, ["bouton"]);
        assert_eq!(a.sorties, ["voitures", "pietons"]);
        assert_eq!(a.etats.len(), 3);
        assert_eq!(a.bits_d_etat(), 2);
        assert_eq!(a.etats[0].sorties, [true, false]);
        assert_eq!(a.suivant(0, 0), 0);
        assert_eq!(a.suivant(0, 1), 1);
        assert_eq!(a.suivant(1, 0), 2);
        assert_eq!(a.suivant(2, 1), 2);
        assert_eq!(a.suivant(2, 0), 0);
        assert_eq!(a.suivant(3, 1), 0, "un numéro sans état ramène au départ");
    }

    #[test]
    fn les_erreurs_disent_quoi() {
        let e = |t: &str| Automate::lire(t).unwrap_err();
        assert!(e("").contains("au moins un état"));
        assert!(e("état a :\n  -> b").contains("« b » n'existe pas"));
        assert!(e("-> a\nétat a :").contains("ligne 1"));
        assert!(e("état a : x").contains("« x »"));
        assert!(e("entrées : clk\nétat a :").contains("réservé"));
        assert!(e("état a :\n si z -> a").contains("« z »"));
        assert!(e("état a :\nétat a :").contains("existe déjà"));
        assert!(e("bonjour").contains("ligne 1"));
        assert!(e("entrées : a b c d e f g\nétat x :\nétat y :\nétat z :").contains("bits"));
    }

    /// La brique fabriquée suit l'automate, front après front, sur une longue suite
    /// d'entrées tirées au hasard (graine fixe).
    #[test]
    fn la_brique_fait_ce_que_dit_l_automate() {
        let textes = [
            EXEMPLE,
            "entrées : e\nsorties : vu\nétat rien :\n si e -> un\nétat un :\n si non e -> un_zero\nétat un_zero :\n si e -> vu\n si non e -> rien\nétat vu : vu\n si e -> un\n si non e -> un_zero\n",
            "sorties : s\nétat seul : s\n",
            "entrées : a, b\nsorties : x\nétat p :\n si a et b -> q\nétat q : x = 1\n si a = 0 -> p\n",
        ];
        for texte in textes {
            let a = Automate::lire(texte).unwrap();
            let mut bib = crate::quete::corriges().clone();
            bib.ajouter("AUTO", a.composant());
            let mut banc = ce_circuit::Banc::nouveau(&bib, "AUTO").unwrap();
            let mut hasard: u64 = 0x2545_f491_4f6c_dd1d;
            let mut etat = 0usize;
            let ne = a.entrees.len() as u32;
            // Départ : raz pendant un front.
            for (raz, clk) in [(1, 0), (1, 1), (0, 1)] {
                banc.ecrire("raz", raz).unwrap();
                banc.ecrire("clk", clk).unwrap();
                banc.stabiliser(10_000);
            }
            for pas in 0..200 {
                hasard ^= hasard << 13;
                hasard ^= hasard >> 7;
                hasard ^= hasard << 17;
                let entrees = (hasard & ((1 << ne) - 1)) as u32;
                for (k, nom) in a.entrees.iter().enumerate() {
                    banc.ecrire(nom, u64::from(entrees >> k & 1)).unwrap();
                }
                banc.ecrire("clk", 0).unwrap();
                banc.stabiliser(10_000);
                banc.ecrire("clk", 1).unwrap();
                banc.stabiliser(10_000);
                etat = a.suivant(etat, entrees);
                assert_eq!(
                    banc.lire("etat").unwrap(),
                    Some(etat as u64),
                    "{texte}\npas {pas}"
                );
                for (j, s) in a.sorties.iter().enumerate() {
                    let attendue = u64::from(a.etats[etat].sorties[j]);
                    assert_eq!(banc.lire(s).unwrap(), Some(attendue), "{s}, pas {pas}");
                }
            }
            // raz ramène au départ.
            banc.ecrire("raz", 1).unwrap();
            banc.ecrire("clk", 0).unwrap();
            banc.stabiliser(10_000);
            banc.ecrire("clk", 1).unwrap();
            banc.stabiliser(10_000);
            assert_eq!(banc.lire("etat").unwrap(), Some(0));
            // Et la brique se pose dans l'application.
            let briques = a.briques("AUTO").unwrap();
            assert_eq!(briques.last().map(|m| m.nom.as_str()), Some("AUTO"));
        }
    }
}
