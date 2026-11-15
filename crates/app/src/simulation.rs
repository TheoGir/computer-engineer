//! Le pont entre le circuit édité et le moteur `ce-moteur`.
//!
//! Le plan et ses briques sont mis à plat par [`crate::pont`], puis simulés par le moteur.
//! Toutes les valeurs affichées sortent du moteur ; ce module n'en calcule aucune lui-même,
//! sauf la cible d'une porte du plan (pour dire qu'elle est « en attente »), obtenue avec la
//! fonction NAND du moteur.
//!
//! Tout ce que l'application demande au moteur passe par la petite structure privée
//! `Moteur` de ce fichier : un moteur plus rapide se branche là, sans toucher au reste.
//!
//! Limite connue : le moteur ne sait pas reprendre un état sur un circuit modifié. Changer le
//! câblage reconstruit donc le simulateur, et les portes repartent de `X` comme à la mise sous
//! tension. Le temps affiché, le chronogramme et le journal, eux, continuent.

use std::collections::{BTreeMap, VecDeque};
use std::rc::Rc;

use ce_moteur::{valeur, NetId, Netlist, Simulateur, Valeur};

use crate::brique::Projet;
use crate::document::{Document, Fil, Genre, Id, Source, Trait};
use crate::pont::{self, Briques, Pont};
use ce_circuit::Circuit;

/// Nombre d'instants conservés pour le chronogramme et le retour dans le temps.
const MEMOIRE: usize = 4096;
/// Nombre total de valeurs conservées : sur un très grand plan, on garde moins d'instants.
const MEMOIRE_VALEURS: usize = 1_500_000;
/// Nombre de lignes conservées dans le journal.
const JOURNAL: usize = 200;
/// Nombre de lignes de journal écrites au plus pour un seul pas.
const JOURNAL_PAR_PAS: usize = 40;
/// Nombre de caractères conservés par écran.
const TEXTE_ECRAN: usize = 4000;

/// Ce qu'on lit sur une sortie d'un ou plusieurs bits.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub struct Lecture {
    /// Nombre de bits.
    pub largeur: u32,
    /// Les bits à 1.
    pub uns: u64,
    /// Les bits qui ne sont ni 0 ni 1 (indéterminés ou en l'air).
    pub flous: u64,
    /// Parmi eux, les bits en l'air.
    pub en_l_air: u64,
}

impl Lecture {
    fn masque(largeur: u32) -> u64 {
        if largeur >= 64 {
            u64::MAX
        } else {
            (1 << largeur) - 1
        }
    }

    /// Lecture d'une broche où rien n'est branché.
    pub fn debranchee(largeur: u32) -> Self {
        let m = Self::masque(largeur);
        Lecture {
            largeur,
            uns: 0,
            flous: m,
            en_l_air: m,
        }
    }

    /// Lecture d'un nombre connu.
    pub fn de_nombre(largeur: u32, n: u64) -> Self {
        Lecture {
            largeur,
            uns: n & Self::masque(largeur),
            flous: 0,
            en_l_air: 0,
        }
    }

    /// Lecture d'un seul bit.
    pub fn de_valeur(v: Valeur) -> Self {
        let mut l = Lecture {
            largeur: 1,
            ..Lecture::default()
        };
        l.poser(0, v);
        l
    }

    fn poser(&mut self, k: u32, v: Valeur) {
        let b = 1u64 << k;
        match v {
            Valeur::Zero => {}
            Valeur::Un => self.uns |= b,
            Valeur::Z => {
                self.flous |= b;
                self.en_l_air |= b;
            }
            _ => self.flous |= b,
        }
    }

    /// Le bit `k`.
    pub fn bit(&self, k: u32) -> Valeur {
        let b = 1u64 << k;
        if self.en_l_air & b != 0 {
            Valeur::Z
        } else if self.flous & b != 0 {
            Valeur::X
        } else {
            Valeur::de_bool(self.uns & b != 0)
        }
    }

    /// Résumé en une valeur : pour un fil simple, sa valeur ; pour un bus, `0` s'il vaut zéro,
    /// `1` s'il porte un nombre non nul, `X` si un bit est inconnu, `Z` si tout est en l'air.
    pub fn valeur(&self) -> Valeur {
        if self.largeur <= 1 {
            self.bit(0)
        } else if self.flous == 0 {
            Valeur::de_bool(self.uns != 0)
        } else if self.en_l_air == Self::masque(self.largeur) {
            Valeur::Z
        } else {
            Valeur::X
        }
    }

    /// Le nombre porté, si tous les bits sont connus.
    pub fn nombre(&self) -> Option<u64> {
        (self.flous == 0).then_some(self.uns)
    }

    /// Texte court : `0`, `1` ou `X` pour un fil ; le nombre en hexadécimal pour un bus.
    pub fn texte(&self) -> String {
        if self.largeur <= 1 {
            return symbole(self.bit(0)).to_string();
        }
        let chiffres = self.largeur.div_ceil(4) as usize;
        match self.nombre() {
            Some(n) => format!("{n:0chiffres$X}"),
            None => "X".repeat(chiffres),
        }
    }
}

