//! La documentation : une page par pièce de base, par notion et par brique de la bibliothèque.
//!
//! Trois sortes de pages :
//! - les **pièces de base** de la palette (NAND, interrupteur, horloge…), écrites à la main ;
//! - les **notions** qui reviennent partout (les quatre valeurs d'un fil, le temps, les bus,
//!   les bascules, l'ordinateur…), écrites à la main ;
//! - les **briques** : une page par composant corrigé de la bibliothèque. Tout ce qui s'y
//!   calcule sort du corrigé lui-même : ses broches, les briques qu'il assemble, son nombre de
//!   portes NAND, son temps de réponse, sa table de vérité (simulée ligne à ligne) ou son
//!   chronogramme (simulé front après front). Les textes viennent de la fiche « Comprendre »
//!   de sa quête ([`crate::savoir`]).
//!
//! Rien ici ne dépend d'egui : la fenêtre est dans [`crate::fenetre_doc`].

use std::collections::BTreeMap;
use std::sync::{Arc, Mutex, OnceLock};

use ce_circuit::minimum::Minimum;
use ce_circuit::{Banc, Circuit, Sens};

use crate::quete::{corriges, Fiche, PARCOURS};
use crate::savoir::CHAPITRES;

/// Un grand nombre lisible : 131 072 en français (espace insécable), 131,072 en anglais.
pub fn milliers(n: u64) -> String {
    let chiffres = n.to_string();
    let separateur = if crate::langue::en_francais() {
        '\u{a0}'
    } else {
        ','
    };
    let mut sortie = String::new();
    for (i, c) in chiffres.chars().enumerate() {
        if i > 0 && (chiffres.len() - i).is_multiple_of(3) {
            sortie.push(separateur);
        }
        sortie.push(c);
    }
    sortie
}

/// Une page de la documentation.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Page {
    /// La page d'entrée : le sommaire en grand.
    Sommaire,
    /// Une pièce de base ou une notion, par sa clé dans [`NOTIONS`].
    Notion(&'static str),
    /// Une brique de la bibliothèque, par le nom de son corrigé.
    Brique(String),
}

/// La famille d'une page écrite à la main.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Famille {
    /// Une pièce de la palette.
    Piece,
    /// Une notion transverse.
    Notion,
}

/// Une page écrite à la main.
#[derive(Clone, Copy, Debug)]
pub struct Notion {
    /// Sa clé, fixe.
    pub cle: &'static str,
    /// Pièce de base ou notion.
    pub famille: Famille,
    /// Son titre.
    pub titre: &'static str,
    /// Une phrase qui la résume.
    pub resume: &'static str,
    /// Son texte : des paragraphes, `# ` pour un intertitre, `- ` pour une puce, `| ` pour une
    /// ligne de tableau (cases séparées par ` | `, la première ligne est l'en-tête).
    pub texte: &'static str,
    /// Les pages à lire ensuite : clés de notions, ou noms de briques.
    pub voir: &'static [&'static str],
    /// Les quêtes où l'on s'en sert, par leur clé.
    pub quetes: &'static [&'static str],
}

/// La notion de cette clé.
pub fn notion(cle: &str) -> Option<&'static Notion> {
    NOTIONS.iter().find(|n| n.cle == cle)
}

/// La page désignée par un lien de [`Notion::voir`] : une notion si la clé en est une, une
/// brique sinon.
pub fn lien(cle: &'static str) -> Page {
    match notion(cle) {
        Some(n) => Page::Notion(n.cle),
        None => Page::Brique(cle.to_owned()),
    }
}

/// Le titre d'une page.
pub fn titre(page: &Page) -> String {
    match page {
        Page::Sommaire => tr!("Documentation").to_owned(),
        Page::Notion(cle) => notion(cle)
            .map(|n| crate::langue::traduire(n.titre).to_owned())
            .unwrap_or_default(),
        Page::Brique(nom) => match quete_de(nom) {
            Some(f) => crate::langue::traduire(f.titre).to_owned(),
            None => crate::langue::nom_affiche(nom).into_owned(),
        },
    }
}

/// La quête qui fait construire cette brique (celle dont c'est le corrigé, hors dépannage).
pub fn quete_de(nom: &str) -> Option<&'static Fiche> {
    PARCOURS
        .iter()
        .find(|f| f.corrige == nom && f.chapitre != "Dépannage")
}

/// Toutes les quêtes qui portent sur cette brique : celle qui la fait construire, et les
/// dépannages du même circuit.
pub fn quetes_de(nom: &str) -> Vec<&'static Fiche> {
    PARCOURS.iter().filter(|f| f.corrige == nom).collect()
}

/// Les briques qui ne se documentent pas seules : l'ordinateur et sa mémoire morte dépendent
/// du programme, ils ont leur page dans les notions.
const A_PART: [&str; 2] = ["ORDINATEUR", "MEMOIRE_MORTE"];

/// Vrai si `nom` est une brique documentée.
pub fn est_documentee(nom: &str) -> bool {
    !A_PART.contains(&nom) && corriges().composants.contains_key(nom)
}

/// Une partie du sommaire : un titre et ses pages.
#[derive(Clone, Debug)]
pub struct Partie {
    /// Le titre de la partie.
    pub titre: String,
    /// Ses pages, dans l'ordre de lecture.
    pub pages: Vec<Page>,
}

/// Le sommaire : les pièces de base, les notions, puis les briques chapitre par chapitre du
/// parcours, et enfin les briques intermédiaires qu'aucune quête ne fait construire seules.
pub fn sommaire() -> Vec<Partie> {
    let mut parties = vec![
        Partie {
            titre: tr!("Les pièces de base").to_owned(),
            pages: NOTIONS
                .iter()
                .filter(|n| n.famille == Famille::Piece)
                .map(|n| Page::Notion(n.cle))
                .collect(),
        },
        Partie {
            titre: tr!("Les notions").to_owned(),
            pages: NOTIONS
                .iter()
                .filter(|n| n.famille == Famille::Notion)
                .map(|n| Page::Notion(n.cle))
                .collect(),
        },
    ];
    let mut vues: Vec<&str> = Vec::new();
    for chapitre in CHAPITRES {
        let mut pages = Vec::new();
        for f in PARCOURS.iter().filter(|f| f.chapitre == chapitre.nom) {
            if f.corrige.is_empty() || vues.contains(&f.corrige) || !est_documentee(f.corrige) {
                continue;
            }
            vues.push(f.corrige);
            pages.push(Page::Brique(f.corrige.to_owned()));
        }
        if !pages.is_empty() {
            parties.push(Partie {
                titre: crate::langue::traduire(chapitre.nom).to_owned(),
                pages,
            });
        }
    }
    let autres: Vec<Page> = corriges()
        .composants
        .keys()
        .filter(|n| est_documentee(n) && !vues.contains(&n.as_str()))
        .map(|n| Page::Brique(n.clone()))
        .collect();
    if !autres.is_empty() {
        parties.push(Partie {
            titre: tr!("Briques intermédiaires").to_owned(),
            pages: autres,
        });
    }
    parties
}

/// Toutes les pages, dans l'ordre du sommaire.
pub fn toutes_les_pages() -> Vec<Page> {
    sommaire().into_iter().flat_map(|p| p.pages).collect()
}

