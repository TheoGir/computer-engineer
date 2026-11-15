//! Le circuit en cours d'édition : des éléments posés sur le plan et reliés par des fils.
//!
//! Ce module ne connaît ni l'écran ni le moteur : il décrit ce que l'utilisateur a construit.
//! Un fil n'est pas un objet à part : c'est le fait qu'une entrée d'un élément désigne une
//! sortie d'un autre, si bien qu'une entrée n'a jamais deux pilotes.
//!
//! Chaque élément a des broches d'entrée et de sortie, chacune large d'un ou plusieurs bits
//! (un bus). Un fil relie toujours une sortie et une entrée de même largeur ; pour prendre une
//! partie d'un bus ou en composer un, on pose un découpeur ou une jonction.

use serde::{Deserialize, Serialize};

/// Identifiant d'un élément, stable pendant toute la vie du document.
pub type Id = u32;

/// Écart vertical entre deux broches voisines d'un même élément.
pub const PAS_BROCHE: f32 = 30.0;

/// Un point du plan.
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize, Default)]
pub struct Point {
    /// Abscisse.
    pub x: f32,
    /// Ordonnée (vers le bas).
    pub y: f32,
}

impl Point {
    /// Point de coordonnées `(x, y)`.
    pub const fn nouveau(x: f32, y: f32) -> Self {
        Point { x, y }
    }

    /// Distance à un autre point.
    pub fn distance(self, autre: Point) -> f32 {
        (self.x - autre.x).hypot(self.y - autre.y)
    }
}

/// Nature d'un élément.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Genre {
    /// Interrupteur : entrée du circuit (ou broche d'entrée d'une brique).
    Entree,
    /// Lampe : sortie du circuit (ou broche de sortie d'une brique).
    Sortie,
    /// Porte NAND à deux entrées.
    Nand,
    /// Une brique : un circuit enregistré, posé comme un seul élément.
    Brique,
    /// Horloge : une sortie qui bascule toute seule.
    Horloge,
    /// Écran : affiche le caractère présenté quand sa commande monte.
    Ecran,
    /// Clavier : présente le code de la touche enfoncée.
    Clavier,
    /// Découpeur : prend des morceaux d'un bus.
    Decoupe,
    /// Jonction : compose un bus à partir de morceaux.
    Jonction,
}

/// L'aspect d'un interrupteur ou d'une lampe : il change sa façon de se dessiner et de se
/// manier, pas ce qu'il est pour le circuit (une entrée ou une sortie comme les autres).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Aspect {
    /// Interrupteur ou lampe ordinaire.
    #[default]
    Normal,
    /// Bouton poussoir : un interrupteur d'un bit qui vaut 1 tant qu'on appuie dessus.
    Poussoir,
    /// Afficheur 7 segments : une lampe de 7 bits (a à g), ou 8 avec le point.
    Segments,
}

impl Aspect {
    fn est_normal(&self) -> bool {
        *self == Aspect::Normal
    }
}

/// Une broche d'un élément : son nom et son nombre de bits.
#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct Broche {
    /// Nom de la broche.
    pub nom: String,
    /// Nombre de bits (1 pour un fil simple).
    pub largeur: u32,
    /// Pour une broche de brique : l'identifiant, dans le plan du modèle, de l'interrupteur ou
    /// de la lampe qu'elle représente. Sert à retrouver les fils quand le modèle change.
    #[serde(default, skip_serializing_if = "est_nul")]
    pub origine: Id,
}

fn est_nul(n: &Id) -> bool {
    *n == 0
}

impl Broche {
    /// Broche de ce nom et de cette largeur.
    pub fn nouvelle(nom: &str, largeur: u32) -> Self {
        Broche {
            nom: nom.to_owned(),
            largeur: largeur.max(1),
            origine: 0,
        }
    }

    /// Broche de brique, qui représente l'élément `origine` du plan de son modèle.
    pub fn de_modele(nom: &str, largeur: u32, origine: Id) -> Self {
        Broche {
            origine,
            ..Broche::nouvelle(nom, largeur)
        }
    }
}

/// Une sortie d'un élément : ce d'où part un fil.
#[derive(
    Clone, Copy, Debug, Default, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize,
)]
#[serde(from = "(Id, usize)", into = "(Id, usize)")]
pub struct Source {
    /// L'élément.
    pub id: Id,
    /// Le numéro de sa sortie.
    pub sortie: usize,
}

impl Source {
    /// La première sortie de l'élément `id` (la seule, pour une porte ou un interrupteur).
    pub const fn de(id: Id) -> Self {
        Source { id, sortie: 0 }
    }

    /// La sortie numéro `sortie` de l'élément `id`.
    pub const fn nouvelle(id: Id, sortie: usize) -> Self {
        Source { id, sortie }
    }
}

impl From<(Id, usize)> for Source {
    fn from((id, sortie): (Id, usize)) -> Self {
        Source { id, sortie }
    }
}

impl From<Source> for (Id, usize) {
    fn from(s: Source) -> Self {
        (s.id, s.sortie)
    }
}

fn est_zero(n: &u64) -> bool {
    *n == 0
}

/// Un élément posé sur le plan.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Element {
    /// Identifiant.
    pub id: Id,
    /// Nature.
    pub genre: Genre,
    /// Nom court (`a`, `s`, `g1`).
    pub nom: String,
    /// Légende affichée sous une sortie (`s · somme`).
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub legende: String,
    /// Position du centre.
    pub pos: Point,
    /// Pour chaque broche d'entrée, la sortie qui y est branchée.
    pub entrees: Vec<Option<Source>>,
    /// Les broches d'entrée (autant que d'`entrees`).
    pub broches_e: Vec<Broche>,
    /// Les broches de sortie.
    pub broches_s: Vec<Broche>,
    /// Pour une brique : le nom de son modèle dans la bibliothèque.
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub modele: String,
    /// Pour une horloge : la durée de chaque niveau. Pour un clavier : la durée d'une touche.
    /// Pour un interrupteur : s'il n'est pas nul, la durée de son impulsion de démarrage.
    #[serde(default, skip_serializing_if = "est_zero")]
    pub param: u64,
    /// Pour un découpeur : les bits (premier, dernier) de chaque sortie.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub plages: Vec<(u32, u32)>,
    /// Pour un interrupteur ou une lampe : son aspect (poussoir, afficheur 7 segments).
    #[serde(default, skip_serializing_if = "Aspect::est_normal")]
    pub aspect: Aspect,
    /// Retourné de gauche à droite : les entrées à droite, les sorties à gauche. Pour faire
    /// revenir un signal vers la gauche sans détour (une boucle, un retour de bus).
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub miroir: bool,
}