/// Une ligne du journal : un signal a changé.
#[derive(Clone, Debug, PartialEq)]
pub struct Ligne {
    /// Instant du changement.
    pub t: u64,
    /// Nom du signal au moment du changement.
    pub nom: String,
    /// Valeur avant.
    pub de: Lecture,
    /// Valeur après.
    pub vers: Lecture,
    /// Vrai si c'est l'utilisateur qui a changé une entrée.
    pub entree: bool,
}

/// Les valeurs de toutes les sources à un instant.
#[derive(Clone, Debug, PartialEq)]
pub struct Instant {
    /// L'instant.
    pub t: u64,
    prises: Rc<[Source]>,
    lectures: Box<[Lecture]>,
}

impl Instant {
    /// Ce qu'on lisait sur une source à cet instant, si elle existait.
    pub fn lecture(&self, source: Source) -> Option<Lecture> {
        self.prises
            .binary_search(&source)
            .ok()
            .map(|i| self.lectures[i])
    }

    /// Valeur résumée d'une source à cet instant, si elle existait.
    pub fn valeur(&self, source: Source) -> Option<Valeur> {
        self.lecture(source).map(|l| l.valeur())
    }
}

/// Tout ce que l'application demande au moteur de simulation.
struct Moteur {
    sim: Simulateur,
    horloges: bool,
    portes: usize,
}

impl Moteur {
    fn nouveau(netlist: &Netlist) -> Option<Self> {
        Some(Moteur {
            sim: Simulateur::nouveau(netlist).ok()?,
            horloges: !netlist.horloges.is_empty(),
            portes: netlist.portes.len(),
        })
    }

    fn vide() -> Self {
        Self::nouveau(&Netlist::nouvelle()).expect("une netlist vide est valide")
    }

    fn valeur(&self, net: NetId) -> Valeur {
        self.sim.valeur(net)
    }

    fn fixer(&mut self, entree: usize, v: Valeur) {
        self.sim.fixer_entree(entree, v);
    }

    fn avancer(&mut self, duree: u64) {
        self.sim.avancer(duree);
    }

    /// Avance d'au plus `duree`, en s'arrêtant dès que le circuit est au repos. Renvoie le
    /// temps réellement écoulé.
    fn avancer_jusqu_au_repos(&mut self, duree: u64) -> u64 {
        if self.horloges {
            self.sim.avancer(duree);
            return duree;
        }
        let avant = self.sim.temps();
        self.sim.stabiliser(duree);
        self.sim.temps() - avant
    }

    fn au_repos(&self) -> bool {
        self.sim.est_stable() && !self.horloges
    }

    fn sonder(&mut self, net: NetId) -> u32 {
        self.sim.sonder(net)
    }

    fn releves(&mut self) -> Vec<(u64, u32, Valeur)> {
        self.sim
            .vider_journal()
            .into_iter()
            .map(|c| (c.t, c.sonde, c.valeur))
            .collect()
    }
}

/// À quoi sert une sonde posée pour un écran.
#[derive(Clone, Copy)]
enum Role {
    Donnee(Id, usize),
    Commande(Id),
}

/// L'état d'un écran : ce qu'il affiche et ce qu'il voit sur ses broches.
#[derive(Clone, Debug)]
struct Ecran {
    texte: String,
    donnee: [Valeur; 8],
    commande: Valeur,
}

/// L'état d'un clavier : les touches à venir et celle qui est enfoncée.
#[derive(Clone, Debug, Default)]
struct Clavier {
    file: VecDeque<u8>,
    /// Instant où la phase en cours (touche enfoncée, puis relâchée) se termine.
    fin: Option<u64>,
    enfonce: bool,
    duree: u64,
}

struct Cablage {
    moteur: Moteur,
    /// Le circuit mis à plat, gardé pour retrouver les signaux de l'intérieur des briques.
    circuit: Option<Circuit>,
    prises: Rc<[Source]>,
    nets: Vec<Vec<Option<NetId>>>,
    entree_de: BTreeMap<Id, (usize, u32)>,
    sondes: Vec<Role>,
    signature: Vec<Trait>,
    revision: u64,
    erreur: Option<String>,
}

impl Cablage {
    fn construire(
        doc: &Document,
        projet: &Projet,
        briques: &mut Briques,
        entrees: &BTreeMap<Id, u64>,
    ) -> Self {
        briques.suivre(projet);
        let (pont, erreur) = match Pont::construire(doc, projet, briques) {
            Ok(p) => (Some(p), None),
            Err(e) => (None, Some(e)),
        };
        let moteur = pont
            .as_ref()
            .and_then(|p| Moteur::nouveau(&p.circuit.netlist));
        let (mut moteur, circuit, prises, entree_de, erreur) = match (moteur, pont) {
            (Some(m), Some(p)) => (m, Some(p.circuit), p.prises, p.entrees, erreur),
            _ => (
                Moteur::vide(),
                None,
                doc.prises()
                    .map(|s| (s, vec![None; doc.largeur_source(s).unwrap_or(1) as usize]))
                    .collect(),
                BTreeMap::new(),
                erreur.or_else(|| Some("le circuit n'a pas pu être simulé".to_owned())),
            ),
        };
        let (prises, nets): (Vec<Source>, Vec<Vec<Option<NetId>>>) = prises.into_iter().unzip();
        let mut c = Cablage {
            moteur: Moteur::vide(),
            circuit,
            prises: prises.into(),
            nets,
            entree_de,
            sondes: Vec::new(),
            signature: doc.signature(),
            revision: projet.revision,
            erreur,
        };
        for (id, &(premiere, largeur)) in &c.entree_de {
            let v = entrees.get(id).copied().unwrap_or(0);
            for k in 0..largeur {
                moteur.fixer(premiere + k as usize, Valeur::de_bool(v >> k & 1 == 1));
            }
        }
        // Les écrans regardent leur commande et leurs huit bits de donnée.
        let mut sondes = Vec::new();
        for e in doc.elements.iter().filter(|e| e.genre == Genre::Ecran) {
            let nets = |i: usize| e.entrees[i].and_then(|s| c.nets_de(s));
            if let Some(Some(net)) = nets(1).map(|n| n.first().copied().flatten()) {
                moteur.sonder(net);
                sondes.push(Role::Commande(e.id));
            }
            if let Some(donnee) = nets(0) {
                for (k, net) in donnee.iter().enumerate().take(8) {
                    if let Some(net) = net {
                        moteur.sonder(*net);
                        sondes.push(Role::Donnee(e.id, k));
                    }
                }
            }
        }
        c.sondes = sondes;
        c.moteur = moteur;
        c
    }