/// Le texte où l'on cherche, pour une page : son titre, son résumé et son contenu écrit.
fn texte_cherchable(page: &Page) -> String {
    match page {
        Page::Sommaire => String::new(),
        Page::Notion(cle) => notion(cle)
            .map(|n| {
                format!(
                    "{}\n{}\n{}",
                    crate::langue::traduire(n.titre),
                    crate::langue::traduire(n.resume),
                    crate::langue::traduire(n.texte)
                )
            })
            .unwrap_or_default(),
        Page::Brique(nom) => {
            let mut t = format!(
                "{}\n{nom}\n{}\n",
                titre(page),
                crate::langue::nom_affiche(nom)
            );
            if let Some(c) = corriges().composants.get(nom) {
                t.push_str(&crate::langue::traduire_texte(&c.description));
                t.push('\n');
            }
            if let Some(s) = quete_de(nom).and_then(|f| crate::savoir::savoir(f.cle)) {
                for (_, texte) in s.rubriques() {
                    t.push_str(texte);
                    t.push('\n');
                }
            }
            t
        }
    }
}

/// Les pages où l'on trouve `terme` (sans se soucier des accents ni des majuscules) : celles
/// dont le titre le contient d'abord, puis les autres.
pub fn chercher(terme: &str) -> Vec<Page> {
    let terme = crate::fenetres::plier(terme.trim());
    if terme.is_empty() {
        return Vec::new();
    }
    let mut par_titre = Vec::new();
    let mut ailleurs = Vec::new();
    for page in toutes_les_pages() {
        let nom = match &page {
            Page::Brique(n) => n.clone(),
            _ => String::new(),
        };
        if crate::fenetres::plier(&titre(&page)).contains(&terme)
            || crate::fenetres::plier(&nom).contains(&terme)
        {
            par_titre.push(page);
        } else if crate::fenetres::plier(&texte_cherchable(&page)).contains(&terme) {
            ailleurs.push(page);
        }
    }
    par_titre.extend(ailleurs);
    par_titre
}

/// La page à ouvrir pour une brique posée dans le plan, d'après le nom de son modèle : la
/// brique de la bibliothèque du même nom, ou celle que répare une quête de dépannage.
pub fn page_de_modele(modele: &str) -> Option<Page> {
    if est_documentee(modele) {
        return Some(Page::Brique(modele.to_owned()));
    }
    PARCOURS
        .iter()
        .find(|f| f.brique == modele && est_documentee(f.corrige))
        .map(|f| Page::Brique(f.corrige.to_owned()))
}

/// La page d'un élément posé dans le plan : sa pièce de base, ou la page de sa brique.
pub fn page_d_element(e: &crate::document::Element) -> Page {
    use crate::document::Genre;
    let cle = match e.genre {
        Genre::Nand => "nand",
        Genre::Entree if e.est_poussoir() => "poussoir",
        Genre::Entree => "interrupteur",
        Genre::Sortie if e.est_afficheur() => "afficheur",
        Genre::Sortie => "lampe",
        Genre::Horloge => "horloge",
        Genre::Ecran => "ecran",
        Genre::Clavier => "clavier",
        Genre::Decoupe => "decoupeur",
        Genre::Jonction => "jonction",
        Genre::Brique => return page_de_modele(&e.modele).unwrap_or(Page::Notion("brique")),
    };
    Page::Notion(cle)
}

/// Les briques de la bibliothèque qui utilisent `nom` directement, dans l'ordre alphabétique.
pub fn utilisee_par(nom: &str) -> Vec<String> {
    corriges()
        .composants
        .iter()
        .filter(|(n, c)| est_documentee(n) && c.instances.iter().any(|i| i.genre == nom))
        .map(|(n, _)| n.clone())
        .collect()
}

/// Ce que veut dire une broche, d'après son nom et son sens, pour les noms d'usage courant.
pub fn role_de_broche(nom: &str, entree: bool) -> Option<&'static str> {
    let texte = match (nom, entree) {
        ("clk", true) => {
            "l'horloge : la brique agit à chaque front montant, quand clk passe de 0 à 1"
        }
        ("raz", true) => "remise à zéro : à 1, la brique revient à son état de départ",
        ("nraz", true) => "remise à zéro active à 0 : le n dit que la commande est inversée",
        ("sel", true) => "le choix : son numéro désigne l'entrée qui passe",
        ("en", true) => "autorisation : à 1, la brique avance ; à 0, elle attend",
        ("ecrire", true) => "à 1, la valeur présentée est rangée au prochain front d'horloge",
        ("charge", true) => "à 1, la valeur d est prise au prochain front d'horloge",
        ("charger", true) => "à 1, une nouvelle valeur est prise au prochain front d'horloge",
        ("d", true) => "la donnée qui entre",
        ("d", false) => "la donnée qui sort",
        ("q", false) => "la valeur gardée en mémoire",
        ("q_n", false) => "l'inverse de q",
        ("s_n", true) => "met q à 1 quand elle vaut 0 (commande active à 0)",
        ("r_n", true) => "met q à 0 quand elle vaut 0 (commande active à 0)",
        ("adresse", true) => "le numéro de la case choisie",
        ("a" | "b", true) => "ce sur quoi la brique travaille : un bit ou un nombre",
        ("s", false) => "le résultat",
        ("e", true) => "l'entrée",
        ("op", true) => "le numéro de l'opération à faire",
        ("pc", false) => "le compteur de programme : l'adresse de l'instruction en cours",
        ("tx", false) => "le fil d'émission, qui vaut 1 au repos",
        ("rx", true) => "le fil de réception",
        ("irq", true) => "les demandes d'interruption, une par bit",
        ("ack", true) => "acquitte la demande servie",
        ("occupe", false) => "vaut 1 tant qu'un envoi est en cours",
        ("pret", false) => "vaut 1 quand une donnée vient d'arriver",
        ("plein", false) => "vaut 1 quand il n'y a plus de place",
        ("vide", false) => "vaut 1 quand il n'y a rien à lire",
        ("retenue", false) => {
            "vaut 1 quand le compte repasse à zéro : de quoi faire avancer le suivant"
        }
        ("rs", false) => "la retenue qui sort",
        ("re", true) => "la retenue qui entre",
        ("fini", false) => "vaut 1 quand le calcul est terminé",
        ("erreur", false) => "vaut 1 quand quelque chose s'est mal passé",
        ("clavier", true) => "le code de la touche enfoncée, 0 sinon",
        ("instr", true) => "l'instruction lue dans la mémoire morte",
        ("lu", true) => "l'octet lu dans la mémoire vive",
        ("egal", false) => "vaut 1 quand a = b",
        ("inf", false) => "vaut 1 quand a < b",
        ("sup", false) => "vaut 1 quand a > b",
        _ => return None,
    };
    Some(texte)
}

/// Tous les rôles de broches, pour les traduire.
pub(crate) fn roles_de_broches() -> Vec<&'static str> {
    let mut roles = Vec::new();
    for c in corriges().composants.values() {
        for b in &c.broches {
            if let Some(r) = role_de_broche(&b.nom, b.sens == Sens::Entree) {
                roles.push(r);
            }
        }
    }
    roles.sort_unstable();
    roles.dedup();
    roles
}

// --- Ce qui se calcule ---------------------------------------------------------------------

/// Une ligne d'exemple : la valeur de chaque broche (`None` : ni 0 ni 1, un `X` ou un `Z`).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Ligne {
    /// Les entrées, dans l'ordre des broches.
    pub entrees: Vec<Option<u64>>,
    /// Les sorties, dans l'ordre des broches.
    pub sorties: Vec<Option<u64>>,
}

/// Un signal d'un chronogramme : sa valeur à chaque pas.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Signal {
    /// Le nom de la broche.
    pub nom: String,
    /// Son nombre de bits.
    pub largeur: u32,
    /// Vrai pour une entrée.
    pub entree: bool,
    /// Sa valeur à chaque pas.
    pub valeurs: Vec<Option<u64>>,
}