/// Demi-période d'une horloge qu'on vient de poser.
pub const DEMI_PERIODE_DEFAUT: u64 = 8;
/// Durée, en unités de temps, pendant laquelle un clavier présente une touche.
pub const DUREE_TOUCHE_DEFAUT: u64 = 2048;

impl Element {
    fn brut(genre: Genre, nom: &str, e: Vec<Broche>, s: Vec<Broche>) -> Self {
        Element {
            id: 0,
            genre,
            nom: nom.to_owned(),
            legende: String::new(),
            pos: Point::default(),
            entrees: vec![None; e.len()],
            broches_e: e,
            broches_s: s,
            modele: String::new(),
            param: 0,
            plages: Vec::new(),
            aspect: Aspect::Normal,
            miroir: false,
        }
    }

    /// Un interrupteur de `largeur` bits.
    pub fn entree(nom: &str, largeur: u32) -> Self {
        Self::brut(
            Genre::Entree,
            nom,
            vec![],
            vec![Broche::nouvelle("s", largeur)],
        )
    }

    /// Un bouton de démarrage : un interrupteur qui vaut 1 à la mise sous tension et retombe
    /// tout seul à 0 après `duree` unités de temps. Il sert à remettre un circuit à zéro.
    pub fn impulsion(nom: &str, duree: u64) -> Self {
        let mut e = Self::entree(nom, 1);
        e.param = duree.max(1);
        e
    }

    /// Un bouton poussoir : un interrupteur d'un bit qui vaut 1 tant qu'on appuie dessus.
    pub fn poussoir(nom: &str) -> Self {
        let mut e = Self::entree(nom, 1);
        e.aspect = Aspect::Poussoir;
        e
    }

    /// Un afficheur 7 segments : une lampe de 7 bits, bit 0 = segment a (en haut), puis b à f
    /// dans le sens des aiguilles d'une montre, bit 6 = g (au milieu).
    pub fn afficheur(nom: &str, legende: &str) -> Self {
        let mut e = Self::sortie(nom, legende, 7);
        e.aspect = Aspect::Segments;
        e
    }

    /// Vrai pour un afficheur 7 segments (une lampe de 7 ou 8 bits à l'aspect « segments »).
    pub fn est_afficheur(&self) -> bool {
        self.genre == Genre::Sortie
            && self.aspect == Aspect::Segments
            && matches!(self.broches_e.first().map(|b| b.largeur), Some(7 | 8))
    }

    /// Vrai pour un bouton poussoir (un interrupteur d'un bit à l'aspect « poussoir »).
    pub fn est_poussoir(&self) -> bool {
        self.genre == Genre::Entree
            && self.aspect == Aspect::Poussoir
            && self.param == 0
            && self.broches_s.first().map(|b| b.largeur) == Some(1)
    }

    /// Une lampe de `largeur` bits.
    pub fn sortie(nom: &str, legende: &str, largeur: u32) -> Self {
        let mut e = Self::brut(
            Genre::Sortie,
            nom,
            vec![Broche::nouvelle("e", largeur)],
            vec![],
        );
        e.legende = legende.to_owned();
        e
    }

    /// Une porte NAND.
    pub fn nand(nom: &str) -> Self {
        Self::brut(
            Genre::Nand,
            nom,
            vec![Broche::nouvelle("a", 1), Broche::nouvelle("b", 1)],
            vec![Broche::nouvelle("s", 1)],
        )
    }

    /// Une brique du modèle `modele`, avec les broches données.
    pub fn brique(nom: &str, modele: &str, entrees: Vec<Broche>, sorties: Vec<Broche>) -> Self {
        let mut e = Self::brut(Genre::Brique, nom, entrees, sorties);
        e.modele = modele.to_owned();
        e
    }

    /// Une horloge qui change de niveau toutes les `demi_periode` unités de temps.
    pub fn horloge(nom: &str, demi_periode: u64) -> Self {
        let mut e = Self::brut(Genre::Horloge, nom, vec![], vec![Broche::nouvelle("s", 1)]);
        e.param = demi_periode.max(1);
        e
    }

    /// Un écran : il affiche le caractère présent sur `d` quand `ecrit` monte.
    pub fn ecran(nom: &str) -> Self {
        Self::brut(
            Genre::Ecran,
            nom,
            vec![Broche::nouvelle("d", 8), Broche::nouvelle("ecrit", 1)],
            vec![],
        )
    }

    /// Un clavier : sa sortie porte le code de la touche enfoncée, 0 sinon.
    pub fn clavier(nom: &str) -> Self {
        let mut e = Self::brut(
            Genre::Clavier,
            nom,
            vec![],
            vec![Broche::nouvelle("touche", 8)],
        );
        e.param = DUREE_TOUCHE_DEFAUT;
        e
    }

    /// Un découpeur : une entrée de `largeur` bits, une sortie par plage `(premier, dernier)`.
    pub fn decoupe(nom: &str, largeur: u32, plages: &[(u32, u32)]) -> Self {
        let sorties = plages
            .iter()
            .map(|&(a, b)| {
                let nom = if a == b {
                    a.to_string()
                } else {
                    format!("{a}:{b}")
                };
                Broche::nouvelle(&nom, a.abs_diff(b) + 1)
            })
            .collect();
        let mut e = Self::brut(
            Genre::Decoupe,
            nom,
            vec![Broche::nouvelle("e", largeur)],
            sorties,
        );
        e.plages = plages.to_vec();
        e
    }