    fn nets_de(&self, source: Source) -> Option<&Vec<Option<NetId>>> {
        self.prises
            .binary_search(&source)
            .ok()
            .map(|i| &self.nets[i])
    }

    fn lire(&self, nets: &[Option<NetId>]) -> Lecture {
        let mut l = Lecture {
            largeur: nets.len() as u32,
            ..Lecture::default()
        };
        for (k, net) in nets.iter().enumerate().take(64) {
            l.poser(k as u32, net.map_or(Valeur::Z, |n| self.moteur.valeur(n)));
        }
        l
    }

    fn lecture(&self, source: Source) -> Lecture {
        self.nets_de(source)
            .map_or(Lecture::debranchee(1), |n| self.lire(n))
    }

    fn toutes(&self) -> Box<[Lecture]> {
        self.nets.iter().map(|n| self.lire(n)).collect()
    }
}

/// Ce que montre une simulation quand on a ouvert une brique posée : le plan simulé reste
/// celui du dessus, et l'on regarde un de ses exemplaires de brique tourner dedans.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Regard {
    /// Le plan réellement simulé : `None` pour le plan principal, sinon le nom de la brique
    /// ouverte seule.
    pub lieu: Option<String>,
    /// Les briques posées traversées depuis ce plan, de la plus haute à celle qu'on regarde.
    /// Jamais vide.
    pub chemin: Vec<Id>,
}

/// La simulation en cours : le moteur, le temps affiché, le chronogramme et le journal.
pub struct Session {
    cablage: Cablage,
    /// L'exemplaire de brique regardé, s'il y en a un : les valeurs affichées sont alors
    /// celles de l'intérieur de cet exemplaire.
    regard: Option<Regard>,
    briques: Briques,
    /// Temps affiché. Il continue quand le câblage change.
    pub t: u64,
    /// Valeur de chaque interrupteur et de chaque clavier.
    pub entrees: BTreeMap<Id, u64>,
    /// Les derniers instants, du plus ancien au plus récent ; le dernier est l'instant courant.
    pub historique: VecDeque<Instant>,
    /// Les derniers changements, du plus ancien au plus récent.
    pub journal: VecDeque<Ligne>,
    /// Instant du dernier changement de chaque source.
    pub dernier: BTreeMap<Source, u64>,
    /// Éléments dont une sortie a changé au dernier pas.
    pub tirs: Vec<Id>,
    /// Sources dont la valeur a changé au dernier pas ou à la dernière action.
    pub changes: Vec<Source>,
    ecrans: BTreeMap<Id, Ecran>,
    claviers: BTreeMap<Id, Clavier>,
    /// Boutons de démarrage encore à 1, avec l'instant où ils retombent.
    impulsions: BTreeMap<Id, u64>,
}

impl Session {
    /// Démarre une simulation du plan `doc` du projet, interrupteurs aux valeurs données
    /// (0 par défaut).
    pub fn nouvelle(doc: &Document, projet: &Projet, entrees: &[(Id, u64)]) -> Self {
        let entrees: BTreeMap<Id, u64> = doc
            .elements
            .iter()
            .filter(|e| matches!(e.genre, Genre::Entree | Genre::Clavier))
            .map(|e| {
                let v = entrees
                    .iter()
                    .find(|(id, _)| *id == e.id)
                    .map_or(0, |x| x.1);
                let v = match e.genre {
                    Genre::Clavier => 0,
                    // Un bouton de démarrage vaut 1 à la mise sous tension.
                    _ if e.param > 0 => 1,
                    _ => v,
                };
                (e.id, v)
            })
            .collect();
        let mut briques = Briques::default();
        let cablage = Cablage::construire(doc, projet, &mut briques, &entrees);
        let mut s = Session {
            cablage,
            regard: None,
            briques,
            t: 0,
            entrees,
            historique: VecDeque::new(),
            journal: VecDeque::new(),
            dernier: BTreeMap::new(),
            tirs: Vec::new(),
            changes: Vec::new(),
            ecrans: BTreeMap::new(),
            claviers: BTreeMap::new(),
            impulsions: BTreeMap::new(),
        };
        s.armer_les_impulsions(doc);
        s.accorder_les_appareils(doc);
        s.relever();
        s.historique.push_back(s.instant());
        s
    }