/// Ce que la page montre du comportement d'une brique.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Comportement {
    /// Brique combinatoire : sa table de vérité, entière ou en partie.
    Table {
        /// Les lignes montrées.
        lignes: Vec<Ligne>,
        /// Le nombre de lignes de la table entière.
        total: u64,
    },
    /// Brique avec mémoire : un chronogramme, deux pas par cycle d'horloge (horloge à 0, puis
    /// à 1), ou un pas par changement d'entrée sans horloge.
    Chronogramme {
        /// Les signaux, l'horloge d'abord, puis les entrées, puis les sorties.
        signaux: Vec<Signal>,
        /// Vrai si la brique a une horloge `clk`.
        horloge: bool,
    },
    /// Rien à montrer (la brique ne se simule pas seule).
    Aucun,
}

/// La fiche technique d'une brique, calculée depuis son corrigé.
#[derive(Clone, Debug)]
pub struct Technique {
    /// Les entrées : nom et nombre de bits.
    pub entrees: Vec<(String, u32)>,
    /// Les sorties : nom et nombre de bits.
    pub sorties: Vec<(String, u32)>,
    /// Les pièces que le corrigé assemble directement, avec leur nombre.
    pub pieces: Vec<(String, usize)>,
    /// Le nombre de portes NAND une fois tout mis à plat.
    pub nand: Option<u64>,
    /// Le temps de réponse, en unités de temps (une par NAND traversée), pour une brique sans
    /// boucle.
    pub reponse: Option<u64>,
    /// La plus petite construction connue.
    pub minimum: Option<Minimum>,
    /// Vrai si la brique a une mémoire (une boucle, ou une horloge).
    pub memoire: bool,
    /// Ce que fait la brique, montré par des exemples simulés.
    pub comportement: Comportement,
}

/// Un générateur pseudo-aléatoire fixe : les exemples sont les mêmes à chaque ouverture.
struct Hasard(u64);

impl Hasard {
    fn de(nom: &str) -> Hasard {
        // FNV-1a : une graine qui dépend du nom, jamais nulle.
        let mut h: u64 = 0xcbf2_9ce4_8422_2325;
        for o in nom.bytes() {
            h ^= u64::from(o);
            h = h.wrapping_mul(0x0100_0000_01b3);
        }
        Hasard(h | 1)
    }

    fn suivant(&mut self) -> u64 {
        // xorshift64*
        self.0 ^= self.0 >> 12;
        self.0 ^= self.0 << 25;
        self.0 ^= self.0 >> 27;
        self.0.wrapping_mul(0x2545_F491_4F6C_DD1D)
    }

    fn bits(&mut self, largeur: u32) -> u64 {
        self.suivant() & masque(largeur)
    }
}

fn masque(largeur: u32) -> u64 {
    if largeur >= 64 {
        u64::MAX
    } else {
        (1u64 << largeur) - 1
    }
}

/// Lignes montrées au plus quand la table entière est trop longue.
pub const LIGNES_MONTREES: usize = 12;
/// Lignes au-delà desquelles on ne montre plus toute la table.
pub const LIGNES_ENTIERES: u64 = 16;
/// Cycles d'horloge simulés pour un chronogramme.
pub const CYCLES: usize = 12;
/// Budget de temps pour attendre le repos après chaque changement.
const BUDGET: u64 = 20_000;

/// La fiche technique de la brique `nom`, calculée une fois puis gardée.
pub fn technique(nom: &str) -> Option<Arc<Technique>> {
    static CACHE: OnceLock<Mutex<BTreeMap<String, Option<Arc<Technique>>>>> = OnceLock::new();
    let cache = CACHE.get_or_init(|| Mutex::new(BTreeMap::new()));
    if let Some(t) = cache.lock().ok()?.get(nom) {
        return t.clone();
    }
    let t = calculer(nom).map(Arc::new);
    cache.lock().ok()?.insert(nom.to_owned(), t.clone());
    t
}

fn calculer(nom: &str) -> Option<Technique> {
    let bib = corriges();
    let composant = bib.composants.get(nom)?;
    let mut entrees = Vec::new();
    let mut sorties = Vec::new();
    for b in &composant.broches {
        match b.sens {
            Sens::Entree => entrees.push((b.nom.clone(), b.largeur)),
            Sens::Sortie => sorties.push((b.nom.clone(), b.largeur)),
        }
    }
    let mut pieces: Vec<(String, usize)> = Vec::new();
    for i in &composant.instances {
        match pieces.iter_mut().find(|(g, _)| *g == i.genre) {
            Some((_, n)) => *n += 1,
            None => pieces.push((i.genre.clone(), 1)),
        }
    }
    pieces.sort_by(|a, b| b.1.cmp(&a.1).then_with(|| a.0.cmp(&b.0)));
    let circuit = Circuit::elaborer(bib, nom).ok();
    let nand = circuit.as_ref().map(ce_circuit::minimum::nb_nand);
    let reponse = circuit
        .as_ref()
        .filter(|c| c.netlist.horloges.is_empty())
        .and_then(|c| ce_circuit::analyse::chemin_critique(&c.netlist));
    let horloge = entrees.iter().any(|(n, l)| n == "clk" && *l == 1);
    let memoire = reponse.is_none() || horloge;
    let minimum = ce_circuit::minimum::minimum(bib, nom);
    let comportement = match circuit.and_then(|c| Banc::de_circuit(c).ok()) {
        None => Comportement::Aucun,
        Some(banc) if !memoire => table(banc, nom, &entrees, &sorties),
        Some(banc) => chronogramme(banc, nom, &entrees, &sorties, horloge),
    };
    Some(Technique {
        entrees,
        sorties,
        pieces,
        nand,
        reponse,
        minimum,
        memoire,
        comportement,
    })
}

fn lire(banc: &Banc, broches: &[(String, u32)]) -> Vec<Option<u64>> {
    broches
        .iter()
        .map(|(n, _)| banc.lire(n).ok().flatten())
        .collect()
}

/// La table de vérité d'une brique combinatoire : entière si elle est courte, sinon
/// [`LIGNES_MONTREES`] lignes tirées au hasard (toujours les mêmes), sans doublon.
fn table(
    mut banc: Banc,
    nom: &str,
    entrees: &[(String, u32)],
    sorties: &[(String, u32)],
) -> Comportement {
    let bits: u32 = entrees.iter().map(|(_, l)| l).sum();
    let total = if bits >= 64 { u64::MAX } else { 1u64 << bits };
    let mut combinaisons: Vec<Vec<u64>> = Vec::new();
    if total <= LIGNES_ENTIERES {
        for k in 0..total {
            let mut reste = k;
            let mut ligne = Vec::new();
            // Comme la table de vérité de `ce_circuit` : la première entrée change le plus vite.
            for (_, l) in entrees {
                ligne.push(reste & masque(*l));
                reste >>= l;
            }
            combinaisons.push(ligne);
        }
    } else {
        let mut hasard = Hasard::de(nom);
        let mut essais = 0;
        while combinaisons.len() < LIGNES_MONTREES && essais < 1000 {
            essais += 1;
            let ligne: Vec<u64> = entrees.iter().map(|(_, l)| hasard.bits(*l)).collect();
            if !combinaisons.contains(&ligne) {
                combinaisons.push(ligne);
            }
        }
        combinaisons.sort();
    }
    let mut lignes = Vec::new();
    for valeurs in combinaisons {
        for ((n, _), v) in entrees.iter().zip(&valeurs) {
            let _ = banc.ecrire(n, *v);
        }
        banc.stabiliser(BUDGET);
        lignes.push(Ligne {
            entrees: valeurs.into_iter().map(Some).collect(),
            sorties: lire(&banc, sorties),
        });
    }
    Comportement::Table { lignes, total }
}