    /// Une jonction : une entrée par largeur donnée, une sortie qui les met bout à bout
    /// (la première entrée donne les bits de poids faible).
    pub fn jonction(nom: &str, largeurs: &[u32]) -> Self {
        let mut debut = 0;
        let entrees = largeurs
            .iter()
            .map(|&l| {
                let l = l.max(1);
                let nom = if l == 1 {
                    debut.to_string()
                } else {
                    format!("{debut}:{}", debut + l - 1)
                };
                debut += l;
                Broche::nouvelle(&nom, l)
            })
            .collect();
        Self::brut(
            Genre::Jonction,
            nom,
            entrees,
            vec![Broche::nouvelle("s", debut.max(1))],
        )
    }

    /// Vrai si l'élément a au moins une sortie.
    pub fn est_source(&self) -> bool {
        !self.broches_s.is_empty()
    }

    /// Largeur du corps de l'élément.
    pub fn largeur(&self) -> f32 {
        if self.est_afficheur() {
            return 72.0;
        }
        match self.genre {
            Genre::Nand => 80.0,
            Genre::Entree | Genre::Horloge => 62.0,
            Genre::Sortie => 52.0,
            Genre::Brique => {
                let long =
                    |b: &[Broche]| b.iter().map(|b| b.nom.chars().count()).max().unwrap_or(0);
                let titre = 36.0 + 8.6 * self.modele.chars().count() as f32;
                let broches = 54.0 + 7.0 * (long(&self.broches_e) + long(&self.broches_s)) as f32;
                (titre.max(broches).max(120.0) / 10.0).ceil().min(36.0) * 10.0
            }
            Genre::Ecran => 320.0,
            Genre::Clavier => 150.0,
            Genre::Decoupe | Genre::Jonction => 18.0,
        }
    }

    fn rangees(&self) -> f32 {
        self.broches_e.len().max(self.broches_s.len()).max(1) as f32
    }

    /// Hauteur du corps de l'élément.
    pub fn hauteur(&self) -> f32 {
        if self.est_afficheur() {
            return 108.0;
        }
        match self.genre {
            Genre::Nand => 64.0,
            Genre::Entree | Genre::Horloge | Genre::Sortie => 52.0,
            Genre::Brique => PAS_BROCHE * self.rangees() + 38.0,
            Genre::Ecran => 170.0,
            Genre::Clavier => 70.0,
            Genre::Decoupe | Genre::Jonction => PAS_BROCHE * self.rangees(),
        }
    }

    /// `-1` pour un élément retourné, `1` sinon : le signe des abscisses relatives au centre.
    pub fn sens(&self) -> f32 {
        if self.miroir {
            -1.0
        } else {
            1.0
        }
    }

    /// Le rectangle du corps `(gauche, haut, droite, bas)` pour un élément placé en `pos`.
    pub fn corps(&self, pos: Point) -> (f32, f32, f32, f32) {
        let (g, h, d, b) = self.corps_droit(pos);
        if self.miroir {
            // Retourné autour de son centre : la gauche et la droite s'échangent.
            (2.0 * pos.x - d, h, 2.0 * pos.x - g, b)
        } else {
            (g, h, d, b)
        }
    }

    /// Le corps relatif au centre, tel qu'il est dessiné avant d'être retourné.
    pub fn corps_local(&self) -> (f32, f32, f32, f32) {
        self.corps_droit(Point::default())
    }

    /// Le corps tel qu'il est sans retournement.
    fn corps_droit(&self, pos: Point) -> (f32, f32, f32, f32) {
        let (l, h) = (self.largeur(), self.hauteur());
        match self.genre {
            Genre::Nand => (pos.x - 45.0, pos.y - 32.0, pos.x + 35.0, pos.y + 32.0),
            // L'en-tête d'une brique est au-dessus de ses broches : le corps monte plus qu'il ne descend.
            Genre::Brique => {
                let demi = PAS_BROCHE * self.rangees() / 2.0;
                (
                    pos.x - l / 2.0,
                    pos.y - demi - 28.0,
                    pos.x + l / 2.0,
                    pos.y + demi + 10.0,
                )
            }
            _ => (
                pos.x - l / 2.0,
                pos.y - h / 2.0,
                pos.x + l / 2.0,
                pos.y + h / 2.0,
            ),
        }
    }

    /// Vrai si le point `p` est sur l'élément placé en `pos`.
    pub fn contient(&self, pos: Point, p: Point) -> bool {
        let (dx, dy) = ((p.x - pos.x) * self.sens(), p.y - pos.y);
        match self.genre {
            Genre::Nand => (-47.0..=37.0).contains(&dx) && dy.abs() <= 34.0,
            Genre::Entree | Genre::Horloge => dx.abs() <= 33.0 && dy.abs() <= 28.0,
            Genre::Sortie if self.est_afficheur() => dx.abs() <= 38.0 && dy.abs() <= 56.0,
            Genre::Sortie => dx.hypot(dy) <= 29.0,
            Genre::Decoupe | Genre::Jonction => {
                dx.abs() <= 14.0 && dy.abs() <= self.hauteur() / 2.0 + 2.0
            }
            _ => {
                let (g, h, d, b) = self.corps_droit(pos);
                (g..=d).contains(&(pos.x + dx)) && (h..=b).contains(&p.y)
            }
        }
    }

    fn decalage(n: usize, i: usize) -> f32 {
        (i as f32 - (n as f32 - 1.0) / 2.0) * PAS_BROCHE
    }

    /// Décalage vertical de l'entrée `i` par rapport au centre.
    pub fn decalage_entree(&self, i: usize) -> f32 {
        match (self.genre, i) {
            (Genre::Nand, 0) => -15.0,
            (Genre::Nand, _) => 15.0,
            _ => Self::decalage(self.broches_e.len(), i),
        }
    }

    /// Décalage vertical de la sortie `j` par rapport au centre.
    pub fn decalage_sortie(&self, j: usize) -> f32 {
        Self::decalage(self.broches_s.len(), j)
    }