    /// Une simulation d'un plan sans brique.
    pub fn du_plan(doc: &Document, entrees: &[(Id, u64)]) -> Self {
        Self::nouvelle(doc, &Projet::nouveau("", Document::nouveau()), entrees)
    }

    /// Le plan réellement simulé : `doc` lui-même, sauf quand on regarde un exemplaire.
    fn racine<'a>(&self, doc: &'a Document, projet: &'a Projet) -> &'a Document {
        match &self.regard {
            Some(r) => projet.plan(r.lieu.as_deref()).unwrap_or(doc),
            None => doc,
        }
    }

    /// L'exemplaire regardé, s'il y en a un.
    pub fn regard(&self) -> Option<&Regard> {
        self.regard.as_ref()
    }

    /// Vrai si l'on regarde l'intérieur d'une brique posée qui tourne dans le plan du dessus :
    /// ses interrupteurs sont alors pilotés par ce plan.
    pub fn exemplaire(&self) -> bool {
        self.regard.is_some()
    }

    /// Regarde le plan `doc` : celui que la simulation fait tourner (`regard` vide), ou celui
    /// d'une brique posée au bout de `regard.chemin` dans le plan simulé. La simulation
    /// continue sans repartir de zéro ; seul le chronogramme recommence.
    pub fn regarder(&mut self, doc: &Document, projet: &Projet, regard: Option<Regard>) {
        self.regard = regard.filter(|r| !r.chemin.is_empty());
        self.cadrer(doc, projet);
    }

    /// Fait correspondre les sorties du plan affiché aux signaux du circuit mis à plat.
    fn cadrer(&mut self, doc: &Document, projet: &Projet) {
        let chemin: &[Id] = self.regard.as_ref().map_or(&[], |r| &r.chemin);
        let prises = match &self.cablage.circuit {
            Some(circuit) => pont::prises(circuit, doc, projet, chemin),
            None => doc
                .prises()
                .map(|s| (s, vec![None; doc.largeur_source(s).unwrap_or(1) as usize]))
                .collect(),
        };
        let (prises, nets): (Vec<Source>, Vec<Vec<Option<NetId>>>) = prises.into_iter().unzip();
        if *self.cablage.prises == *prises && self.cablage.nets == nets {
            return;
        }
        self.cablage.prises = prises.into();
        self.cablage.nets = nets;
        // Les numéros d'éléments d'un plan à l'autre ne désignent pas les mêmes signaux.
        self.dernier.clear();
        self.tirs.clear();
        self.changes.clear();
        self.historique.clear();
        self.historique.push_back(self.instant());
    }

    /// Le clavier du plan simulé, s'il n'y en a qu'un : il reçoit les touches même quand on
    /// regarde l'intérieur d'une brique.
    pub fn seul_clavier(&self) -> Option<Id> {
        let mut claviers = self.claviers.keys();
        let premier = claviers.next().copied();
        premier.filter(|_| claviers.next().is_none())
    }

    fn instant(&self) -> Instant {
        Instant {
            t: self.t,
            prises: Rc::clone(&self.cablage.prises),
            lectures: self.cablage.toutes(),
        }
    }

    fn noter(&mut self, ligne: Ligne) {
        self.journal.push_back(ligne);
        while self.journal.len() > JOURNAL {
            self.journal.pop_front();
        }
    }

    fn memoire(&self) -> usize {
        (MEMOIRE_VALEURS / self.cablage.prises.len().max(1)).clamp(16, MEMOIRE)
    }

    /// Note quand retombera chaque bouton de démarrage encore à 1.
    fn armer_les_impulsions(&mut self, doc: &Document) {
        self.impulsions = doc
            .elements
            .iter()
            .filter(|e| e.genre == Genre::Entree && e.param > 0)
            .filter(|e| self.entrees.get(&e.id) == Some(&1))
            .map(|e| (e.id, self.t + e.param))
            .collect();
    }

    /// Crée l'état des écrans et des claviers du plan, en gardant celui des appareils déjà là.
    fn accorder_les_appareils(&mut self, doc: &Document) {
        self.ecrans.retain(|id, _| doc.element(*id).is_some());
        self.claviers.retain(|id, _| doc.element(*id).is_some());
        for e in &doc.elements {
            match e.genre {
                Genre::Ecran => {
                    let ecran = self.ecrans.entry(e.id).or_insert_with(|| Ecran {
                        texte: String::new(),
                        donnee: [Valeur::Z; 8],
                        commande: Valeur::Z,
                    });
                    // Le moteur est neuf : il redonnera l'état des broches.
                    ecran.donnee = [Valeur::Z; 8];
                    ecran.commande = Valeur::Z;
                }
                Genre::Clavier => {
                    let c = self.claviers.entry(e.id).or_default();
                    c.duree = e.param.max(1);
                }
                _ => {}
            }
        }
    }