/// Ce qu'on présente sur une entrée au cycle `cycle` d'un chronogramme de `cycles` cycles :
/// la remise à zéro au premier cycle seulement, l'autorisation toujours ; une mémoire est
/// écrite case après case pendant la première moitié, puis relue dans le même ordre ; un
/// chargement ou un envoi a lieu au premier cycle et au milieu ; le reste au hasard (toujours
/// le même).
fn stimulus(nom: &str, largeur: u32, cycle: usize, cycles: usize, hasard: &mut Hasard) -> u64 {
    let moitie = (cycles / 2).max(1);
    match nom {
        "raz" => u64::from(cycle == 0),
        "nraz" => u64::from(cycle != 0),
        "en" => 1,
        "ecrire" => u64::from(cycle < moitie),
        "adresse" | "ad_e" | "ad_a" | "ad_b" | "ligne_e" => {
            (cycle % moitie) as u64 & masque(largeur)
        }
        "charger" | "envoyer" => u64::from(cycle == 1 || cycle == moitie + 1),
        _ if largeur == 1 => hasard.bits(1),
        _ => hasard.bits(largeur),
    }
}

/// Le chronogramme d'une brique avec mémoire, depuis la mise sous tension.
fn chronogramme(
    mut banc: Banc,
    nom: &str,
    entrees: &[(String, u32)],
    sorties: &[(String, u32)],
    horloge: bool,
) -> Comportement {
    let mut hasard = Hasard::de(nom);
    let mut signaux: Vec<Signal> = Vec::new();
    let vide = |n: &str, l: u32, entree: bool| Signal {
        nom: n.to_owned(),
        largeur: l,
        entree,
        valeurs: Vec::new(),
    };
    if horloge {
        signaux.push(vide("clk", 1, true));
    }
    let autres: Vec<&(String, u32)> = entrees
        .iter()
        .filter(|(n, _)| !(horloge && n == "clk"))
        .collect();
    for (n, l) in &autres {
        signaux.push(vide(n, *l, true));
    }
    for (n, l) in sorties {
        signaux.push(vide(n, *l, false));
    }
    let noter = |banc: &Banc, signaux: &mut Vec<Signal>| {
        for s in signaux.iter_mut() {
            s.valeurs.push(banc.lire(&s.nom).ok().flatten());
        }
    };
    let pas = if horloge { CYCLES } else { CYCLES + 4 };
    for cycle in 0..pas {
        for (n, l) in &autres {
            let v = stimulus(n, *l, cycle, pas, &mut hasard);
            let _ = banc.ecrire(n, v);
        }
        if horloge {
            let _ = banc.ecrire("clk", 0);
            banc.stabiliser(BUDGET);
            noter(&banc, &mut signaux);
            let _ = banc.ecrire("clk", 1);
        }
        banc.stabiliser(BUDGET);
        noter(&banc, &mut signaux);
    }
    Comportement::Chronogramme { signaux, horloge }
}

// --- Les pages écrites à la main -------------------------------------------------------------