    /// Position de l'entrée `i` (là où arrive le fil) pour un élément placé en `pos`.
    pub fn entree_en(&self, pos: Point, i: usize) -> Point {
        let x = match self.genre {
            Genre::Nand => -45.0,
            Genre::Sortie if !self.est_afficheur() => -28.0,
            _ => -self.largeur() / 2.0,
        };
        Point::nouveau(pos.x + x * self.sens(), pos.y + self.decalage_entree(i))
    }

    /// Position du point de la sortie `j` tel qu'il est dessiné sur l'élément.
    pub fn point_sortie(&self, pos: Point, j: usize) -> Point {
        let x = match self.genre {
            Genre::Nand => 40.0,
            _ => self.largeur() / 2.0,
        };
        Point::nouveau(pos.x + x * self.sens(), pos.y + self.decalage_sortie(j))
    }

    /// Position d'où part le fil de la sortie `j` pour un élément placé en `pos`.
    pub fn sortie_en(&self, pos: Point, j: usize) -> Point {
        let p = self.point_sortie(pos, j);
        match self.genre {
            Genre::Entree | Genre::Horloge => p,
            _ => Point::nouveau(p.x + 5.0 * self.sens(), p.y),
        }
    }
}

/// Désigne un fil par son arrivée : l'entrée `entree` de l'élément `puits`.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct Fil {
    /// Élément d'arrivée.
    pub puits: Id,
    /// Numéro de l'entrée sur cet élément.
    pub entree: usize,
}

/// Pourquoi un fil ne peut pas être posé.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Refus {
    /// La sortie ou l'entrée n'existe pas.
    Inconnu,
    /// La sortie et l'entrée n'ont pas le même nombre de bits.
    Largeur {
        /// Nombre de bits de la sortie.
        source: u32,
        /// Nombre de bits de l'entrée.
        entree: u32,
    },
}

/// Longueur du plus long chemin de portes d'un circuit.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Profondeur {
    /// Nombre de portes NAND sur le plus long chemin.
    Pas(u32),
    /// Le circuit contient une boucle.
    Boucle,
    /// Le circuit contient des briques : leur profondeur n'est pas comptée ici.
    Briques,
}

/// Ce qui détermine le comportement d'un élément, sans sa position ni son nom.
pub type Trait = (
    Id,
    Genre,
    String,
    u64,
    Vec<(u32, u32)>,
    Vec<u32>,
    Vec<u32>,
    Vec<Option<Source>>,
);

/// Le circuit en cours d'édition.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Document {
    /// Les éléments, dans l'ordre de création (donc par identifiant croissant).
    pub elements: Vec<Element>,
    prochain_id: Id,
    prochaine_porte: u32,
}

impl Default for Document {
    fn default() -> Self {
        Document {
            elements: Vec::new(),
            prochain_id: 1,
            prochaine_porte: 1,
        }
    }
}

impl Document {
    /// Document vide.
    pub fn nouveau() -> Self {
        Document::default()
    }

    /// Le demi-additionneur en cinq portes NAND, point de départ de la première quête.
    pub fn demi_additionneur() -> Self {
        let mut d = Document::nouveau();
        let a = d.ajouter(Genre::Entree, "a", "", Point::nouveau(61.0, 135.0));
        let b = d.ajouter(Genre::Entree, "b", "", Point::nouveau(61.0, 345.0));
        let g1 = d.ajouter_porte(Point::nouveau(260.0, 240.0));
        let g2 = d.ajouter_porte(Point::nouveau(460.0, 150.0));
        let g3 = d.ajouter_porte(Point::nouveau(460.0, 330.0));
        let g4 = d.ajouter_porte(Point::nouveau(660.0, 240.0));
        let g5 = d.ajouter_porte(Point::nouveau(460.0, 460.0));
        let s = d.ajouter(
            Genre::Sortie,
            "s",
            "s · somme",
            Point::nouveau(836.0, 240.0),
        );
        let c = d.ajouter(
            Genre::Sortie,
            "c",
            "c · retenue",
            Point::nouveau(836.0, 460.0),
        );
        for (puits, entree, source) in [
            (g1, 0, a),
            (g1, 1, b),
            (g2, 0, a),
            (g2, 1, g1),
            (g3, 0, g1),
            (g3, 1, b),
            (g4, 0, g2),
            (g4, 1, g3),
            (g5, 0, g1),
            (g5, 1, g1),
            (s, 0, g4),
            (c, 0, g5),
        ] {
            d.relier(Source::de(source), Fil { puits, entree });
        }
        d
    }

    /// Pose un élément tout fait en `pos` et renvoie son identifiant. Un nom vide est
    /// remplacé par un nom libre.
    pub fn poser(&mut self, mut element: Element, pos: Point) -> Id {
        let id = self.prochain_id;
        self.prochain_id += 1;
        element.id = id;
        element.pos = pos;
        element.entrees.resize(element.broches_e.len(), None);
        if element.nom.is_empty() {
            element.nom = match element.genre {
                Genre::Nand => loop {
                    let nom = format!("g{}", self.prochaine_porte);
                    self.prochaine_porte += 1;
                    if self.nomme(&nom).is_none() {
                        break nom;
                    }
                },
                Genre::Entree => self.nom_libre("e"),
                Genre::Sortie => self.nom_libre("s"),
                Genre::Horloge => self.nom_libre("h"),
                Genre::Ecran => self.nom_libre("ecran"),
                Genre::Clavier => self.nom_libre("clavier"),
                Genre::Decoupe => self.nom_libre("d"),
                Genre::Jonction => self.nom_libre("j"),
                Genre::Brique => {
                    let prefixe: String = element
                        .modele
                        .chars()
                        .filter(|c| c.is_alphanumeric())
                        .take(3)
                        .flat_map(char::to_lowercase)
                        .collect();
                    self.nom_libre(if prefixe.is_empty() { "b" } else { &prefixe })
                }
            };
        }
        self.elements.push(element);
        id
    }

    /// Le premier nom `prefixe1`, `prefixe2`… qu'aucun élément ne porte.
    pub fn nom_libre(&self, prefixe: &str) -> String {
        (1..)
            .map(|k| format!("{prefixe}{k}"))
            .find(|n| self.elements.iter().all(|e| e.nom != *n))
            .expect("il reste toujours un nom libre")
    }