    /// Lit ce que les sondes ont vu : un écran écrit un caractère quand sa commande monte.
    fn relever(&mut self) {
        if self.cablage.sondes.is_empty() {
            return;
        }
        let releves = self.cablage.moteur.releves();
        let mut i = 0;
        let mut fronts: Vec<Id> = Vec::new();
        while i < releves.len() {
            let t = releves[i].0;
            fronts.clear();
            while i < releves.len() && releves[i].0 == t {
                let (_, sonde, v) = releves[i];
                i += 1;
                match self.cablage.sondes.get(sonde as usize) {
                    Some(Role::Donnee(id, k)) => {
                        if let Some(e) = self.ecrans.get_mut(id) {
                            e.donnee[*k] = v;
                        }
                    }
                    Some(Role::Commande(id)) => {
                        if let Some(e) = self.ecrans.get_mut(id) {
                            if e.commande == Valeur::Zero && v == Valeur::Un {
                                fronts.push(*id);
                            }
                            e.commande = v;
                        }
                    }
                    None => {}
                }
            }
            for id in &fronts {
                if let Some(e) = self.ecrans.get_mut(id) {
                    let mut code = Some(0u8);
                    for (k, v) in e.donnee.iter().enumerate() {
                        code = match (code, v) {
                            (Some(c), Valeur::Un) => Some(c | 1 << k),
                            (Some(c), Valeur::Zero) => Some(c),
                            _ => None,
                        };
                    }
                    e.texte.push(caractere(code));
                    if e.texte.len() > TEXTE_ECRAN {
                        let debut = e
                            .texte
                            .char_indices()
                            .map(|(i, _)| i)
                            .find(|i| e.texte.len() - i <= TEXTE_ECRAN / 2)
                            .unwrap_or(0);
                        e.texte.drain(..debut);
                    }
                }
            }
        }
    }

    fn fixer(&mut self, entree: Id, v: u64) {
        if let Some(&(premiere, largeur)) = self.cablage.entree_de.get(&entree) {
            for k in 0..largeur {
                self.cablage
                    .moteur
                    .fixer(premiere + k as usize, Valeur::de_bool(v >> k & 1 == 1));
            }
        }
        self.entrees.insert(entree, v);
    }

    /// Fait vivre les claviers à l'instant courant : une touche reste enfoncée un moment,
    /// puis le clavier revient à 0 avant de présenter la suivante.
    fn servir_les_claviers(&mut self) {
        if self.claviers.is_empty() && self.impulsions.is_empty() {
            return;
        }
        let t = self.t;
        let mut ordres: Vec<(Id, u64)> = Vec::new();
        self.impulsions.retain(|id, fin| {
            let retombe = t >= *fin;
            if retombe {
                ordres.push((*id, 0));
            }
            !retombe
        });
        for (id, c) in &mut self.claviers {
            if c.fin.is_some_and(|fin| t >= fin) {
                if c.enfonce {
                    c.enfonce = false;
                    c.fin = Some(t + c.duree);
                    ordres.push((*id, 0));
                } else {
                    c.fin = None;
                }
            }
            if c.fin.is_none() {
                if let Some(code) = c.file.pop_front() {
                    c.enfonce = true;
                    c.fin = Some(t + c.duree);
                    ordres.push((*id, u64::from(code)));
                }
            }
        }
        for (id, v) in ordres {
            self.fixer(id, v);
        }
    }

    fn prochaine_echeance(&self) -> Option<u64> {
        self.claviers
            .values()
            .filter_map(|c| c.fin)
            .chain(self.impulsions.values().copied())
            .min()
    }

    /// Ce qu'on lit en ce moment sur la sortie d'un élément.
    pub fn lecture(&self, source: Source) -> Lecture {
        self.cablage.lecture(source)
    }

    /// Valeur actuelle, résumée, de la sortie d'un élément.
    pub fn valeur(&self, source: Source) -> Valeur {
        self.lecture(source).valeur()
    }

    /// Ce que lit une entrée d'élément (en l'air si rien n'y est branché).
    pub fn lecture_lue(&self, doc: &Document, fil: Fil) -> Lecture {
        match doc.source_de(fil) {
            Some(s) => self.lecture(s),
            None => Lecture::debranchee(doc.largeur_entree(fil).unwrap_or(1)),
        }
    }

    /// Valeur lue par une entrée d'élément (`Z` si rien n'y est branché).
    pub fn valeur_lue(&self, doc: &Document, fil: Fil) -> Valeur {
        doc.source_de(fil).map_or(Valeur::Z, |s| self.valeur(s))
    }

    /// Valeur que prendra la sortie d'une porte si ses entrées ne bougent plus.
    pub fn cible(&self, doc: &Document, porte: Id) -> Valeur {
        let lue = |i| {
            self.valeur_lue(
                doc,
                Fil {
                    puits: porte,
                    entree: i,
                },
            )
        };
        valeur::nand(lue(0), lue(1))
    }

    /// Les portes du plan dont la sortie n'a pas encore rejoint sa cible.
    pub fn en_attente(&self, doc: &Document) -> Vec<Id> {
        doc.portes()
            .filter(|g| self.cible(doc, g.id) != self.valeur(Source::de(g.id)))
            .map(|g| g.id)
            .collect()
    }

    /// Vrai si plus rien ne changera sans nouvelle action : aucune porte à réévaluer, pas
    /// d'horloge, pas de touche de clavier en attente.
    pub fn stable(&self) -> bool {
        self.cablage.moteur.au_repos()
            && self.impulsions.is_empty()
            && self
                .claviers
                .values()
                .all(|c| c.fin.is_none() && c.file.is_empty())
    }

    /// Vrai si le circuit contient une horloge : il ne sera jamais au repos.
    pub fn a_une_horloge(&self) -> bool {
        self.cablage.moteur.horloges
    }