/// Les pièces de base et les notions.
pub const NOTIONS: &[Notion] = &[
    // Les pièces de la palette.
    Notion {
        cle: "nand",
        famille: Famille::Piece,
        titre: "La porte NAND",
        resume: "La seule porte logique de l'application : tout le reste en est fait.",
        texte: "# Ce qu'elle fait
Sa sortie s vaut 0 seulement quand ses deux entrées a et b valent 1. Dans tous les autres cas, elle vaut 1. Son nom vient de l'anglais « not and » : c'est un ET suivi d'un NON.

# Sa table de vérité
| a | b | s
| 0 | 0 | 1
| 0 | 1 | 1
| 1 | 0 | 1
| 1 | 1 | 0

# Ses broches
- a et b : les deux entrées, d'un bit chacune.
- s : la sortie, d'un bit.

# Son temps de réponse
Une NAND met une unité de temps à répondre : quand une entrée change, la sortie change un instant plus tard. Le temps de réponse d'un circuit se compte donc en portes traversées.

# Ce qu'elle lit quand une entrée est inconnue
Si une entrée vaut 0, la sortie vaut 1 quoi que vaille l'autre. Sinon, une entrée X ou Z (rien ne la pilote) rend la sortie X : on ne peut rien garantir.

# Pourquoi une seule porte suffit
Une NAND dont les deux entrées sont reliées donne le NON. Une NAND suivie d'un NON donne le ET. Avec des NON devant ses entrées, elle donne le OU. Avec NON, ET et OU, on écrit n'importe quelle table de vérité : la NAND est universelle.",
        voir: &["NON", "ET", "OU", "signaux", "temps"],
        quetes: &["non", "et", "ou"],
    },
    Notion {
        cle: "interrupteur",
        famille: Famille::Piece,
        titre: "L'interrupteur",
        resume: "Une entrée du circuit : un clic la fait passer de 0 à 1.",
        texte: "# Ce qu'il fait
L'interrupteur est une entrée du circuit. Clique dessus pour le faire passer de 0 à 1, ou de 1 à 0. Tout ce qui est branché dessus réagit aussitôt, porte après porte.

# Un ou plusieurs bits
Dans les infos, « Bits » règle sa largeur, de 1 à 64. Au-delà de 1, c'est un bus : l'interrupteur porte un nombre, qu'on règle en cliquant sur chacun de ses bits.

# Dans une brique
Les interrupteurs du plan d'une brique deviennent ses broches d'entrée, avec leur nom. Une fois la brique posée ailleurs, ce sont les fils qui arrivent sur ces broches qui les pilotent.

# Bon à savoir
Donne-lui un nom court et parlant (a, sel, clk, raz) : c'est le nom de la broche, et celui que l'on lit dans le chronogramme. Dans une quête, ses noms sont imposés.",
        voir: &["poussoir", "lampe", "bus"],
        quetes: &["non"],
    },
    Notion {
        cle: "poussoir",
        famille: Famille::Piece,
        titre: "Le bouton poussoir",
        resume: "Une entrée d'un bit qui vaut 1 tant qu'on appuie dessus.",
        texte: "# Ce qu'il fait
Le bouton poussoir est un interrupteur d'un bit qui revient tout seul à 0 : il vaut 1 tant que tu appuies dessus, 0 dès que tu le lâches.

# À quoi il sert
Il donne une impulsion : faire avancer un compteur d'un pas, envoyer un octet, remettre à zéro, lancer le dé. Pour le circuit, c'est une entrée comme une autre.

# Bon à savoir
Un appui dure longtemps à l'échelle du circuit : des centaines d'unités de temps. Pour qu'un appui ne compte qu'une fois, on le lit au front d'horloge avec un détecteur de front.",
        voir: &["interrupteur", "DETECTEUR_FRONT", "horloge-et-fronts"],
        quetes: &["detecteur-de-front", "de"],
    },
    Notion {
        cle: "lampe",
        famille: Famille::Piece,
        titre: "La lampe",
        resume: "Une sortie du circuit : elle montre la valeur du fil qui l'alimente.",
        texte: "# Ce qu'elle fait
La lampe est une sortie du circuit. Elle s'allume quand son fil vaut 1, reste éteinte à 0, et montre X ou Z quand la valeur est inconnue ou que rien ne la pilote.

# Un ou plusieurs bits
Dans les infos, « Bits » règle sa largeur, de 1 à 64. Une lampe de plusieurs bits montre un nombre.

# Dans une brique
Les lampes du plan d'une brique deviennent ses broches de sortie. Dans une quête, ce sont elles que « Vérifier » compare au corrigé.",
        voir: &["afficheur", "interrupteur", "signaux"],
        quetes: &["non"],
    },
    Notion {
        cle: "afficheur",
        famille: Famille::Piece,
        titre: "L'afficheur 7 segments",
        resume: "Une lampe de 7 bits dessinée comme les chiffres d'une calculatrice.",
        texte: "# Ce qu'il fait
L'afficheur 7 segments est une lampe de 7 bits (8 avec le point). Chaque bit allume un bâton lumineux ; ensemble, ils dessinent un chiffre.

# L'ordre des segments
| bit | segment | place
| 0 | a | en haut
| 1 | b | en haut à droite
| 2 | c | en bas à droite
| 3 | d | en bas
| 4 | e | en bas à gauche
| 5 | f | en haut à gauche
| 6 | g | au milieu
| 7 | point | en bas à droite, s'il y en a 8

# Avec quoi le brancher
Branche-le sur la sortie seg de ton décodeur 7 segments : il traduit un nombre de 0 à 15 en segments à allumer, et l'afficheur montre le chiffre hexadécimal.",
        voir: &["SEPT_SEGMENTS", "lampe", "binaire"],
        quetes: &["sept-segments"],
    },
    Notion {
        cle: "horloge",
        famille: Famille::Piece,
        titre: "L'horloge",
        resume: "Une sortie qui bascule toute seule entre 0 et 1, et cadence le circuit.",
        texte: "# Ce qu'elle fait
L'horloge n'a pas d'entrée. Sa sortie s bascule toute seule : 0 pendant une demi-période, 1 pendant la suivante, et ainsi de suite.

# Sa demi-période
Dans les infos, « Demi-période » règle le nombre d'unités de temps entre deux bascules (8 par défaut). Elle doit laisser au circuit le temps de se calmer entre deux fronts : plus long que le plus long trajet entre deux bascules.

# Pourquoi elle n'est pas faite de NAND
Une boucle d'un nombre impair de NON oscille aussi, mais à un rythme qui dépend des portes. L'horloge joue le rôle du quartz d'un vrai ordinateur : un rythme fixe et sûr. C'est la seule pièce active qui ne soit pas une NAND.

# Bon à savoir
Pendant la lecture, la simulation avance toute seule et l'horloge bat. En pause, « Pas » avance d'une unité de temps.",
        voir: &["horloge-et-fronts", "BASCULE_D", "temps"],
        quetes: &["bascule-d", "compteur-4-bits"],
    },
    Notion {
        cle: "ecran",
        famille: Famille::Piece,
        titre: "L'écran",
        resume: "Affiche un caractère quand sa commande ecrit monte.",
        texte: "# Ce qu'il fait
L'écran a deux entrées : d, un octet, et ecrit, un bit. Quand ecrit passe de 0 à 1, le caractère dont le code est sur d s'ajoute à l'écran. Le code est celui de la table ASCII : 65 pour A, 48 pour 0, 10 pour aller à la ligne.

# Ses broches
- d : le code du caractère, sur 8 bits.
- ecrit : son front montant écrit le caractère.

# Avec quoi le brancher
Dans l'ordinateur miniature, la broche ecran porte l'accumulateur et ecran_ecrit monte à chaque instruction OUT.",
        voir: &["clavier", "ordinateur", "horloge-et-fronts"],
        quetes: &["ordinateur"],
    },
    Notion {
        cle: "clavier",
        famille: Famille::Piece,
        titre: "Le clavier",
        resume: "Présente sur 8 bits le code de la touche tapée.",
        texte: "# Ce qu'il fait
Le clavier a une sortie touche de 8 bits. Sélectionne-le, puis tape au clavier de ton ordinateur : chaque touche présente son code ASCII sur touche pendant un moment, puis touche revient à 0.

# Ses broches
- touche : le code de la touche tapée, 0 quand rien n'est tapé.

# Bon à savoir
Dans les infos, la durée d'une touche se règle : elle doit laisser au programme le temps de la lire. Dans l'ordinateur miniature, l'instruction IN copie touche dans l'accumulateur.",
        voir: &["ecran", "ordinateur"],
        quetes: &["ordinateur"],
    },
    Notion {
        cle: "decoupeur",
        famille: Famille::Piece,
        titre: "Le découpeur",
        resume: "Prend des morceaux d'un bus : un bit, ou une plage de bits.",
        texte: "# Ce qu'il fait
Le découpeur a une entrée e, un bus, et une sortie par morceau choisi. Une sortie 3 donne le bit 3 de e ; une sortie 0:3 donne les bits 0 à 3, comme un bus de 4 bits.

# À quoi il sert
À séparer les parties d'un nombre : les unités et les dizaines d'une montre, le code et l'opérande d'une instruction, les bits d'un octet pour les mener chacun à sa porte.

# Bon à savoir
Il ne coûte aucune porte et ne retarde rien : ce n'est qu'une façon de ranger les fils.",
        voir: &["jonction", "bus", "binaire"],
        quetes: &["additionneur-4-bits"],
    },
    Notion {
        cle: "jonction",
        famille: Famille::Piece,
        titre: "La jonction",
        resume: "Met des fils bout à bout pour en faire un bus.",
        texte: "# Ce qu'elle fait
La jonction a une entrée par morceau et une sortie s, un bus qui les met bout à bout. La première entrée donne les bits de poids faible.

# À quoi elle sert
À rassembler les bits calculés un par un en un nombre : les quatre sommes d'un additionneur 4 bits, les bits d'un compteur, les lignes d'une mémoire.

# Bon à savoir
Comme le découpeur, elle ne coûte aucune porte et ne retarde rien.",
        voir: &["decoupeur", "bus", "binaire"],
        quetes: &["additionneur-4-bits"],
    },
    Notion {
        cle: "brique",
        famille: Famille::Piece,
        titre: "La brique",
        resume: "Un circuit enregistré, posé comme une seule pièce et réutilisable partout.",
        texte: "# Ce qu'elle est
Une brique est un circuit enregistré sous un nom. Posée dans un plan, elle se montre comme une seule pièce, avec ses broches : les interrupteurs de son plan deviennent ses entrées, ses lampes ses sorties.

# La fabriquer
- Sélectionne des éléments (Maj + glisser un cadre, ou Maj + clic), puis Ctrl+G : ils deviennent une brique, rangée dans la palette.
- Chaque quête réussie te donne aussi une brique : ta propre solution.

# Entrer dedans
Double-clic sur une brique posée pour entrer dedans et voir ses portes en marche ; Échap ou le fil d'Ariane pour remonter. La simulation continue : tu vois ce que la brique fait, là où elle est posée.

# La décrire
Sélectionne une brique posée : dans les infos, écris ce qu'elle fait en une phrase. La description s'affiche au survol dans la palette, et la recherche de la palette la lit.

# Pourquoi c'est la clé
On ne pense pas un processeur en portes NAND : on le pense en registres, en unité de calcul, en compteur de programme. Chaque brique cache ses détails pour qu'on puisse construire au niveau suivant.",
        voir: &["hierarchie", "nand"],
        quetes: &["et"],
    },
    // Les notions.
    Notion {
        cle: "signaux",
        famille: Famille::Notion,
        titre: "Les quatre valeurs d'un fil",
        resume: "0, 1, X et Z : ce que porte un fil, et comment on le voit.",
        texte: "# Les quatre valeurs
| valeur | ce qu'elle veut dire | à l'écran
| 1 | le fil est allumé | vert, lumineux, des perles avancent
| 0 | le fil est éteint | gris, sans lueur
| X | inconnu, ou en conflit | orange, tireté qui défile
| Z | rien ne pilote le fil | gris clair, pointillé immobile

# D'où vient un X
- À la mise sous tension, avant que rien n'ait été calculé : une mémoire ne sait pas encore ce qu'elle contient. Une remise à zéro la fixe.
- Quand deux sorties reliées au même fil se contredisent : c'est un conflit.
- Quand une porte lit un X ou un Z et que son autre entrée ne suffit pas à décider.
- Quand un circuit hésite sans fin, comme une boucle d'un nombre impair de NON.

# Le X ne ment jamais
Le simulateur ne transforme jamais un X en une valeur qu'il ne pourrait pas garantir : quand il affiche 0 ou 1, ce serait vrai quelle que soit la vraie valeur des fils inconnus. C'est démontré dans le document « Sémantique » du projet.

# D'où vient un Z
Un fil qui n'est branché sur aucune sortie, ou une broche d'une brique que rien ne pilote. Une porte qui lit un Z le lit comme un inconnu.",
        voir: &["temps", "memoire", "lampe"],
        quetes: &["non"],
    },
    Notion {
        cle: "temps",
        famille: Famille::Notion,
        titre: "Le temps et le délai des portes",
        resume: "Chaque NAND met une unité de temps à répondre : d'où le chemin critique.",
        texte: "# Une unité de temps par porte
Quand une entrée change, la sortie d'une NAND change une unité de temps plus tard. Un signal qui traverse dix portes met donc dix unités à arriver.

# Le chemin critique
C'est le plus long trajet, compté en portes, entre une entrée et une sortie. Il fixe le temps de réponse du circuit : après lui, plus rien ne bouge. La documentation de chaque brique sans mémoire donne le sien.

# Pourquoi ça compte
- Un additionneur 8 bits à retenue propagée attend que la retenue traverse les huit colonnes ; l'additionneur rapide calcule les retenues en avance et répond bien plus vite.
- Dans un circuit cadencé, le chemin critique entre deux bascules doit tenir dans une période d'horloge.

# Les impulsions trop courtes
Une porte ignore un changement d'entrée qui ne dure pas aussi longtemps que son délai : c'est le délai inertiel. Pendant qu'elle hésite, sa sortie peut valoir X. Un court scintillement, quand deux chemins de longueurs différentes se rejoignent, s'appelle un aléa.",
        voir: &["ADD8", "ADD8_RAPIDE", "horloge-et-fronts", "simulation"],
        quetes: &["additionneur-rapide"],
    },
    Notion {
        cle: "combinatoire",
        famille: Famille::Notion,
        titre: "Combinatoire ou séquentiel",
        resume: "Avec ou sans mémoire : deux familles de circuits, deux façons de les décrire.",
        texte: "# Combinatoire : sans mémoire
Les sorties ne dépendent que des entrées du moment. Mêmes entrées, mêmes sorties, toujours. Un tel circuit n'a pas de boucle, et il se décrit entièrement par sa table de vérité. Exemples : les portes, l'additionneur, le multiplexeur, le décodeur.

# Séquentiel : avec mémoire
Les sorties dépendent aussi du passé. Le circuit contient une boucle, qui garde un état. Il se décrit par ce qu'il fait au fil du temps : un chronogramme, ou un automate (des états et des transitions). Exemples : le verrou, la bascule, le registre, le compteur, la mémoire.

# Dans la documentation
Chaque brique sans mémoire montre sa table de vérité, simulée ligne à ligne. Chaque brique avec mémoire montre un chronogramme, simulé front après front depuis la mise sous tension.

# Dans les quêtes
Une quête combinatoire essaie toutes les combinaisons d'entrées (ou un large échantillon). Une quête séquentielle joue une longue suite d'actions sur ton circuit et sur le corrigé, et compare.",
        voir: &["memoire", "horloge-et-fronts"],
        quetes: &["verrou-sr"],
    },
    Notion {
        cle: "binaire",
        famille: Famille::Notion,
        titre: "Les nombres en binaire",
        resume: "Bits, poids, hexadécimal, nombres négatifs : comment un bus porte un nombre.",
        texte: "# Les bits et leur poids
En binaire, chaque chiffre vaut 0 ou 1, et chaque rang vaut le double du précédent : 1, 2, 4, 8, 16… Ainsi 1101 vaut 8 + 4 + 0 + 1 = 13.

# Dans l'application
Sur un bus, le bit 0 est le bit de poids faible (il vaut 1) ; le bit n vaut 2 puissance n. Un bus de n bits porte un nombre de 0 à 2 puissance n moins 1 : de 0 à 255 pour un octet.

| décimal | binaire | hexadécimal
| 0 | 0000 | 0
| 5 | 0101 | 5
| 9 | 1001 | 9
| 10 | 1010 | A
| 15 | 1111 | F

# L'hexadécimal
Un chiffre hexadécimal (de 0 à 9, puis de A à F) résume quatre bits. Un octet s'écrit avec deux : 0x2A vaut 42.

# Les nombres négatifs
En complément à deux, on obtient −a en inversant tous les bits de a puis en ajoutant 1. Sur 8 bits, −1 s'écrit 11111111. L'avantage : l'addition ordinaire marche aussi pour les négatifs, et a − b se calcule comme a + (−b).

# Le décimal codé en binaire
Une montre ou une calculatrice range souvent chaque chiffre décimal dans 4 bits : 59 s'écrit 0101 1001. C'est plus simple à afficher, chiffre par chiffre.",
        voir: &["bus", "NEGATIF8", "BINAIRE_DCB", "ADD4"],
        quetes: &["negatif", "binaire-decimal"],
    },
    Notion {
        cle: "bus",
        famille: Famille::Notion,
        titre: "Les bus",
        resume: "Un groupe de fils qui portent ensemble un nombre de plusieurs bits.",
        texte: "# Ce qu'est un bus
Un bus est un faisceau de fils tirés d'un seul trait : un interrupteur, une lampe ou une broche de 8 bits se relie par un seul fil épais. Sa largeur est écrite à côté.

# Découper et réunir
- Le découpeur prend un bit, ou une plage de bits, d'un bus.
- La jonction met des fils bout à bout pour en faire un bus.

# Les bus partagés
Dans un vrai ordinateur, plusieurs pièces parlent à tour de rôle sur un même bus : chacune se débranche quand ce n'est pas son tour, avec un tampon trois états, et le fil vaut Z quand personne ne parle. Ici, on choisit plutôt qui parle avec un multiplexeur : rien ne peut entrer en conflit.

# Bon à savoir
Une brique dont deux broches ont des largeurs différentes refuse le fil : la largeur s'affiche dans les infos de chaque broche.",
        voir: &["decoupeur", "jonction", "MUX8", "binaire"],
        quetes: &["additionneur-4-bits", "registre-8-bits"],
    },
    Notion {
        cle: "memoire",
        famille: Famille::Notion,
        titre: "Comment un circuit se souvient",
        resume: "Une boucle qui s'entretient : du verrou SR jusqu'à la mémoire vive.",
        texte: "# L'idée : une boucle
Deux NON en boucle tiennent une valeur sans fin : chacun redonne à l'autre ce qu'il lui faut. Pour pouvoir écrire dans cette boucle, on remplace les NON par des NAND : c'est le verrou SR.

# Du verrou à la mémoire vive
- Verrou SR : deux NAND croisées. s_n à 0 met q à 1, r_n à 0 le met à 0, les deux à 1 gardent la valeur.
- Verrou D : q suit d tant que e vaut 1, et garde sa valeur quand e repasse à 0.
- Bascule D : deux verrous D l'un derrière l'autre ; q ne change qu'au front montant de l'horloge.
- Registre : une bascule par bit, et un multiplexeur pour choisir entre garder et charger.
- Mémoire vive : des registres, un décodeur qui choisit lequel écrire, et des multiplexeurs qui choisissent lequel lire.

# Pourquoi la bascule plutôt que le verrou
Un verrou ouvert laisse tout passer : si sa sortie revient à son entrée par un calcul, la boucle s'emballe. La bascule ne prend sa nouvelle valeur qu'à l'instant du front : tout le circuit avance d'un pas à la fois, au rythme de l'horloge.

# À la mise sous tension
Une mémoire ne sait pas ce qu'elle contient : ses sorties valent X jusqu'à la première écriture ou la première remise à zéro. Les chronogrammes de la documentation le montrent.",
        voir: &["VERROU_SR", "VERROU_D", "BASCULE_D", "REGISTRE8", "RAM16"],
        quetes: &["verrou-sr", "verrou-d", "bascule-d", "registre-8-bits", "memoire-vive"],
    },
    Notion {
        cle: "horloge-et-fronts",
        famille: Famille::Notion,
        titre: "Horloge, fronts et circuits cadencés",
        resume: "Tout avance au même instant, au front montant de l'horloge.",
        texte: "# Le front montant
Le front montant est l'instant où un signal passe de 0 à 1 ; le front descendant, de 1 à 0. Les bascules, registres, compteurs et mémoires de l'application agissent au front montant de leur entrée clk.

# Un circuit cadencé
Toutes les bascules reçoivent la même horloge. Entre deux fronts, les portes calculent les valeurs suivantes ; au front, toutes les bascules les prennent ensemble. On raisonne alors pas à pas : à chaque front, l'état devient l'état suivant.

# Les règles d'or
- Les entrées d'une bascule doivent être calmes juste avant le front : la période doit être plus longue que le chemin critique entre deux bascules.
- Une commande (ecrire, charger, raz…) se lit au front : il suffit qu'elle soit présente à ce moment-là.
- Pour qu'une impulsion longue, comme l'appui sur un bouton, ne compte qu'une fois, on détecte son front.

# Lire un chronogramme
Chaque ligne est un signal, le temps va de gauche à droite. Un signal d'un bit est haut à 1 et bas à 0 ; un bus montre sa valeur dans une case qui change à chaque nouvelle valeur ; X est hachuré. Les traits verticaux marquent les fronts montants de l'horloge.",
        voir: &["horloge", "BASCULE_D", "COMPTEUR4", "DETECTEUR_FRONT", "temps"],
        quetes: &["bascule-d", "compteur-4-bits", "detecteur-de-front"],
    },
    Notion {
        cle: "hierarchie",
        famille: Famille::Notion,
        titre: "Les briques et les niveaux",
        resume: "De la NAND à l'ordinateur : chaque niveau se construit avec le précédent.",
        texte: "# Monter d'un niveau à la fois
L'ordinateur miniature s'emboîte sur six niveaux : la NAND, les portes (NON, ET, OU, OU exclusif), les blocs (additionneur, multiplexeur, bascule), les grandes pièces (unité de calcul, registres, compteur de programme, mémoire vive), le processeur, puis l'ordinateur.

# Ce que montre chaque page de brique
- Construite avec : les pièces que la brique assemble directement, à un niveau en dessous.
- Utilisée par : les briques qui s'en servent, un niveau au-dessus.
- Portes NAND : leur nombre une fois tout mis à plat, jusqu'en bas.
- Plus petite construction connue : le record, parfois démontré, à comparer avec ta solution.

# Le prix d'une brique
Réutiliser une brique est commode, mais pas toujours le plus économe : un OU fait d'un NON et d'une NON-OU coûte plus qu'un OU fait de trois NAND. Les étoiles des quêtes mesurent cet écart.",
        voir: &["brique", "ordinateur", "nand"],
        quetes: &["processeur"],
    },
    Notion {
        cle: "ordinateur",
        famille: Famille::Notion,
        titre: "L'ordinateur miniature",
        resume: "Un processeur 8 bits, sa mémoire, un écran et un clavier, tout en NAND.",
        texte: "# Ce qu'il contient
| pièce | brique | rôle
| Horloge | HORLOGE | le rythme, une instruction par cycle
| Processeur | PROCESSEUR | lit et exécute les instructions
| Mémoire vive | RAM16 | 16 octets qu'on lit et qu'on écrit
| Mémoire morte | MEMOIRE_MORTE | le programme, fabriqué en NAND
| Écran | broches ecran et ecran_ecrit | affiche un caractère
| Clavier | broche clavier | le code de la touche tapée
| Remise à zéro | broche raz | remet le processeur au début

# Dans le processeur
- Le décodeur d'instructions traduit le code de l'instruction en commandes.
- L'unité de calcul additionne, soustrait ou fait un NON-ET.
- L'accumulateur A garde le résultat ; deux drapeaux retiennent si le dernier résultat vaut zéro (Z) et s'il y a eu une retenue (C).
- Le compteur de programme garde l'adresse de l'instruction suivante.

# Un cycle d'horloge
- La mémoire morte présente l'instruction rangée à l'adresse du compteur de programme.
- Le décodeur en tire les commandes : quoi calculer, où ranger, faut-il sauter.
- Au front montant, l'accumulateur, les drapeaux, la mémoire vive et le compteur de programme prennent tous leur nouvelle valeur.

# Le programmer
Ouvre l'exemple « Ordinateur miniature » depuis tes projets, puis Ctrl+P : écris un programme en assembleur et charge-le. La mémoire morte est refabriquée porte par porte. Les seize instructions sont dans l'aide (F1), rubrique « Programmer l'ordinateur ».",
        voir: &["PROCESSEUR", "UAL", "DECODEUR", "COMPTEUR_PROGRAMME", "RAM16", "ecran", "clavier"],
        quetes: &["ordinateur", "ordinateur-majuscules", "ordinateur-a-toi", "ordinateur-256"],
    },
    Notion {
        cle: "simulation",
        famille: Famille::Notion,
        titre: "Comment l'application simule",
        resume: "Un moteur qui ne recalcule que ce qui change, et ce qu'il garantit.",
        texte: "# Ne recalculer que ce qui change
Le moteur ne recalcule une porte que lorsqu'une de ses entrées vient de changer. Un circuit au repos ne coûte rien, et l'ordinateur miniature, avec ses milliers de portes, tourne à plusieurs milliers de cycles par seconde.

# Les vitesses
- Pas : avance d'une unité de temps.
- Lecture : le temps avance tout seul, à la vitesse choisie.
- Mode rapide : pour les grands circuits cadencés, le calcul avance cycle par cycle, sans montrer chaque unité de temps ; le résultat est le même.

# Ce qui est garanti
- Le résultat ne dépend pas de l'ordre dans lequel les portes sont calculées.
- Le simulateur rapide donne exactement ce que donne la définition écrite pas à pas.
- Un X affiché n'en cache pas une valeur sûre, et une valeur affichée est vraie.
Ces résultats sont démontrés, ou vérifiés sur des dizaines de milliers de circuits, dans le document « Sémantique » du projet.

# Ce que le modèle ne représente pas
Pas de tensions ni de courant : une porte est une fonction logique idéale suivie d'un délai. Le niveau des transistors viendra plus tard.",
        voir: &["signaux", "temps"],
        quetes: &[],
    },
];