    /// Ajoute un interrupteur, une lampe ou une porte d'un bit et renvoie son identifiant.
    pub fn ajouter(&mut self, genre: Genre, nom: &str, legende: &str, pos: Point) -> Id {
        let element = match genre {
            Genre::Entree => Element::entree(nom, 1),
            Genre::Sortie => Element::sortie(nom, legende, 1),
            _ => Element::nand(nom),
        };
        self.poser(element, pos)
    }

    /// Ajoute une porte NAND nommée `g1`, `g2`, … et renvoie son identifiant.
    pub fn ajouter_porte(&mut self, pos: Point) -> Id {
        self.poser(Element::nand(""), pos)
    }

    fn rang(&self, id: Id) -> Option<usize> {
        self.elements.binary_search_by_key(&id, |e| e.id).ok()
    }

    /// L'élément d'identifiant `id`.
    pub fn element(&self, id: Id) -> Option<&Element> {
        self.rang(id).map(|i| &self.elements[i])
    }

    /// L'élément d'identifiant `id`, modifiable.
    pub fn element_mut(&mut self, id: Id) -> Option<&mut Element> {
        self.rang(id).map(|i| &mut self.elements[i])
    }

    /// L'élément qui porte ce nom, s'il y en a un.
    pub fn nomme(&self, nom: &str) -> Option<&Element> {
        self.elements.iter().find(|e| e.nom == nom)
    }

    /// Les éléments qui ont une sortie, dans l'ordre de création.
    pub fn sources(&self) -> impl Iterator<Item = &Element> {
        self.elements.iter().filter(|e| e.est_source())
    }