    /// Nombre de portes NAND réellement simulées, briques ouvertes jusqu'au fond.
    pub fn nb_portes(&self) -> usize {
        self.cablage.moteur.portes
    }

    /// Pourquoi le circuit n'a pas pu être simulé, le cas échéant.
    pub fn erreur(&self) -> Option<&str> {
        self.cablage.erreur.as_deref()
    }

    /// À appeler après toute modification du plan ou des briques. Si le câblage a changé, le
    /// simulateur est reconstruit et renvoie `true`.
    pub fn suivre(&mut self, doc: &Document, projet: &Projet) -> bool {
        let affiche = doc;
        let doc = self.racine(affiche, projet);
        if self.cablage.revision == projet.revision && self.cablage.signature == doc.signature() {
            return false;
        }
        self.entrees.retain(|id, _| {
            doc.element(*id)
                .is_some_and(|e| matches!(e.genre, Genre::Entree | Genre::Clavier))
        });
        for e in &doc.elements {
            match e.genre {
                Genre::Entree => {
                    let v = self.entrees.entry(e.id).or_insert(0);
                    *v &= Lecture::masque(e.broches_s[0].largeur);
                }
                Genre::Clavier => {
                    self.entrees.entry(e.id).or_insert(0);
                }
                _ => {}
            }
        }
        self.impulsions.retain(|id, _| doc.element(*id).is_some());
        self.cablage = Cablage::construire(doc, projet, &mut self.briques, &self.entrees);
        self.accorder_les_appareils(doc);
        self.relever();
        if self.regard.is_some() {
            self.cadrer(affiche, projet);
        }
        self.dernier.retain(|s, _| affiche.element(s.id).is_some());
        let courant = self.instant();
        self.changes = courant.prises.to_vec();
        match self.historique.back_mut() {
            Some(i) => *i = courant,
            None => self.historique.push_back(courant),
        }
        self.tirs.clear();
        true
    }

    /// Avance d'une unité de temps.
    pub fn pas(&mut self, doc: &Document) {
        let avant = self.cablage.toutes();
        self.cablage.moteur.avancer(1);
        self.t += 1;
        self.relever();
        self.servir_les_claviers();
        let apres = self.instant();
        self.tirs.clear();
        self.changes.clear();
        let mut notes = 0;
        for (i, (de, vers)) in avant.iter().zip(apres.lectures.iter()).enumerate() {
            if de != vers {
                let source = apres.prises[i];
                if self.tirs.last() != Some(&source.id) {
                    self.tirs.push(source.id);
                }
                self.changes.push(source);
                self.dernier.insert(source, self.t);
                if notes < JOURNAL_PAR_PAS {
                    notes += 1;
                    self.noter(Ligne {
                        t: self.t,
                        nom: doc.nom_signal(source),
                        de: *de,
                        vers: *vers,
                        entree: false,
                    });
                }
            }
        }
        self.historique.push_back(apres);
        let memoire = self.memoire();
        while self.historique.len() > memoire {
            self.historique.pop_front();
        }
    }

    /// Avance d'au plus `duree` unités de temps d'un coup, sans s'arrêter à chaque instant :
    /// pour faire tourner un circuit à horloge. S'arrête dès que le circuit est au repos.
    /// Le chronogramme repart de l'instant d'arrivée. Renvoie le temps écoulé.
    pub fn avancer_vite(&mut self, duree: u64) -> u64 {
        let (debut, fin) = (self.t, self.t + duree);
        while self.t < fin && !self.stable() {
            let etape = self
                .prochaine_echeance()
                .map_or(fin, |e| e.clamp(self.t + 1, fin))
                - self.t;
            let fait = self.cablage.moteur.avancer_jusqu_au_repos(etape);
            self.t += fait;
            self.relever();
            self.servir_les_claviers();
            if fait < etape && self.prochaine_echeance().is_none() {
                break;
            }
            if fait < etape {
                // Au repos en attendant une touche ou la fin d'une impulsion : on y va.
                let reste = etape - fait;
                self.cablage.moteur.avancer(reste);
                self.t += reste;
                self.servir_les_claviers();
            }
        }
        let duree = self.t - debut;
        if duree > 0 {
            self.tirs.clear();
            self.changes.clear();
            self.historique.clear();
            self.historique.push_back(self.instant());
        }
        duree
    }

    fn apres_action(&mut self, doc: &Document, entree: Id, de: Lecture) {
        let source = Source::de(entree);
        self.dernier.insert(source, self.t);
        self.changes = vec![source];
        self.tirs.clear();
        let vers = self.lecture(source);
        self.noter(Ligne {
            t: self.t,
            nom: doc.nom_signal(source),
            de,
            vers,
            entree: true,
        });
        let courant = self.instant();
        if let Some(i) = self.historique.back_mut() {
            *i = courant;
        }
    }

    /// Règle un interrupteur sur une valeur, à l'instant courant.
    pub fn regler(&mut self, doc: &Document, entree: Id, v: u64) {
        let Some(e) = doc.element(entree).filter(|e| e.genre == Genre::Entree) else {
            return;
        };
        // Dans un exemplaire, les interrupteurs sont pilotés par le plan du dessus.
        if self.regard.is_some() || !self.cablage.entree_de.contains_key(&entree) {
            return;
        }
        let de = self.lecture(Source::de(entree));
        // Une action à la main prend le pas sur l'impulsion de démarrage.
        self.impulsions.remove(&entree);
        self.fixer(entree, v & Lecture::masque(e.broches_s[0].largeur));
        self.apres_action(doc, entree, de);
    }