/// Tous les textes écrits à la main de la documentation, pour les traduire.
pub(crate) fn textes() -> Vec<&'static str> {
    let mut t = Vec::new();
    for n in NOTIONS {
        t.extend([n.titre, n.resume, n.texte]);
    }
    t.extend(roles_de_broches());
    t
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn chaque_lien_mene_a_une_page() {
        for n in NOTIONS {
            for cle in n.voir {
                let ok = notion(cle).is_some() || est_documentee(cle);
                assert!(ok, "« {} » renvoie vers « {cle} », qui n'existe pas", n.cle);
            }
            for q in n.quetes {
                assert!(
                    PARCOURS.iter().any(|f| f.cle == *q),
                    "« {} » cite la quête « {q} », qui n'existe pas",
                    n.cle
                );
            }
        }
    }

    #[test]
    fn les_cles_sont_uniques_et_chaque_texte_est_bien_balise() {
        let mut cles: Vec<&str> = NOTIONS.iter().map(|n| n.cle).collect();
        cles.sort_unstable();
        let avant = cles.len();
        cles.dedup();
        assert_eq!(avant, cles.len(), "deux notions ont la même clé");
        for n in NOTIONS {
            assert!(
                n.texte.starts_with("# "),
                "« {} » commence sans intertitre",
                n.cle
            );
            // Les lignes d'un tableau ont toutes le même nombre de cases que l'en-tête.
            let mut cases = None;
            for ligne in n.texte.lines() {
                match ligne.strip_prefix("| ") {
                    Some(l) => {
                        let k = l.split(" | ").count();
                        assert_eq!(
                            *cases.get_or_insert(k),
                            k,
                            "tableau boiteux dans « {} »",
                            n.cle
                        );
                    }
                    None => cases = None,
                }
            }
        }
    }

    #[test]
    fn le_sommaire_couvre_chaque_brique_du_parcours_une_fois() {
        let pages = toutes_les_pages();
        for f in PARCOURS {
            if est_documentee(f.corrige) {
                let n = pages
                    .iter()
                    .filter(|p| **p == Page::Brique(f.corrige.to_owned()))
                    .count();
                assert_eq!(n, 1, "{} apparaît {n} fois", f.corrige);
            }
        }
        // Chaque brique de la bibliothèque a sa page, sauf celles qui dépendent du programme.
        for nom in corriges().composants.keys() {
            assert_eq!(
                pages.contains(&Page::Brique(nom.clone())),
                !A_PART.contains(&nom.as_str()),
                "{nom}"
            );
        }
    }

    #[test]
    fn la_table_du_ou_exclusif_est_juste_et_entiere() {
        let t = technique("OU_X").expect("OU_X");
        assert_eq!(t.nand, Some(4));
        assert_eq!(t.reponse, Some(3));
        assert!(!t.memoire);
        let Comportement::Table { lignes, total } = &t.comportement else {
            panic!("une table");
        };
        assert_eq!(*total, 4);
        for l in lignes {
            let (a, b) = (l.entrees[0].unwrap(), l.entrees[1].unwrap());
            assert_eq!(l.sorties[0], Some(a ^ b));
        }
        assert_eq!(t.pieces, vec![("NAND".to_owned(), 4)]);
    }

    #[test]
    fn les_lignes_tirees_d_une_grande_table_sont_justes() {
        // L'additionneur 8 bits : 2^17 lignes, on en montre quelques-unes, toutes vérifiées.
        let t = technique("ADD8").expect("ADD8");
        let Comportement::Table { lignes, total } = &t.comportement else {
            panic!("une table");
        };
        assert_eq!(*total, 1 << 17);
        assert_eq!(lignes.len(), LIGNES_MONTREES);
        let noms: Vec<&str> = t.entrees.iter().map(|(n, _)| n.as_str()).collect();
        let sorties: Vec<&str> = t.sorties.iter().map(|(n, _)| n.as_str()).collect();
        let lire =
            |l: &Ligne, n: &str| l.entrees[noms.iter().position(|x| *x == n).unwrap()].unwrap();
        for l in lignes {
            let attendu = lire(l, "a") + lire(l, "b") + lire(l, "re");
            let s = l.sorties[sorties.iter().position(|x| *x == "s").unwrap()].unwrap();
            let r = l.sorties[sorties.iter().position(|x| *x == "rs").unwrap()].unwrap();
            assert_eq!(s + 256 * r, attendu);
        }
    }

    #[test]
    fn le_chronogramme_du_compteur_compte() {
        let t = technique("COMPTEUR4").expect("COMPTEUR4");
        assert!(t.memoire);
        let Comportement::Chronogramme { signaux, horloge } = &t.comportement else {
            panic!("un chronogramme");
        };
        assert!(horloge);
        assert_eq!(signaux[0].nom, "clk");
        let q = signaux.iter().find(|s| s.nom == "q").expect("q");
        // Deux pas par cycle ; raz au premier cycle, puis q compte à chaque front.
        assert_eq!(q.valeurs.len(), 2 * CYCLES);
        assert_eq!(q.valeurs[1], Some(0));
        for cycle in 1..CYCLES {
            assert_eq!(q.valeurs[2 * cycle + 1], Some(cycle as u64 % 16));
        }
    }

    #[test]
    fn chaque_brique_se_calcule() {
        for page in toutes_les_pages() {
            if let Page::Brique(nom) = page {
                let t = technique(&nom).unwrap_or_else(|| panic!("{nom}"));
                assert!(t.nand.is_some(), "{nom} ne se met pas à plat");
                assert_ne!(t.comportement, Comportement::Aucun, "{nom}");
            }
        }
    }

    #[test]
    fn la_recherche_trouve_par_titre_puis_par_texte() {
        crate::langue::choisir_pour_ce_fil(Some(crate::langue::Langue::Francais));
        let r = chercher("bascule");
        assert!(r.contains(&Page::Brique("BASCULE_D".to_owned())));
        assert!(r.contains(&Page::Notion("memoire")));
        // Sans accents ni majuscules.
        assert_eq!(chercher("DECODEUR")[0], chercher("décodeur")[0]);
        assert!(chercher("zzzz introuvable").is_empty());
        crate::langue::choisir_pour_ce_fil(None);
    }

    #[test]
    fn les_grands_nombres_se_lisent() {
        crate::langue::choisir_pour_ce_fil(Some(crate::langue::Langue::Francais));
        assert_eq!(milliers(7), "7");
        assert_eq!(milliers(131_072), "131\u{a0}072");
        assert_eq!(milliers(1_000), "1\u{a0}000");
        crate::langue::choisir_pour_ce_fil(Some(crate::langue::Langue::Anglais));
        assert_eq!(milliers(2_219), "2,219");
        crate::langue::choisir_pour_ce_fil(None);
    }

    #[test]
    fn une_brique_posee_mene_a_sa_page() {
        assert_eq!(page_de_modele("MUX"), Some(Page::Brique("MUX".to_owned())));
        assert_eq!(
            page_de_modele("OU_X_REPARE"),
            Some(Page::Brique("OU_X".to_owned()))
        );
        assert_eq!(page_de_modele("MA_BRIQUE"), None);
        assert!(utilisee_par("OU_X").contains(&"EQUIV".to_owned()));
    }
}