    /// Toutes les sorties de tous les éléments, dans l'ordre d'affichage des signaux.
    pub fn prises(&self) -> impl Iterator<Item = Source> + '_ {
        self.elements
            .iter()
            .flat_map(|e| (0..e.broches_s.len()).map(|j| Source::nouvelle(e.id, j)))
    }

    /// Les broches qu'aurait une brique faite de ce plan : ses interrupteurs à gauche, ses
    /// lampes à droite, de haut en bas.
    pub fn interface(&self) -> (Vec<Broche>, Vec<Broche>) {
        let liste = |genre: Genre| {
            let mut e: Vec<&Element> = self.elements.iter().filter(|e| e.genre == genre).collect();
            e.sort_by(|a, b| {
                (a.pos.y, a.pos.x)
                    .partial_cmp(&(b.pos.y, b.pos.x))
                    .unwrap_or(std::cmp::Ordering::Equal)
                    .then(a.id.cmp(&b.id))
            });
            e.into_iter()
                .map(|e| {
                    let largeur = match genre {
                        Genre::Entree => e.broches_s[0].largeur,
                        _ => e.broches_e[0].largeur,
                    };
                    Broche::de_modele(&e.nom, largeur, e.id)
                })
                .collect::<Vec<_>>()
        };
        (liste(Genre::Entree), liste(Genre::Sortie))
    }

    /// Les portes NAND.
    pub fn portes(&self) -> impl Iterator<Item = &Element> {
        self.elements.iter().filter(|e| e.genre == Genre::Nand)
    }

    /// Nombre de bits d'une sortie, si elle existe.
    pub fn largeur_source(&self, source: Source) -> Option<u32> {
        self.element(source.id)?
            .broches_s
            .get(source.sortie)
            .map(|b| b.largeur)
    }

    /// Nombre de bits d'une entrée, si elle existe.
    pub fn largeur_entree(&self, fil: Fil) -> Option<u32> {
        self.element(fil.puits)?
            .broches_e
            .get(fil.entree)
            .map(|b| b.largeur)
    }

    /// La source branchée sur un fil, s'il existe.
    pub fn source_de(&self, fil: Fil) -> Option<Source> {
        self.element(fil.puits)?
            .entrees
            .get(fil.entree)
            .copied()
            .flatten()
    }

    /// Tous les fils, avec leur source.
    pub fn fils(&self) -> Vec<(Fil, Source)> {
        let mut liste = Vec::new();
        for e in &self.elements {
            for (i, s) in e.entrees.iter().enumerate() {
                if let Some(s) = s {
                    liste.push((
                        Fil {
                            puits: e.id,
                            entree: i,
                        },
                        *s,
                    ));
                }
            }
        }
        liste
    }

    /// Dit si la sortie `source` peut être branchée sur l'arrivée `fil`.
    ///
    /// Une lampe, une entrée de découpeur encore libre de ses sorties ou une jonction d'une
    /// seule entrée s'adaptent à la largeur de ce qu'on y branche : voir [`Self::relier`].
    pub fn verifier(&self, source: Source, fil: Fil) -> Result<(), Refus> {
        let (Some(ls), Some(le)) = (self.largeur_source(source), self.largeur_entree(fil)) else {
            return Err(Refus::Inconnu);
        };
        if ls == le || self.s_adapte(fil, ls) {
            Ok(())
        } else {
            Err(Refus::Largeur {
                source: ls,
                entree: le,
            })
        }
    }

    /// Vrai si l'arrivée `fil` peut changer de largeur pour accepter `largeur` bits.
    fn s_adapte(&self, fil: Fil, largeur: u32) -> bool {
        let Some(e) = self.element(fil.puits) else {
            return false;
        };
        match e.genre {
            // Une lampe affiche ce qu'on lui donne.
            Genre::Sortie => true,
            // Un découpeur s'élargit tant que ses plages tiennent dans le bus.
            Genre::Decoupe => e.plages.iter().all(|&(a, b)| a.max(b) < largeur),
            _ => false,
        }
    }

    /// Branche la sortie `source` sur l'arrivée `fil`, à la place de ce qui s'y trouvait.
    /// Renvoie `false` si la source ou l'arrivée n'existe pas, ou si les largeurs ne vont pas.
    pub fn relier(&mut self, source: Source, fil: Fil) -> bool {
        if self.verifier(source, fil).is_err() {
            return false;
        }
        let largeur = self.largeur_source(source).unwrap_or(1);
        let Some(e) = self.element_mut(fil.puits) else {
            return false;
        };
        e.broches_e[fil.entree].largeur = largeur;
        e.entrees[fil.entree] = Some(source);
        true
    }

    /// Débranche un fil. Renvoie la source qui y était branchée.
    pub fn debrancher(&mut self, fil: Fil) -> Option<Source> {
        self.element_mut(fil.puits)?
            .entrees
            .get_mut(fil.entree)?
            .take()
    }

    /// Retire un élément et tous les fils qui en partent. Renvoie `false` s'il n'existe pas.
    pub fn retirer(&mut self, id: Id) -> bool {
        let avant = self.elements.len();
        self.elements.retain(|e| e.id != id);
        for e in &mut self.elements {
            for s in &mut e.entrees {
                if s.is_some_and(|s| s.id == id) {
                    *s = None;
                }
            }
        }
        self.elements.len() != avant
    }

    /// Change le nombre de bits d'un interrupteur ou d'une lampe. Les fils qui n'ont plus la
    /// bonne largeur sont débranchés. Renvoie `false` si l'élément n'est ni l'un ni l'autre.
    pub fn changer_largeur(&mut self, id: Id, largeur: u32) -> bool {
        let largeur = largeur.clamp(1, 64);
        let Some(e) = self.element_mut(id) else {
            return false;
        };
        match e.genre {
            Genre::Entree => e.broches_s[0].largeur = largeur,
            Genre::Sortie => e.broches_e[0].largeur = largeur,
            _ => return false,
        }
        self.debrancher_les_fils_faux();
        true
    }

    /// Redonne ses plages à un découpeur : `largeur` bits en entrée, une sortie par plage.
    /// Les fils qui ne vont plus sont débranchés. Renvoie `false` si une plage dépasse.
    pub fn redefinir_decoupe(&mut self, id: Id, largeur: u32, plages: &[(u32, u32)]) -> bool {
        let largeur = largeur.clamp(1, 64);
        if plages.is_empty() || plages.iter().any(|&(a, b)| a.max(b) >= largeur) {
            return false;
        }
        let Some(e) = self.element_mut(id).filter(|e| e.genre == Genre::Decoupe) else {
            return false;
        };
        let neuf = Element::decoupe(&e.nom, largeur, plages);
        e.broches_e = neuf.broches_e;
        e.broches_s = neuf.broches_s;
        e.plages = neuf.plages;
        self.debrancher_les_fils_faux();
        true
    }

    /// Redonne ses entrées à une jonction : une par largeur. Les fils qui ne vont plus sont
    /// débranchés. Renvoie `false` si le total dépasse 64 bits.
    pub fn redefinir_jonction(&mut self, id: Id, largeurs: &[u32]) -> bool {
        if largeurs.is_empty() || largeurs.iter().sum::<u32>() > 64 || largeurs.contains(&0) {
            return false;
        }
        let Some(e) = self.element_mut(id).filter(|e| e.genre == Genre::Jonction) else {
            return false;
        };
        let neuf = Element::jonction(&e.nom, largeurs);
        e.entrees.resize(neuf.broches_e.len(), None);
        e.broches_e = neuf.broches_e;
        e.broches_s = neuf.broches_s;
        self.debrancher_les_fils_faux();
        true
    }

    /// Débranche les fils dont la source a disparu ou n'a plus la largeur de l'entrée.
    /// Renvoie le nombre de fils débranchés.
    pub fn debrancher_les_fils_faux(&mut self) -> usize {
        let faux: Vec<Fil> = self
            .fils()
            .into_iter()
            .filter(|(fil, source)| self.largeur_source(*source) != self.largeur_entree(*fil))
            .map(|(fil, _)| fil)
            .collect();
        for fil in &faux {
            self.debrancher(*fil);
        }
        faux.len()
    }

    /// Copie des éléments, avec les fils qui les relient entre eux et ceux qui leur arrivent
    /// du reste du circuit. Les copies sont décalées de `(dx, dy)`. Renvoie leurs identifiants.
    pub fn dupliquer(&mut self, ids: &[Id], dx: f32, dy: f32) -> Vec<Id> {
        let originaux: Vec<Element> = ids
            .iter()
            .filter_map(|id| self.element(*id).cloned())
            .collect();
        self.inserer_copies(originaux, dx, dy)
    }

    /// Pose des copies d'éléments (venus de ce plan ou d'un autre), décalées de `(dx, dy)`.
    /// Les fils qui les relient entre eux suivent les copies ; les autres restent tels quels
    /// (à retirer d'abord s'ils viennent d'un autre plan). Renvoie les identifiants des copies.
    pub fn inserer_copies(&mut self, originaux: Vec<Element>, dx: f32, dy: f32) -> Vec<Id> {
        let mut correspondance: Vec<(Id, Id)> = Vec::new();
        for mut copie in originaux {
            let (ancien, pos) = (copie.id, copie.pos);
            copie.nom = match copie.genre {
                Genre::Nand => String::new(),
                _ => {
                    let racine = copie.nom.trim_end_matches(|c: char| c.is_ascii_digit());
                    self.nom_libre(if racine.is_empty() { "e" } else { racine })
                }
            };
            let neuf = self.poser(copie, Point::nouveau(pos.x + dx, pos.y + dy));
            correspondance.push((ancien, neuf));
        }
        for &(_, neuf) in &correspondance {
            if let Some(e) = self.element_mut(neuf) {
                for s in e.entrees.iter_mut().flatten() {
                    if let Some(&(_, copie)) = correspondance.iter().find(|(a, _)| *a == s.id) {
                        s.id = copie;
                    }
                }
            }
        }
        correspondance.into_iter().map(|(_, n)| n).collect()
    }

    /// Nom du signal porté par une sortie : le nom de l'interrupteur, sinon celui de la
    /// première lampe branchée dessus, sinon `n` suivi du numéro de la porte, ou
    /// `brique.broche` pour une brique.
    pub fn nom_signal(&self, source: Source) -> String {
        let Some(e) = self.element(source.id) else {
            return "?".to_owned();
        };
        if matches!(e.genre, Genre::Entree | Genre::Horloge | Genre::Clavier) {
            return e.nom.clone();
        }
        if let Some(lampe) = self
            .elements
            .iter()
            .find(|o| o.genre == Genre::Sortie && o.entrees[0] == Some(source))
        {
            return lampe.nom.clone();
        }
        match e.genre {
            Genre::Nand => format!("n{}", e.nom.trim_start_matches('g')),
            Genre::Decoupe => {
                let amont = e.entrees[0].map_or("?".to_owned(), |s| self.nom_signal(s));
                let broche = e
                    .broches_s
                    .get(source.sortie)
                    .map_or("?", |b| b.nom.as_str());
                format!("{amont}[{broche}]")
            }
            Genre::Jonction => e.nom.clone(),
            _ => {
                let broche = e
                    .broches_s
                    .get(source.sortie)
                    .map_or("?", |b| b.nom.as_str());
                format!("{}.{broche}", e.nom)
            }
        }
    }

    /// Ce qui détermine le comportement du circuit : la liste des éléments et le câblage,
    /// sans les positions. Deux documents de même signature se simulent de la même façon.
    pub fn signature(&self) -> Vec<Trait> {
        self.elements
            .iter()
            .map(|e| {
                (
                    e.id,
                    e.genre,
                    e.modele.clone(),
                    e.param,
                    e.plages.clone(),
                    e.broches_e.iter().map(|b| b.largeur).collect(),
                    e.broches_s.iter().map(|b| b.largeur).collect(),
                    e.entrees.clone(),
                )
            })
            .collect()
    }

    /// Longueur du plus long chemin de portes entre une entrée et une sortie de porte.
    pub fn chemin_critique(&self) -> Profondeur {
        if self.elements.iter().any(|e| e.genre == Genre::Brique) {
            return Profondeur::Briques;
        }
        // Parcours en profondeur itératif : 0 = pas vu, 1 = en cours, 2 = fait.
        let n = self.elements.len();
        let mut etat = vec![0u8; n];
        let mut profondeur = vec![0u32; n];
        let mut max = 0;
        for depart in 0..n {
            if etat[depart] != 0 {
                continue;
            }
            let mut pile = vec![(depart, 0usize)];
            etat[depart] = 1;
            while let Some(&mut (i, ref mut k)) = pile.last_mut() {
                let e = &self.elements[i];
                if let Some(s) = e.entrees.get(*k) {
                    *k += 1;
                    let Some(j) = s.and_then(|s| self.rang(s.id)) else {
                        continue;
                    };
                    match etat[j] {
                        0 => {
                            etat[j] = 1;
                            pile.push((j, 0));
                        }
                        1 => return Profondeur::Boucle,
                        _ => {}
                    }
                } else {
                    let amont = e
                        .entrees
                        .iter()
                        .flatten()
                        .filter_map(|s| self.rang(s.id))
                        .map(|j| profondeur[j])
                        .max()
                        .unwrap_or(0);
                    profondeur[i] = amont + u32::from(e.genre == Genre::Nand);
                    max = max.max(profondeur[i]);
                    etat[i] = 2;
                    pile.pop();
                }
            }
        }
        Profondeur::Pas(max)
    }

    /// Remet les éléments dans l'ordre de leurs identifiants et les compteurs au-delà du plus
    /// grand : à appeler après avoir lu un document écrit par quelqu'un d'autre.
    pub fn reparer(&mut self) {
        self.elements.sort_by_key(|e| e.id);
        self.elements.dedup_by_key(|e| e.id);
        let max = self.elements.iter().map(|e| e.id).max().unwrap_or(0);
        self.prochain_id = self.prochain_id.max(max + 1);
        for e in &mut self.elements {
            e.entrees.resize(e.broches_e.len(), None);
        }
        let ids: Vec<Id> = self.elements.iter().map(|e| e.id).collect();
        for e in &mut self.elements {
            for s in &mut e.entrees {
                if s.is_some_and(|s| ids.binary_search(&s.id).is_err()) {
                    *s = None;
                }
            }
        }
        self.debrancher_les_fils_faux();
    }
}