    /// Bascule un interrupteur d'un bit ; sur un interrupteur de plusieurs bits, ajoute 1.
    pub fn basculer(&mut self, doc: &Document, entree: Id) {
        let v = self.entrees.get(&entree).copied().unwrap_or(0);
        let largeur = doc.largeur_source(Source::de(entree)).unwrap_or(1);
        self.regler(
            doc,
            entree,
            if largeur == 1 {
                v ^ 1
            } else {
                v.wrapping_add(1)
            },
        );
    }

    /// Bascule un seul bit d'un interrupteur.
    pub fn basculer_bit(&mut self, doc: &Document, entree: Id, k: u32) {
        let v = self.entrees.get(&entree).copied().unwrap_or(0);
        self.regler(doc, entree, v ^ (1 << k));
    }

    /// Tape une touche sur un clavier : son code sera présenté un moment, à son tour.
    pub fn taper(&mut self, clavier: Id, code: u8) {
        if let Some(c) = self.claviers.get_mut(&clavier) {
            if c.file.len() < 256 {
                c.file.push_back(code);
            }
        }
        self.servir_les_claviers();
    }

    /// Nombre de touches tapées qu'un clavier n'a pas encore présentées.
    pub fn touches_en_attente(&self, clavier: Id) -> usize {
        self.claviers.get(&clavier).map_or(0, |c| c.file.len())
    }

    /// Ce qu'un écran a affiché depuis la remise à zéro.
    pub fn ecran(&self, ecran: Id) -> &str {
        self.ecrans.get(&ecran).map_or("", |e| e.texte.as_str())
    }

    /// Revient à t = 0 : simulateur neuf, portes à `X`, interrupteurs conservés, écrans et
    /// claviers vidés.
    pub fn remettre_a_zero(&mut self, doc: &Document, projet: &Projet) {
        let affiche = doc;
        let doc = self.racine(affiche, projet);
        for (id, v) in &mut self.entrees {
            if self.claviers.contains_key(id) {
                *v = 0;
            } else if doc.element(*id).is_some_and(|e| e.param > 0) {
                *v = 1;
            }
        }
        self.cablage = Cablage::construire(doc, projet, &mut self.briques, &self.entrees);
        self.t = 0;
        self.armer_les_impulsions(doc);
        self.ecrans.clear();
        self.claviers.clear();
        self.accorder_les_appareils(doc);
        self.relever();
        if self.regard.is_some() {
            self.cadrer(affiche, projet);
        }
        self.historique.clear();
        self.journal.clear();
        self.dernier.clear();
        self.tirs.clear();
        self.changes.clear();
        self.historique.push_back(self.instant());
    }

    /// L'instant `t` du chronogramme, s'il est encore en mémoire.
    pub fn instant_a(&self, t: u64) -> Option<&Instant> {
        let premier = self.historique.front()?.t;
        self.historique
            .get(usize::try_from(t.checked_sub(premier)?).ok()?)
    }
}

/// Le caractère qu'un écran affiche pour un code ; `None` : un bit n'était pas connu.
fn caractere(code: Option<u8>) -> char {
    match code {
        Some(b'\n') => '\n',
        Some(c @ 32..=126) => char::from(c),
        Some(c @ 160..=255) => char::from(c),
        Some(_) => '·',
        None => '?',
    }
}