/// Pile d'annulation : des clichés du document.
#[derive(Clone, Debug, Default)]
pub struct Historique {
    defaire: Vec<Document>,
    refaire: Vec<Document>,
    reference: Option<Document>,
}

impl Historique {
    /// Historique dont l'état de référence est `doc`.
    pub fn nouveau(doc: &Document) -> Self {
        Historique {
            defaire: Vec::new(),
            refaire: Vec::new(),
            reference: Some(doc.clone()),
        }
    }

    /// Enregistre l'état courant s'il diffère du dernier état enregistré.
    /// Renvoie `true` si quelque chose a été enregistré.
    pub fn noter(&mut self, doc: &Document, profondeur: usize) -> bool {
        if self.reference.as_ref() == Some(doc) {
            return false;
        }
        if let Some(avant) = self.reference.replace(doc.clone()) {
            self.defaire.push(avant);
        }
        if self.defaire.len() > profondeur.max(1) {
            let trop = self.defaire.len() - profondeur.max(1);
            self.defaire.drain(..trop);
        }
        self.refaire.clear();
        true
    }

    /// Vrai s'il y a quelque chose à annuler.
    pub fn peut_annuler(&self) -> bool {
        !self.defaire.is_empty()
    }

    /// Vrai s'il y a quelque chose à rétablir.
    pub fn peut_retablir(&self) -> bool {
        !self.refaire.is_empty()
    }

    /// Revient à l'état précédent, s'il y en a un.
    pub fn annuler(&mut self) -> Option<Document> {
        let avant = self.defaire.pop()?;
        if let Some(courant) = self.reference.replace(avant.clone()) {
            self.refaire.push(courant);
        }
        Some(avant)
    }

    /// Refait ce qui vient d'être annulé, s'il y a lieu.
    pub fn retablir(&mut self) -> Option<Document> {
        let apres = self.refaire.pop()?;
        if let Some(courant) = self.reference.replace(apres.clone()) {
            self.defaire.push(courant);
        }
        Some(apres)
    }
}