/// Symbole d'affichage d'une valeur : `0`, `1`, `X` ; la haute impédance se montre comme `X`.
pub fn symbole(v: Valeur) -> char {
    match v {
        Valeur::Zero => '0',
        Valeur::Un => '1',
        _ => 'X',
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::document::Point;

    fn vide() -> Projet {
        Projet::nouveau("", Document::nouveau())
    }

    fn id(doc: &Document, nom: &str) -> Id {
        doc.elements.iter().find(|e| e.nom == nom).unwrap().id
    }

    fn ligne(doc: &Document, i: &Instant) -> String {
        ["a", "b", "g1", "g2", "g3", "g5", "g4"]
            .iter()
            .map(|n| symbole(i.valeur(Source::de(id(doc, n))).unwrap()).to_ascii_lowercase())
            .collect()
    }

    /// La trace de référence du demi-additionneur, obtenue en pilotant directement le moteur
    /// (colonnes : a b n1 n2 n3 c s). La session doit la reproduire à l'identique.
    const TRACE: [&str; 13] = [
        "10xxxxx", "101x1xx", "101010x", "1010101", "1010101", "1010101", "1110101", "1100001",
        "1101111", "1101110", "1101110", "1101110", "1101110",
    ];

    #[test]
    fn la_session_reproduit_la_trace_du_moteur() {
        let doc = Document::demi_additionneur();
        let mut s = Session::du_plan(&doc, &[(id(&doc, "a"), 1)]);
        for _ in 0..6 {
            s.pas(&doc);
        }
        s.basculer(&doc, id(&doc, "b"));
        for _ in 0..6 {
            s.pas(&doc);
        }
        let trace: Vec<String> = s.historique.iter().map(|i| ligne(&doc, i)).collect();
        assert_eq!(trace, TRACE);
        assert!(s.stable());
        assert_eq!(s.t, 12);
    }

    #[test]
    fn les_portes_en_attente_sont_celles_qui_vont_changer() {
        let doc = Document::demi_additionneur();
        let mut s = Session::du_plan(&doc, &[(id(&doc, "a"), 1)]);
        assert_eq!(
            s.en_attente(&doc),
            vec![id(&doc, "g1"), id(&doc, "g3")],
            "à t = 0, seules les portes qui voient un 0 ont une cible connue"
        );
        for _ in 0..3 {
            assert!(!s.stable());
            s.pas(&doc);
        }
        assert!(s.stable());
        s.basculer(&doc, id(&doc, "b"));
        assert_eq!(s.en_attente(&doc), vec![id(&doc, "g1"), id(&doc, "g3")]);
        s.pas(&doc);
        assert_eq!(s.tirs, vec![id(&doc, "g1"), id(&doc, "g3")]);
    }

    #[test]
    fn le_journal_note_les_changements_avec_le_nom_du_signal() {
        let doc = Document::demi_additionneur();
        let mut s = Session::du_plan(&doc, &[(id(&doc, "a"), 1)]);
        for _ in 0..3 {
            s.pas(&doc);
        }
        let noms: Vec<(u64, &str)> = s.journal.iter().map(|l| (l.t, l.nom.as_str())).collect();
        assert_eq!(noms, [(1, "n1"), (1, "n3"), (2, "n2"), (2, "c"), (3, "s")]);
        s.basculer(&doc, id(&doc, "b"));
        let l = s.journal.back().unwrap();
        assert!(
            l.entree
                && l.nom == "b"
                && l.de.valeur() == Valeur::Zero
                && l.vers.valeur() == Valeur::Un
                && l.t == 3
        );
    }

    #[test]
    fn modifier_le_cablage_reconstruit_et_le_temps_continue() {
        let mut doc = Document::demi_additionneur();
        let mut s = Session::du_plan(&doc, &[(id(&doc, "a"), 1)]);
        for _ in 0..4 {
            s.pas(&doc);
        }
        // Déplacer un élément ne change rien pour le moteur.
        let g4 = id(&doc, "g4");
        doc.element_mut(g4).unwrap().pos = Point::nouveau(1.0, 2.0);
        assert!(!s.suivre(&doc, &vide()));
        assert_eq!(s.valeur(Source::de(g4)), Valeur::Un);
        // Ajouter une porte libre : reconstruction, tout repart de X, le temps continue.
        let g6 = doc.ajouter_porte(Point::default());
        assert!(s.suivre(&doc, &vide()));
        assert_eq!(s.t, 4);
        assert_eq!(s.valeur(Source::de(g4)), Valeur::X);
        for _ in 0..4 {
            s.pas(&doc);
        }
        assert_eq!(s.valeur(Source::de(g4)), Valeur::Un);
        assert_eq!(
            s.valeur(Source::de(g6)),
            Valeur::X,
            "une porte aux entrées libres reste indéterminée"
        );
        assert!(s.stable());
        // Une seule entrée branchée sur un 0 suffit à forcer la sortie à 1.
        assert!(doc.relier(
            Source::de(id(&doc, "b")),
            Fil {
                puits: g6,
                entree: 1
            }
        ));
        assert!(s.suivre(&doc, &vide()));
        for _ in 0..4 {
            s.pas(&doc);
        }
        assert_eq!(s.valeur(Source::de(g6)), Valeur::Un);
        assert_eq!(s.historique.back().unwrap().t, 12);
    }

    #[test]
    fn remettre_a_zero_garde_les_interrupteurs() {
        let doc = Document::demi_additionneur();
        let mut s = Session::du_plan(&doc, &[(id(&doc, "a"), 1)]);
        s.basculer(&doc, id(&doc, "b"));
        for _ in 0..5 {
            s.pas(&doc);
        }
        s.remettre_a_zero(&doc, &vide());
        assert_eq!(s.t, 0);
        assert_eq!(ligne(&doc, s.historique.back().unwrap()), "11xxxxx");
        assert!(s.journal.is_empty());
    }

    #[test]
    fn une_boucle_oscille_sans_jamais_se_stabiliser() {
        // Une NAND rebouclée sur elle-même, l'autre entrée à 1 : un inverseur en anneau.
        let mut doc = Document::nouveau();
        let a = doc.ajouter(Genre::Entree, "a", "", Point::default());
        let g = doc.ajouter_porte(Point::default());
        doc.relier(
            Source::de(a),
            Fil {
                puits: g,
                entree: 0,
            },
        );
        doc.relier(
            Source::de(g),
            Fil {
                puits: g,
                entree: 1,
            },
        );
        let mut s = Session::du_plan(&doc, &[]);
        // a = 0 force la sortie à 1, ce qui donne une valeur binaire à la boucle.
        s.pas(&doc);
        assert_eq!(s.valeur(Source::de(g)), Valeur::Un);
        s.basculer(&doc, a);
        let mut vues = Vec::new();
        for _ in 0..6 {
            s.pas(&doc);
            vues.push(symbole(s.valeur(Source::de(g))));
            assert!(!s.stable());
        }
        assert_eq!(vues, ['0', '1', '0', '1', '0', '1']);
    }
}