#[cfg(test)]
mod tests_miroir {
    use super::*;

    #[test]
    fn un_element_retourne_echange_ses_cotes() {
        let pos = Point::nouveau(100.0, 50.0);
        for mut e in [
            Element::nand("g"),
            Element::entree("a", 1),
            Element::sortie("s", "s", 1),
            Element::brique(
                "b",
                "ADD",
                vec![Broche::nouvelle("a", 1), Broche::nouvelle("b", 1)],
                vec![Broche::nouvelle("s", 1)],
            ),
        ] {
            let droit = e.clone();
            e.miroir = true;
            for i in 0..e.broches_e.len() {
                let (d, m) = (droit.entree_en(pos, i), e.entree_en(pos, i));
                assert_eq!((m.x - pos.x, m.y), (pos.x - d.x, d.y), "{:?}", e.genre);
            }
            for j in 0..e.broches_s.len() {
                let (d, m) = (droit.sortie_en(pos, j), e.sortie_en(pos, j));
                assert_eq!((m.x - pos.x, m.y), (pos.x - d.x, d.y), "{:?}", e.genre);
            }
            let (g, h, dr, b) = droit.corps(pos);
            assert_eq!(e.corps(pos), (2.0 * pos.x - dr, h, 2.0 * pos.x - g, b));
            // On le touche là où il est dessiné : au miroir de l'élément à l'endroit.
            for (dx, dy) in [(-44.0, 0.0), (34.0, 10.0), (0.0, 0.0), (-60.0, 0.0)] {
                assert_eq!(
                    e.contient(pos, Point::nouveau(pos.x - dx, pos.y + dy)),
                    droit.contient(pos, Point::nouveau(pos.x + dx, pos.y + dy)),
                    "{:?} {dx}",
                    e.genre
                );
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn le_demi_additionneur_a_cinq_portes_et_douze_fils() {
        let d = Document::demi_additionneur();
        assert_eq!(d.portes().count(), 5);
        assert_eq!(d.sources().count(), 7);
        assert_eq!(d.fils().len(), 12);
        assert_eq!(d.chemin_critique(), Profondeur::Pas(3));
    }

    #[test]
    fn les_signaux_portent_le_nom_de_leur_lampe_ou_de_leur_porte() {
        let d = Document::demi_additionneur();
        let noms: Vec<String> = d.prises().map(|s| d.nom_signal(s)).collect();
        assert_eq!(noms, ["a", "b", "n1", "n2", "n3", "s", "c"]);
    }

    #[test]
    fn retirer_une_porte_debranche_ses_fils() {
        let mut d = Document::demi_additionneur();
        let g1 = d.elements.iter().find(|e| e.nom == "g1").unwrap().id;
        assert!(d.retirer(g1));
        assert!(d.fils().iter().all(|(_, s)| s.id != g1));
        // Quatre fils partaient de g1, deux y arrivaient.
        assert_eq!(d.fils().len(), 12 - 4 - 2);
        assert!(!d.retirer(g1));
    }

    #[test]
    fn une_lampe_ne_peut_pas_piloter_un_fil() {
        let mut d = Document::demi_additionneur();
        let s = d.elements.iter().find(|e| e.nom == "s").unwrap().id;
        let g1 = d.elements.iter().find(|e| e.nom == "g1").unwrap().id;
        assert!(!d.relier(
            Source::de(s),
            Fil {
                puits: g1,
                entree: 0
            }
        ));
        assert!(!d.relier(
            Source::de(g1),
            Fil {
                puits: g1,
                entree: 7
            }
        ));
        assert!(!d.relier(
            Source::de(999),
            Fil {
                puits: g1,
                entree: 0
            }
        ));
    }

    #[test]
    fn une_boucle_est_detectee() {
        let mut d = Document::nouveau();
        let g = d.ajouter_porte(Point::default());
        assert_eq!(d.chemin_critique(), Profondeur::Pas(1));
        assert!(d.relier(
            Source::de(g),
            Fil {
                puits: g,
                entree: 0
            }
        ));
        assert_eq!(d.chemin_critique(), Profondeur::Boucle);
    }

    #[test]
    fn annuler_et_retablir_rendent_les_etats_exacts() {
        let mut d = Document::demi_additionneur();
        let depart = d.clone();
        let mut h = Historique::nouveau(&d);
        assert!(!h.noter(&d, 80), "rien n'a changé");
        let g = d.ajouter_porte(Point::nouveau(10.0, 10.0));
        assert!(h.noter(&d, 80));
        let avec_porte = d.clone();
        d.element_mut(g).unwrap().pos = Point::nouveau(50.0, 60.0);
        assert!(h.noter(&d, 80));
        let deplacee = d.clone();
        assert_eq!(h.annuler().unwrap(), avec_porte);
        assert_eq!(h.annuler().unwrap(), depart);
        assert!(h.annuler().is_none());
        assert_eq!(h.retablir().unwrap(), avec_porte);
        assert_eq!(h.retablir().unwrap(), deplacee);
        assert!(h.retablir().is_none());
        // Une nouvelle action après une annulation efface ce qui pouvait être rétabli.
        h.annuler();
        d = avec_porte.clone();
        d.retirer(g);
        assert!(h.noter(&d, 80));
        assert!(!h.peut_retablir());
    }

    #[test]
    fn l_historique_est_borne() {
        let mut d = Document::nouveau();
        let mut h = Historique::nouveau(&d);
        for i in 0..10 {
            d.ajouter_porte(Point::nouveau(i as f32, 0.0));
            h.noter(&d, 3);
        }
        let mut n = 0;
        while h.annuler().is_some() {
            n += 1;
        }
        assert_eq!(n, 3);
    }

    #[test]
    fn le_document_fait_l_aller_retour_en_json() {
        let d = Document::demi_additionneur();
        let texte = serde_json::to_string(&d).unwrap();
        assert_eq!(serde_json::from_str::<Document>(&texte).unwrap(), d);
    }
}
