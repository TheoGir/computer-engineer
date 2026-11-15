//! L'atelier : tout l'état logique de l'application, sans rien d'écran.
//!
//! Le dessin lit l'atelier et lui demande des actions (`pas`, `basculer`, `relier`…). Rien ici
//! ne dépend d'egui : tout se teste sans fenêtre.

use std::time::{Duration, Instant};

use ce_moteur::Valeur;

use crate::apparence::Apparence;
use crate::brique::{self, Projet};
use crate::document::{Document, Element, Fil, Genre, Historique, Id, Point, Source};
use crate::exemples;
use crate::parcours::Parcours;
use crate::quete::{Fiche, Quete, PARCOURS};
use crate::savoir;
use crate::simulation::{Lecture, Regard, Session};

/// Ce qui est sélectionné sur le plan.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Selection {
    /// Un ou plusieurs éléments (jamais zéro).
    Elements(Vec<Id>),
    /// Un fil.
    Fil(Fil),
}

/// Les panneaux affichés ou cachés.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Volets {
    /// Panneau des briques.
    pub briques: bool,
    /// Panneau de la quête.
    pub quete: bool,
    /// Panneau des infos.
    pub infos: bool,
    /// Chronogramme et journal.
    pub temps: bool,
    /// Étiquettes des signaux sur le plan.
    pub etiquettes: bool,
}

impl Default for Volets {
    fn default() -> Self {
        Volets {
            briques: true,
            quete: true,
            infos: true,
            temps: true,
            etiquettes: true,
        }
    }
}

/// Un message passager affiché au-dessus du plan.
#[derive(Clone, Debug, PartialEq)]
pub struct Message {
    /// Numéro unique, pour suivre son animation.
    pub id: u64,
    /// Le texte.
    pub texte: String,
    /// Instant d'apparition, à l'horloge de l'atelier.
    pub ne: f64,
    /// Durée d'affichage en secondes ; `None` : jusqu'à ce qu'on le ferme.
    pub duree: Option<f32>,
    /// Une action proposée à côté du texte, en plus de « Plus tard ».
    pub action: Option<Proposition>,
}

/// Une action qu'un message propose.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Proposition {
    /// Télécharger et installer la nouvelle version.
    MettreAJour,
    /// Relancer l'application sur la version installée.
    Redemarrer,
    /// Ramener de la corbeille le projet qu'on vient d'y mettre.
    Restaurer,
    /// Ouvrir le dossier des rapports de plantage.
    VoirLeRapport,
}

impl Proposition {
    /// Le libellé du lien.
    pub fn libelle(self) -> &'static str {
        match self {
            Proposition::MettreAJour => "Mettre à jour",
            Proposition::Redemarrer => "Redémarrer",
            Proposition::Restaurer => "Annuler",
            Proposition::VoirLeRapport => "Voir le rapport",
        }
    }
}

/// Une vitesse écrite pour l'utilisateur : « 1,7 pas/s », « 4 pas/s », « 33 k pas/s ».
pub fn libelle_pas_par_seconde(par_seconde: f32) -> String {
    let arrondi = (par_seconde * 10.0).round() / 10.0;
    if arrondi >= 10_000.0 {
        format!("{} k pas/s", (arrondi / 1000.0).round() as i64)
    } else if (arrondi - arrondi.round()).abs() < 0.05 {
        format!("{} pas/s", arrondi.round() as i64)
    } else {
        format!("{arrondi:.1} pas/s").replace('.', ",")
    }
}

/// L'état de la simulation, résumé pour la pastille de la barre du haut.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Etat {
    /// On regarde un instant passé.
    Passe(u64),
    /// Plus rien ne bouge.
    Stable,
    /// Le courant se propage.
    EnCours,
    /// En pause alors que des portes attendent.
    EnPause,
}

/// Comment on est arrivé sur le plan affiché. Le plan s'en sert pour animer le passage ; la
/// simulation n'en dépend pas.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum Trajet {
    /// Un autre projet, ou un plan sans rapport avec le précédent.
    #[default]
    Autre,
    /// On est entré dans cette brique posée.
    Entree(Id),
    /// On a ouvert une brique seule, depuis la liste des briques.
    Dedans,
    /// On est remonté ; l'exemplaire posé d'où l'on sort, si on y était entré par lui.
    Sortie(Option<Id>),
}

/// Tout l'état logique de l'application.
pub struct Atelier {
    /// Tout ce qui est construit : le plan principal et les briques. Toujours à jour.
    pub projet: Projet,
    /// Les briques ouvertes, de la plus haute à celle dont on voit le plan. Vide : on est
    /// sur le plan principal.
    pub chemin: Vec<String>,
    /// Le plan affiché : copie de travail du plan principal ou de celui d'une brique.
    pub doc: Document,
    /// Annuler et rétablir, pour le plan affiché.
    pub historique: Historique,
    /// La simulation du plan affiché.
    pub session: Session,
    /// La quête en cours, s'il y en a une.
    pub quete: Option<Quete>,
    /// Les quêtes réussies et les briques gagnées. Survit au changement de projet.
    pub parcours: Parcours,
    /// La sélection.
    pub selection: Option<Selection>,
    /// Vrai si la simulation avance toute seule.
    pub lecture: bool,
    /// Vitesse choisie : d'abord les vitesses lentes de l'apparence, puis les rapides.
    pub vitesse: usize,
    /// Instant passé que l'on regarde, s'il y en a un.
    pub vue_t: Option<u64>,
    /// Panneaux affichés.
    pub volets: Volets,
    /// Messages passagers.
    pub messages: Vec<Message>,
    /// Vrai tant que personne n'a cliqué sur un interrupteur : ils clignotent pour inviter.
    pub invite: bool,
    /// Secondes écoulées depuis le lancement.
    pub horloge: f64,
    /// Dernier cas de la quête validé, avec l'instant où il l'a été.
    pub cas_neuf: Option<(Vec<bool>, f64)>,
    /// L'instant de la dernière quête réussie : le plan la fête.
    pub fete: Option<f64>,
    /// Un chapitre ou une branche vient d'être terminé : l'instant, le titre du bandeau et sa
    /// phrase. Le plan fait une grande fête.
    pub grande_fete: Option<(f64, String, String)>,
    /// Comment on est arrivé sur le plan affiché.
    pub trajet: Trajet,
    /// Augmente à chaque changement du projet : sert à savoir quand enregistrer.
    pub version: u64,
    /// En simulation rapide : nombre de pas réellement simulés par seconde, lissé.
    pub debit: f32,
    /// Augmente à chaque changement de plan affiché : le plan sait qu'il doit se recadrer.
    pub deplacements: u64,
    /// Pour chaque brique du chemin : l'exemplaire posé par lequel on l'a ouverte (on la voit
    /// alors tourner dans le plan du dessus), ou `None` si on l'a ouverte seule, depuis la
    /// liste des briques.
    pub exemplaires: Vec<Option<Id>>,
    /// Pour chaque niveau quitté : sa simulation, sauf si le niveau suivant la regarde encore
    /// (exemplaire), et son historique.
    parents: Vec<(Option<Session>, Historique)>,
    /// Le message qui présente le plan ouvert : il s'en va quand on change de plan.
    message_du_lieu: Option<u64>,
    reste: f32,
    dette: f64,
    prochain_message: u64,
}

/// L'astuce qui accompagne le plan de démonstration du lancement.
pub const ASTUCE_DU_PLAN: &str =
    "Glisse les portes, tire un fil d'un point à un autre, molette pour zoomer.";

impl Atelier {
    /// L'atelier au lancement : le demi-additionneur, `a` à 1, lecture en marche.
    pub fn nouveau(a: &Apparence) -> Self {
        let mut projet = Projet::nouveau("Premier circuit", Document::demi_additionneur());
        projet.quete = Some(Quete::demi_additionneur().cle.to_owned());
        let mut atelier = Self::avec(projet, a);
        atelier.dire_longtemps(ASTUCE_DU_PLAN);
        atelier
    }

    /// Un atelier ouvert sur ce projet.
    pub fn avec(projet: Projet, a: &Apparence) -> Self {
        let doc = projet.principal.clone();
        let quete = projet.quete.as_deref().and_then(Quete::de_cle);
        let session = Self::demarrer(&doc, &projet, quete.is_some());
        Atelier {
            historique: Historique::nouveau(&doc),
            doc,
            session,
            quete,
            parcours: Parcours::default(),
            projet,
            chemin: Vec::new(),
            selection: None,
            lecture: true,
            vitesse: a
                .mouvement
                .vitesse_initiale
                .min(a.mouvement.vitesses.len().saturating_sub(1)),
            vue_t: None,
            volets: Volets::default(),
            messages: Vec::new(),
            invite: true,
            horloge: 0.0,
            cas_neuf: None,
            fete: None,
            grande_fete: None,
            trajet: Trajet::Autre,
            version: 0,
            debit: 0.0,
            deplacements: 0,
            exemplaires: Vec::new(),
            parents: Vec::new(),
            message_du_lieu: None,
            reste: 0.0,
            dette: 0.0,
            prochain_message: 0,
        }
    }

    /// Démarre la simulation d'un plan. Avec une quête, le premier interrupteur part à 1 pour
    /// qu'il se passe tout de suite quelque chose.
    fn demarrer(doc: &Document, projet: &Projet, quete: bool) -> Session {
        let premier: Vec<(Id, u64)> = doc
            .elements
            .iter()
            .find(|e| quete && e.genre == Genre::Entree)
            .map(|e| (e.id, 1))
            .into_iter()
            .collect();
        Session::nouvelle(doc, projet, &premier)
    }

    /// Remplace tout le projet par un autre.
    pub fn ouvrir(&mut self, projet: Projet, a: &Apparence) {
        let (volets, horloge, prochain) = (self.volets, self.horloge, self.prochain_message);
        let version = self.version + 1;
        let deplacements = self.deplacements + 1;
        let parcours = std::mem::take(&mut self.parcours);
        *self = Self::avec(projet, a);
        self.parcours = parcours;
        self.volets = volets;
        self.horloge = horloge;
        // Les messages du projet précédent ne le concernent plus : ils ne sont pas repris.
        self.prochain_message = prochain;
        self.version = version;
        self.deplacements = deplacements;
        self.invite = false;
        if self.session.a_une_horloge() {
            // Un circuit à horloge se regarde tourner, pas porte par porte.
            self.vitesse = a.mouvement.vitesses.len()
                + a.mouvement.vitesses_rapides.len().saturating_sub(1).min(2);
        }
    }

    /// Ouvre l'ordinateur miniature sous le nom `nom`, programmé avec ce texte assembleur.
    pub fn ouvrir_ordinateur(&mut self, nom: &str, programme: &str, a: &Apparence) -> bool {
        match exemples::ordinateur(nom, programme) {
            Ok(projet) => {
                self.ouvrir(projet, a);
                true
            }
            Err(e) => {
                self.dire(&format!("L'ordinateur n'a pas pu être ouvert : {e}"));
                false
            }
        }
    }

    /// Ouvre un des projets tout faits, sous le nom `nom`.
    pub fn ouvrir_exemple(
        &mut self,
        exemple: &exemples::Exemple,
        nom: &str,
        a: &Apparence,
    ) -> bool {
        let ouvert = self.ouvrir_ordinateur(nom, exemple.programme, a);
        if ouvert {
            self.dire_longtemps(exemple.accueil);
        }
        ouvert
    }

    /// La brique dont on voit le plan, ou `None` sur le plan principal.
    pub fn lieu(&self) -> Option<&str> {
        self.chemin.last().map(String::as_str)
    }

    /// Ce que la simulation doit montrer au niveau affiché : l'exemplaire au bout de la suite
    /// de briques posées ouvertes depuis le dernier plan simulé.
    fn regard(&self) -> Option<Regard> {
        let debut = self
            .exemplaires
            .iter()
            .rposition(Option::is_none)
            .map_or(0, |i| i + 1);
        let chemin: Vec<Id> = self.exemplaires[debut..]
            .iter()
            .flatten()
            .copied()
            .collect();
        (!chemin.is_empty()).then(|| Regard {
            lieu: debut.checked_sub(1).map(|i| self.chemin[i].clone()),
            chemin,
        })
    }

    /// Affiche le plan du bout du chemin. `session` remplace la simulation en cours ; `None`
    /// la garde (on regarde un exemplaire de la même simulation).
    fn charger_le_lieu(&mut self, session: Option<Session>) {
        let lieu = self.chemin.last().cloned();
        self.doc = self
            .projet
            .plan(lieu.as_deref())
            .cloned()
            .unwrap_or_default();
        if let Some(s) = session {
            self.session = s;
        }
        let regard = self.regard();
        self.session.regarder(&self.doc, &self.projet, regard);
        // Les briques ont pu changer pendant qu'on était ailleurs.
        self.session.suivre(&self.doc, &self.projet);
        self.historique = Historique::nouveau(&self.doc);
        self.selection = None;
        self.vue_t = None;
        self.reste = 0.0;
        self.dette = 0.0;
        self.cas_neuf = None;
        self.fete = None;
        self.grande_fete = None;
        self.trajet = Trajet::Autre;
        self.deplacements += 1;
        if let Some(id) = self.message_du_lieu.take() {
            self.fermer_message(id);
        }
    }

    /// Présente le plan qu'on vient d'ouvrir, à la place de la présentation du précédent.
    fn presenter_le_lieu(&mut self, texte: &str) {
        self.dire(texte);
        self.message_du_lieu = Some(self.prochain_message);
    }

    /// Ouvre la brique posée `id` : on voit son plan, avec les valeurs de cet exemplaire-là
    /// pendant que le plan du dessus continue de tourner. Renvoie `false` si ce n'est pas une
    /// brique.
    pub fn entrer(&mut self, id: Id) -> bool {
        let Some(nom) = self
            .doc
            .element(id)
            .filter(|e| e.genre == Genre::Brique)
            .map(|e| e.modele.clone())
        else {
            return false;
        };
        if self.session.erreur().is_some() {
            // Le plan du dessus ne tourne pas : on essaie la brique seule.
            return self.entrer_dans(&nom);
        }
        if self.projet.modele(&nom).is_none() || self.chemin.contains(&nom) {
            return false;
        }
        let historique = std::mem::replace(&mut self.historique, Historique::nouveau(&self.doc));
        self.parents.push((None, historique));
        self.chemin.push(nom.clone());
        self.exemplaires.push(Some(id));
        self.charger_le_lieu(None);
        self.trajet = Trajet::Entree(id);
        if self.doc.elements.iter().any(|e| e.genre == Genre::Entree) {
            self.presenter_le_lieu(&format!(
                "Tu vois cet exemplaire de « {nom} » tourner dans le plan du dessus : ses entrées viennent de là-haut. Pour l'essayer seule, ouvre-la depuis Mes briques."
            ));
        }
        true
    }

    /// Ouvre le plan du modèle `nom`, simulé seul : ses interrupteurs le pilotent.
    pub fn entrer_dans(&mut self, nom: &str) -> bool {
        if self.projet.modele(nom).is_none() || self.chemin.iter().any(|n| n == nom) {
            return false;
        }
        let historique = std::mem::replace(&mut self.historique, Historique::nouveau(&self.doc));
        let session = std::mem::replace(
            &mut self.session,
            Session::du_plan(&Document::nouveau(), &[]),
        );
        self.parents.push((Some(session), historique));
        self.chemin.push(nom.to_owned());
        self.exemplaires.push(None);
        let doc = self.projet.plan(Some(nom)).cloned().unwrap_or_default();
        let session = Self::demarrer(&doc, &self.projet, false);
        self.charger_le_lieu(Some(session));
        self.trajet = Trajet::Dedans;
        if self.doc.elements.iter().any(|e| e.genre == Genre::Entree) {
            self.presenter_le_lieu(&format!(
                "Tu essaies « {nom} » seule : ses interrupteurs remplacent ce qui la pilote dans le plan du dessus. Change-les pour voir comment elle réagit."
            ));
        }
        true
    }

    /// Le nom de chaque étape du chemin, pour le fil d'Ariane : la brique, précédée du nom de
    /// l'exemplaire quand on l'a ouverte par une brique posée (« cpu · PROCESSEUR »).
    pub fn etapes_du_chemin(&self) -> Vec<String> {
        self.chemin
            .iter()
            .enumerate()
            .map(|(i, modele)| {
                let parent = if i == 0 {
                    Some(&self.projet.principal)
                } else {
                    self.projet.plan(Some(&self.chemin[i - 1]))
                };
                let exemplaire = self.exemplaires[i]
                    .and_then(|id| parent?.element(id))
                    .map(|e| e.nom.as_str())
                    .filter(|n| !n.is_empty() && n != modele);
                match exemplaire {
                    Some(nom) => format!("{nom} · {modele}"),
                    None => modele.clone(),
                }
            })
            .collect()
    }

    /// Revient au niveau `niveau` du chemin (0 : le plan principal).
    pub fn remonter(&mut self, niveau: usize) -> bool {
        if niveau >= self.chemin.len() {
            return false;
        }
        // La brique du plan où l'on revient, par laquelle on était descendu.
        let exemplaire = self.exemplaires.get(niveau).copied().flatten();
        self.chemin.truncate(niveau);
        self.exemplaires.truncate(niveau);
        // La simulation de ce niveau est la première gardée à partir de lui ; s'il n'y en a
        // aucune, c'est celle qui tourne en ce moment.
        let session = self.parents.drain(niveau..).find_map(|(s, _)| s);
        self.charger_le_lieu(session);
        self.trajet = Trajet::Sortie(exemplaire);
        true
    }

    /// Referme la brique ouverte.
    pub fn sortir(&mut self) -> bool {
        !self.chemin.is_empty() && self.remonter(self.chemin.len() - 1)
    }

    fn lentes(a: &Apparence) -> usize {
        a.mouvement.vitesses.len()
    }

    fn nb_vitesses(a: &Apparence) -> usize {
        (Self::lentes(a) + a.mouvement.vitesses_rapides.len()).max(1)
    }

    /// En vitesse rapide : le nombre de pas par seconde demandé (0 : le plus possible).
    pub fn rapide(&self, a: &Apparence) -> Option<f32> {
        self.vitesse
            .checked_sub(Self::lentes(a))
            .and_then(|i| a.mouvement.vitesses_rapides.get(i).copied())
    }

    /// Durée d'un pas de simulation à la vitesse choisie, en secondes.
    pub fn duree_pas(&self, a: &Apparence) -> f32 {
        let v = &a.mouvement.vitesses;
        match self.rapide(a) {
            Some(debit) if debit > 0.0 => 1.0 / debit,
            Some(_) => 1e-6,
            None => v
                .get(self.vitesse.min(v.len().saturating_sub(1)))
                .copied()
                .unwrap_or(0.6),
        }
    }

    /// La vitesse choisie, écrite pour la barre du haut (« 1,7 pas/s »).
    pub fn libelle_vitesse(&self, a: &Apparence) -> String {
        if self.rapide(a) == Some(0.0) {
            return "max".to_owned();
        }
        libelle_pas_par_seconde(1.0 / self.duree_pas(a))
    }

    /// Passe à la vitesse suivante.
    pub fn changer_vitesse(&mut self, a: &Apparence) {
        self.vitesse = (self.vitesse + 1) % Self::nb_vitesses(a);
        self.reste = 0.0;
        self.dette = 0.0;
    }

    /// Fait passer `dt` secondes sans simuler (l'atelier n'est pas affiché) : l'horloge avance
    /// et les messages passagers arrivés à leur terme s'en vont.
    pub fn laisser_passer(&mut self, dt: f32) {
        self.horloge += f64::from(dt);
        let maintenant = self.horloge;
        self.messages
            .retain(|m| m.duree.is_none_or(|d| maintenant - m.ne < f64::from(d)));
    }

    /// Fait passer `dt` secondes : la simulation avance si elle est en lecture.
    pub fn avancer(&mut self, a: &Apparence, dt: f32) {
        self.laisser_passer(dt);
        if !self.lecture || self.vue_t.is_some() || self.session.stable() {
            self.reste = 0.0;
            self.dette = 0.0;
            self.debit = 0.0;
            return;
        }
        if let Some(debit) = self.rapide(a) {
            self.avancer_vite(a, debit, dt);
            return;
        }
        self.reste += dt;
        let duree = self.duree_pas(a);
        let mut faits = 0;
        while self.reste >= duree && faits < 4 && !self.session.stable() {
            self.reste -= duree;
            self.session.pas(&self.doc);
            self.observer();
            faits += 1;
        }
        if faits == 4 {
            // L'image a duré très longtemps : on ne rattrape pas le retard.
            self.reste = 0.0;
        }
    }

    /// Simulation rapide : autant de pas que demandé, dans la limite du temps de calcul
    /// accordé à chaque image pour que l'affichage reste fluide.
    fn avancer_vite(&mut self, a: &Apparence, debit: f32, dt: f32) {
        self.reste = 0.0;
        let voulus = if debit > 0.0 {
            self.dette += f64::from(debit) * f64::from(dt);
            let n = self.dette.floor();
            self.dette -= n;
            n as u64
        } else {
            u64::MAX
        };
        let budget = Duration::from_secs_f32(a.mouvement.budget_rapide / 1000.0);
        let debut = Instant::now();
        let (mut faits, mut lot) = (0u64, 16u64);
        while faits < voulus && !self.session.stable() {
            let n = lot.min(voulus - faits);
            let ecoule = self.session.avancer_vite(n);
            faits += ecoule;
            let passe = debut.elapsed();
            if ecoule < n || passe >= budget {
                break;
            }
            // Le lot suivant est taillé pour tenir dans le temps de calcul qui reste : sur un
            // très gros circuit, on s'arrête à temps au lieu de déborder sur l'image.
            let par_pas = passe.as_secs_f64() / faits.max(1) as f64;
            let tient = ((budget - passe).as_secs_f64() / par_pas.max(1e-9)) as u64;
            if tient == 0 {
                break;
            }
            lot = (lot * 2).min(4096).min(tient);
        }
        if debit > 0.0 && faits < voulus {
            // La machine ne suit pas : on ne laisse pas le retard s'accumuler.
            self.dette = 0.0;
        }
        if dt > 0.0 {
            let mesure = faits as f32 / dt;
            self.debit = if self.debit > 0.0 {
                self.debit * 0.9 + mesure * 0.1
            } else {
                mesure
            };
        }
        self.observer();
    }

    /// Part du pas en cours déjà écoulée, de 0 à 1 (pour les animations).
    pub fn phase(&self, a: &Apparence) -> f32 {
        if self.rapide(a).is_some() {
            return 1.0;
        }
        (self.reste / self.duree_pas(a)).clamp(0.0, 1.0)
    }

    fn observer(&mut self) {
        if !self.chemin.is_empty() {
            return;
        }
        let Some(quete) = &mut self.quete else {
            return;
        };
        if let Some(cas) = quete.observer(&self.doc, &self.session) {
            self.cas_neuf = Some((cas, self.horloge));
            if quete.reussie() {
                self.gagner();
            }
        }
    }

    /// La quête est réussie : on le dit, et sa solution rejoint les briques gagnées.
    fn gagner(&mut self) {
        let Some(quete) = &self.quete else {
            return;
        };
        // Le chapitre de la quête était-il encore à finir ?
        let chapitre = Quete::fiche(quete.cle).map(|f| f.chapitre);
        let fini = |p: &Parcours, nom: &str| {
            savoir::quetes_du_chapitre(nom)
                .iter()
                .all(|&i| p.faite(PARCOURS[i].cle))
        };
        let avant = chapitre.map(|c| fini(&self.parcours, c));
        self.parcours.gagner(quete, &self.projet);
        let texte = quete.felicitations.clone();
        self.fete = Some(self.horloge);
        if let (Some(c), Some(false)) = (chapitre, avant) {
            if fini(&self.parcours, c) {
                let (titre, phrase) = savoir::fin_de_chapitre(c);
                self.grande_fete = Some((self.horloge, titre, phrase));
            }
        }
        // Le panneau de la quête dit déjà ces félicitations : le message ne les répète sur le
        // plan que si le panneau est fermé.
        if !self.volets.quete {
            self.dire(&texte);
        }
    }

    /// Vérifie la quête en cours contre son corrigé. Renvoie `true` si elle est réussie.
    pub fn verifier_la_quete(&mut self) -> bool {
        if !self.chemin.is_empty() {
            return false;
        }
        let Some(quete) = &mut self.quete else {
            return false;
        };
        let reussi = quete.verifier(&self.projet).reussi;
        if reussi {
            self.gagner();
        }
        reussi
    }

    /// Ouvre la quête de cette fiche, dans un projet neuf nommé `nom`, avec les briques
    /// gagnées dans la palette.
    pub fn ouvrir_quete(&mut self, fiche: &'static Fiche, nom: &str, a: &Apparence) -> bool {
        match self.parcours.projet(fiche, nom) {
            Ok(projet) => {
                self.ouvrir(projet, a);
                let briques = self.projet.modeles.len();
                let texte = if fiche.epreuve.ordinateur() {
                    format!("{} : {}", fiche.titre, fiche.consigne)
                } else if briques > 0 && !self.parcours.briques.is_empty() {
                    format!(
                        "{} : {} Tes briques gagnées sont dans la palette. Clique sur Vérifier quand c'est prêt.",
                        fiche.titre, fiche.consigne
                    )
                } else {
                    format!(
                        "{} : {} Clique sur Vérifier quand c'est prêt.",
                        fiche.titre, fiche.consigne
                    )
                };
                self.dire_longtemps(&texte);
                true
            }
            Err(e) => {
                self.dire(&format!("La quête n'a pas pu être ouverte : {e}"));
                false
            }
        }
    }

    /// Avance d'un pas à la main. Revient d'abord au présent.
    pub fn pas(&mut self) {
        self.vue_t = None;
        self.reste = 0.0;
        self.session.pas(&self.doc);
        self.observer();
    }

    /// Lance ou met en pause. Revient d'abord au présent.
    pub fn lecture_ou_pause(&mut self) {
        if self.vue_t.take().is_some() {
            self.lecture = true;
        } else {
            self.lecture = !self.lecture;
        }
        self.reste = 0.0;
        self.dette = 0.0;
    }

    /// Bascule un interrupteur.
    pub fn basculer(&mut self, entree: Id) {
        if self.dans_un_exemplaire(entree) {
            return;
        }
        self.vue_t = None;
        self.invite = false;
        self.reste = 0.0;
        self.session.basculer(&self.doc, entree);
        self.observer();
    }

    /// Bascule un seul bit d'un interrupteur de plusieurs bits.
    pub fn basculer_bit(&mut self, entree: Id, bit: u32) {
        if self.dans_un_exemplaire(entree) {
            return;
        }
        self.vue_t = None;
        self.invite = false;
        self.reste = 0.0;
        self.session.basculer_bit(&self.doc, entree, bit);
        self.observer();
    }

    /// Vrai si l'on regarde un exemplaire de brique : ses interrupteurs ne se touchent pas,
    /// et on le dit.
    fn dans_un_exemplaire(&mut self, entree: Id) -> bool {
        if !self.session.exemplaire() {
            return false;
        }
        if self
            .doc
            .element(entree)
            .is_some_and(|e| e.genre == Genre::Entree)
        {
            self.dire(
                "Cette entrée est pilotée par le plan du dessus. Pour la changer à la main, ouvre la brique seule depuis Mes briques.",
            );
        }
        true
    }

    /// Le clavier qui reçoit les touches : celui qui est sélectionné, sinon le seul du plan.
    /// Dans un exemplaire de brique, c'est le clavier du plan qui tourne.
    pub fn clavier_actif(&self) -> Option<Id> {
        if self.session.exemplaire() {
            return self.session.seul_clavier();
        }
        let claviers = || {
            self.doc
                .elements
                .iter()
                .filter(|e| e.genre == Genre::Clavier)
                .map(|e| e.id)
        };
        match &self.selection {
            Some(Selection::Elements(ids)) => claviers().find(|c| ids.contains(c)),
            _ => None,
        }
        .or_else(|| {
            let mut tous = claviers();
            let premier = tous.next();
            premier.filter(|_| tous.next().is_none())
        })
    }

    /// Tape une touche sur le clavier actif. Renvoie `false` s'il n'y en a pas.
    pub fn taper(&mut self, code: u8) -> bool {
        let Some(clavier) = self.clavier_actif() else {
            return false;
        };
        self.vue_t = None;
        self.session.taper(clavier, code);
        true
    }

    /// Revient à t = 0.
    pub fn zero(&mut self) {
        self.vue_t = None;
        self.reste = 0.0;
        self.dette = 0.0;
        self.session.remettre_a_zero(&self.doc, &self.projet);
    }

    /// Regarde l'instant `t` du passé, ou revient au présent si `t` est l'instant courant.
    pub fn regarder(&mut self, t: u64) {
        self.vue_t = (t < self.session.t && self.session.instant_a(t).is_some()).then_some(t);
    }

    /// L'instant affiché.
    pub fn t_affiche(&self) -> u64 {
        self.vue_t.unwrap_or(self.session.t)
    }

    /// Ce qu'on lit sur la sortie d'un élément (au présent, ou à l'instant regardé).
    pub fn lecture(&self, source: Source) -> Lecture {
        match self.vue_t.and_then(|t| self.session.instant_a(t)) {
            Some(instant) => instant.lecture(source).unwrap_or(Lecture::debranchee(1)),
            None => self.session.lecture(source),
        }
    }

    /// Valeur affichée, résumée, de la sortie d'un élément.
    pub fn valeur(&self, source: Source) -> Valeur {
        self.lecture(source).valeur()
    }

    /// Ce que lit une entrée (en l'air si rien n'y est branché).
    pub fn lecture_lue(&self, fil: Fil) -> Lecture {
        match self.doc.source_de(fil) {
            Some(s) => self.lecture(s),
            None => Lecture::debranchee(self.doc.largeur_entree(fil).unwrap_or(1)),
        }
    }

    /// Valeur affichée lue par une entrée (`Z` si rien n'y est branché).
    pub fn valeur_lue(&self, fil: Fil) -> Valeur {
        self.doc
            .source_de(fil)
            .map_or(Valeur::Z, |s| self.valeur(s))
    }

    /// Résumé de l'état de la simulation.
    pub fn etat(&self) -> Etat {
        if let Some(t) = self.vue_t {
            Etat::Passe(t)
        } else if self.session.stable() {
            Etat::Stable
        } else if self.lecture {
            Etat::EnCours
        } else {
            Etat::EnPause
        }
    }

    /// Affiche un message quelques secondes.
    pub fn dire(&mut self, texte: &str) {
        self.message(texte, Some(4.5 + texte.chars().count() as f32 * 0.035));
    }

    /// Affiche un message jusqu'à ce qu'on le ferme.
    pub fn dire_longtemps(&mut self, texte: &str) {
        self.message(texte, None);
    }

    /// Affiche un message qui propose une action, jusqu'à ce qu'on la choisisse ou qu'on le
    /// ferme. Un seul message par action : le nouveau remplace l'ancien.
    pub fn proposer(&mut self, texte: &str, action: Proposition) {
        self.messages.retain(|m| m.action != Some(action));
        self.message(texte, None);
        if let Some(m) = self.messages.last_mut() {
            m.action = Some(action);
        }
    }

    fn message(&mut self, texte: &str, duree: Option<f32>) {
        self.messages.retain(|m| m.texte != texte);
        self.prochain_message += 1;
        self.messages.push(Message {
            id: self.prochain_message,
            texte: texte.to_owned(),
            ne: self.horloge,
            duree,
            action: None,
        });
        if self.messages.len() > 3 {
            // Les messages qui proposent une action (mettre à jour, redémarrer) restent : on
            // retire d'abord le plus ancien des autres.
            let i = self
                .messages
                .iter()
                .position(|m| m.action.is_none())
                .unwrap_or(0);
            self.messages.remove(i);
        }
    }

    /// Ferme un message.
    pub fn fermer_message(&mut self, id: u64) {
        self.messages.retain(|m| m.id != id);
    }

    /// À appeler à la fin de chaque geste qui a pu modifier le circuit : le projet est mis à
    /// jour, la simulation suit, la quête repart si le câblage a changé, et l'action devient
    /// annulable. Renvoie `true` si quelque chose a changé.
    pub fn apres_modification(&mut self, a: &Apparence) -> bool {
        let change = self
            .historique
            .noter(&self.doc, a.manipulation.profondeur_annuler);
        if change {
            self.deposer();
        }
        self.suivre();
        change
    }

    /// Range le plan affiché dans le projet.
    fn deposer(&mut self) {
        let lieu = self.chemin.last().cloned();
        self.projet.deposer(lieu.as_deref(), self.doc.clone());
        self.version += 1;
    }

    fn suivre(&mut self) {
        if self.session.suivre(&self.doc, &self.projet) {
            if let Some(q) = &mut self.quete {
                if self.chemin.is_empty() {
                    q.recommencer();
                }
            }
            self.cas_neuf = None;
            self.vue_t = None;
            self.reste = 0.0;
            self.observer();
            if let Some(e) = self.session.erreur().map(str::to_owned) {
                self.dire(&format!("Ce circuit ne peut pas être simulé : {e}."));
            }
        }
        let doc = &self.doc;
        self.selection = match self.selection.take() {
            Some(Selection::Elements(mut ids)) => {
                ids.retain(|id| doc.element(*id).is_some());
                (!ids.is_empty()).then_some(Selection::Elements(ids))
            }
            Some(Selection::Fil(f)) => doc.source_de(f).map(|_| Selection::Fil(f)),
            None => None,
        };
    }

    /// Annule la dernière action. Renvoie `true` s'il y avait quelque chose à annuler.
    pub fn annuler(&mut self) -> bool {
        match self.historique.annuler() {
            Some(doc) => {
                self.doc = doc;
                self.deposer();
                self.suivre();
                true
            }
            None => false,
        }
    }

    /// Rétablit l'action annulée. Renvoie `true` s'il y avait quelque chose à rétablir.
    pub fn retablir(&mut self) -> bool {
        match self.historique.retablir() {
            Some(doc) => {
                self.doc = doc;
                self.deposer();
                self.suivre();
                true
            }
            None => false,
        }
    }

    /// Les éléments sélectionnés.
    pub fn elements_selectionnes(&self) -> &[Id] {
        match &self.selection {
            Some(Selection::Elements(ids)) => ids,
            _ => &[],
        }
    }

    /// L'élément sélectionné, s'il n'y en a qu'un.
    pub fn element_selectionne(&self) -> Option<Id> {
        match self.elements_selectionnes() {
            [id] => Some(*id),
            _ => None,
        }
    }

    /// Le fil sélectionné.
    pub fn fil_selectionne(&self) -> Option<Fil> {
        match self.selection {
            Some(Selection::Fil(f)) => Some(f),
            _ => None,
        }
    }

    /// Vrai si l'élément fait partie de la sélection.
    pub fn est_selectionne(&self, id: Id) -> bool {
        self.elements_selectionnes().contains(&id)
    }

    /// Sélectionne un seul élément.
    pub fn selectionner(&mut self, id: Id) {
        self.selection = Some(Selection::Elements(vec![id]));
    }

    /// Sélectionne ces éléments (rien si la liste est vide).
    pub fn selectionner_tous(&mut self, mut ids: Vec<Id>) {
        ids.sort_unstable();
        ids.dedup();
        self.selection = (!ids.is_empty()).then_some(Selection::Elements(ids));
    }

    /// Ajoute l'élément à la sélection, ou l'en retire s'il y était.
    pub fn basculer_selection(&mut self, id: Id) {
        let mut ids = self.elements_selectionnes().to_vec();
        match ids.iter().position(|x| *x == id) {
            Some(i) => {
                ids.remove(i);
            }
            None => ids.push(id),
        }
        self.selectionner_tous(ids);
    }

    /// Pose un élément tout fait. L'action devient annulable au prochain
    /// [`Self::apres_modification`].
    pub fn poser(&mut self, element: Element, pos: Point) -> Id {
        let id = self.doc.poser(element, pos);
        self.selectionner(id);
        id
    }

    /// Pose une porte NAND.
    pub fn poser_porte(&mut self, pos: Point) -> Id {
        self.poser(Element::nand(""), pos)
    }

    /// Vrai si la brique `modele` peut être posée sur le plan affiché : elle existe et ne se
    /// contiendrait pas elle-même.
    pub fn peut_poser(&self, modele: &str) -> bool {
        self.projet.peut_poser(self.lieu(), modele)
    }

    /// Pose une brique du modèle `modele`, si c'est possible.
    pub fn poser_brique(&mut self, modele: &str, pos: Point) -> Option<Id> {
        if !self.peut_poser(modele) {
            self.dire("Une brique ne peut pas se contenir elle-même.");
            return None;
        }
        let element = self.projet.modele(modele)?.instance();
        Some(self.poser(element, pos))
    }

    /// Regroupe les éléments sélectionnés en une nouvelle brique. Renvoie la brique posée.
    pub fn regrouper_selection(&mut self, a: &Apparence) -> Option<Id> {
        let ids = self.elements_selectionnes().to_vec();
        let nom = self.projet.nom_libre("Brique");
        let Some((modele, id)) = brique::regrouper(&mut self.doc, &ids, &nom) else {
            self.dire("Sélectionne d'abord des portes ou des briques à regrouper.");
            return None;
        };
        let portes = self.projet.nb_portes(&modele.plan);
        let nom = self.projet.ajouter(modele);
        self.selectionner(id);
        self.apres_modification(a);
        self.dire(&format!(
            "« {nom} » est une nouvelle brique de {portes} porte{}. Elle est dans la palette, prête à resservir ; double-clique dessus pour l'ouvrir.",
            if portes > 1 { "s" } else { "" }
        ));
        Some(id)
    }

    /// Enregistre tout le plan affiché comme une brique : ses interrupteurs et ses lampes
    /// deviennent ses broches. Renvoie le nom de la brique.
    pub fn plan_en_brique(&mut self) -> Option<String> {
        if !self.chemin.is_empty() {
            self.dire("Ce plan est déjà celui d'une brique.");
            return None;
        }
        if self.doc.portes().count() == 0
            && !self.doc.elements.iter().any(|e| e.genre == Genre::Brique)
        {
            self.dire("Il n'y a encore rien à enregistrer sur ce plan.");
            return None;
        }
        let base = self.quete.as_ref().map_or("Brique", |q| q.brique);
        let nom = self.projet.nom_libre(base);
        let nom = self.projet.ajouter(brique::modele_du_plan(&self.doc, &nom));
        self.version += 1;
        self.dire(&format!(
            "Le plan est enregistré comme la brique « {nom} ». Elle est dans la palette."
        ));
        Some(nom)
    }

    /// Renomme un élément du plan. Refuse un nom vide ou déjà porté.
    pub fn renommer_element(&mut self, id: Id, nom: &str, a: &Apparence) -> bool {
        let nom = nom.trim();
        if nom.is_empty() {
            return false;
        }
        if self.doc.elements.iter().any(|e| e.id != id && e.nom == nom) {
            self.dire(&format!("Un autre élément s'appelle déjà « {nom} »."));
            return false;
        }
        if self
            .quete
            .as_ref()
            .is_some_and(|q| self.chemin.is_empty() && q.tient_a(&self.doc, id))
        {
            self.dire("La quête a besoin de ce nom.");
            return false;
        }
        match self.doc.element_mut(id) {
            Some(e) => e.nom = nom.to_owned(),
            None => return false,
        }
        self.apres_modification(a);
        true
    }

    /// Change le nombre de bits d'un interrupteur ou d'une lampe.
    pub fn changer_largeur(&mut self, id: Id, largeur: u32, a: &Apparence) -> bool {
        if !(1..=64).contains(&largeur) {
            self.dire("Un fil porte de 1 à 64 bits.");
            return false;
        }
        if self
            .quete
            .as_ref()
            .is_some_and(|q| self.chemin.is_empty() && q.tient_a(&self.doc, id))
        {
            self.dire("La quête a besoin de cet élément tel qu'il est.");
            return false;
        }
        self.doc.changer_largeur(id, largeur) && self.apres_modification(a)
    }

    /// Règle un découpeur d'après un texte comme `0:3 4:7` (ou `0 1 2 3`) : une sortie par
    /// morceau. La largeur du bus d'entrée est gardée si elle suffit.
    pub fn regler_decoupe(&mut self, id: Id, texte: &str, a: &Apparence) -> bool {
        let mut plages = Vec::new();
        for mot in texte.split([' ', ',', ';']).filter(|m| !m.is_empty()) {
            let lu = match mot.split_once(':') {
                Some((x, y)) => x.trim().parse().ok().zip(y.trim().parse().ok()),
                None => mot.trim().parse().ok().map(|x: u32| (x, x)),
            };
            match lu {
                Some(p) => plages.push(p),
                None => {
                    self.dire("Écris les morceaux comme 0:3 4:7, ou 0 1 2 3 pour des bits seuls.");
                    return false;
                }
            }
        }
        let besoin = plages.iter().map(|&(x, y)| x.max(y) + 1).max().unwrap_or(0);
        let actuelle = self
            .doc
            .largeur_entree(Fil {
                puits: id,
                entree: 0,
            })
            .unwrap_or(1);
        if !self
            .doc
            .redefinir_decoupe(id, actuelle.max(besoin), &plages)
        {
            self.dire("Ces morceaux ne tiennent pas dans un bus de 64 bits.");
            return false;
        }
        self.apres_modification(a)
    }

    /// Règle une jonction d'après un texte comme `4 4` : une entrée par largeur.
    pub fn regler_jonction(&mut self, id: Id, texte: &str, a: &Apparence) -> bool {
        let largeurs: Option<Vec<u32>> = texte
            .split([' ', ',', ';', '+'])
            .filter(|m| !m.is_empty())
            .map(|m| m.trim().parse().ok())
            .collect();
        match largeurs {
            Some(l) if self.doc.redefinir_jonction(id, &l) => self.apres_modification(a),
            _ => {
                self.dire(
                    "Écris la largeur de chaque entrée, comme 4 4 ou 1 1 1 1 (64 bits au plus).",
                );
                false
            }
        }
    }

    /// Change la demi-période d'une horloge, la durée d'une touche de clavier ou celle d'une
    /// impulsion de démarrage (0 : un interrupteur ordinaire).
    pub fn changer_duree(&mut self, id: Id, duree: u64, a: &Apparence) -> bool {
        let Some(e) = self.doc.element_mut(id) else {
            return false;
        };
        e.param = match e.genre {
            Genre::Horloge | Genre::Clavier => duree.clamp(1, 1_000_000),
            Genre::Entree => duree.min(1_000_000),
            _ => return false,
        };
        self.apres_modification(a)
    }

    /// Change le nom du projet.
    pub fn renommer_projet(&mut self, nom: &str) -> bool {
        let nom = nom.trim();
        if nom.is_empty() {
            return false;
        }
        self.projet.nom = nom.to_owned();
        self.version += 1;
        true
    }

    /// Le clavier sélectionné, s'il y en a un : c'est lui qui reçoit ce qu'on tape.
    pub fn clavier_selectionne(&self) -> Option<Id> {
        self.element_selectionne().filter(|id| {
            self.doc
                .element(*id)
                .is_some_and(|e| e.genre == Genre::Clavier)
        })
    }

    /// Renomme une brique, partout.
    pub fn renommer_modele(&mut self, ancien: &str, nouveau: &str) -> bool {
        let nouveau = nouveau.trim().to_owned();
        match self.projet.renommer(ancien, &nouveau) {
            Ok(()) => {
                for n in &mut self.chemin {
                    if n == ancien {
                        n.clone_from(&nouveau);
                    }
                }
                for e in &mut self.doc.elements {
                    if e.genre == Genre::Brique && e.modele == ancien {
                        e.modele.clone_from(&nouveau);
                    }
                }
                self.historique = Historique::nouveau(&self.doc);
                self.session.suivre(&self.doc, &self.projet);
                self.version += 1;
                true
            }
            Err(e) => {
                self.dire(&e);
                false
            }
        }
    }

    /// Remplace le programme de la mémoire morte `modele` par `texte` ([`crate::programme`]),
    /// puis redémarre la simulation : l'ordinateur repart de zéro sur son nouveau programme.
    /// En cas d'erreur (assemblage), rien ne change.
    pub fn programmer(
        &mut self,
        modele: &str,
        texte: &str,
    ) -> Result<crate::programme::Charge, String> {
        let charge = crate::programme::charger(&mut self.projet, modele, texte)?;
        if let Some(lieu) = self.chemin.last() {
            if let Some(plan) = self.projet.plan(Some(lieu)) {
                self.doc = plan.clone();
            }
        }
        self.historique = Historique::nouveau(&self.doc);
        self.version += 1;
        self.suivre();
        self.zero();
        Ok(charge)
    }

    /// La mémoire morte programmable à proposer : celle de la brique sélectionnée si elle en
    /// est une, sinon celle du plan affiché, sinon la première du projet.
    pub fn memoire_programmable(&self) -> Option<String> {
        let programmables = crate::programme::programmables(&self.projet);
        let selection = self
            .element_selectionne()
            .and_then(|id| self.doc.element(id))
            .filter(|e| e.genre == Genre::Brique)
            .map(|e| e.modele.as_str());
        selection
            .filter(|m| programmables.contains(m))
            .or_else(|| {
                self.chemin
                    .last()
                    .map(String::as_str)
                    .filter(|m| programmables.contains(m))
            })
            .or_else(|| programmables.first().copied())
            .map(str::to_owned)
    }

    /// Retire de la palette une brique qui n'est posée nulle part.
    pub fn retirer_modele(&mut self, nom: &str) -> bool {
        match self.projet.retirer(nom) {
            Ok(()) => {
                self.version += 1;
                true
            }
            Err(e) => {
                self.dire(&e);
                false
            }
        }
    }

    /// Vrai si l'élément peut être supprimé : les interrupteurs et les lampes dont la quête
    /// a besoin restent.
    pub fn supprimable(&self, id: Id) -> bool {
        self.doc.element(id).is_some()
            && !self
                .quete
                .as_ref()
                .is_some_and(|q| self.chemin.is_empty() && q.tient_a(&self.doc, id))
    }

    /// Supprime ce qui est sélectionné. Renvoie `true` si quelque chose a été supprimé.
    pub fn supprimer_selection(&mut self, a: &Apparence) -> bool {
        match self.selection.clone() {
            Some(Selection::Elements(ids)) => {
                let (oui, non): (Vec<Id>, Vec<Id>) =
                    ids.iter().partition(|id| self.supprimable(**id));
                if !non.is_empty() {
                    self.dire("Les interrupteurs et les lampes de la quête restent en place.");
                }
                if oui.is_empty() {
                    return false;
                }
                for id in oui {
                    self.doc.retirer(id);
                }
            }
            Some(Selection::Fil(fil)) => {
                self.doc.debrancher(fil);
            }
            None => return false,
        }
        self.selection = None;
        self.apres_modification(a)
    }

    /// Copie les éléments sélectionnés, un peu décalés, et sélectionne les copies.
    pub fn dupliquer_selection(&mut self, a: &Apparence) -> bool {
        let ids = self.elements_selectionnes().to_vec();
        if ids.is_empty() {
            return false;
        }
        let pas = (40.0 / a.manipulation.grille).round().max(1.0) * a.manipulation.grille;
        let copies = self.doc.dupliquer(&ids, pas, pas);
        self.selectionner_tous(copies);
        self.apres_modification(a)
    }

    /// Nom d'un genre d'élément, pour l'affichage.
    pub fn nom_genre(e: &Element) -> &str {
        match e.genre {
            Genre::Nand => "NAND",
            Genre::Entree if e.param > 0 => "démarrage",
            Genre::Entree => "interrupteur",
            Genre::Sortie => "lampe",
            Genre::Brique => &e.modele,
            Genre::Horloge => "horloge",
            Genre::Ecran => "écran",
            Genre::Clavier => "clavier",
            Genre::Decoupe => "découpeur",
            Genre::Jonction => "jonction",
        }
    }

    /// Nom affiché de la sélection (« g4 · NAND », « fil n1 », « 3 éléments »).
    pub fn nom_selection(&self) -> Option<String> {
        match self.selection.as_ref()? {
            Selection::Elements(ids) => match ids.as_slice() {
                [id] => {
                    let e = self.doc.element(*id)?;
                    Some(format!("{} · {}", e.nom, Self::nom_genre(e)))
                }
                plusieurs => Some(format!("{} éléments", plusieurs.len())),
            },
            Selection::Fil(f) => Some(format!(
                "fil {}",
                self.doc.nom_signal(self.doc.source_de(*f)?)
            )),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn id(at: &Atelier, nom: &str) -> Id {
        at.doc.elements.iter().find(|e| e.nom == nom).unwrap().id
    }

    #[test]
    fn les_messages_qui_proposent_une_action_ne_sont_pas_chasses_par_les_autres() {
        let a = Apparence::default();
        let mut at = Atelier::nouveau(&a);
        at.messages.clear();
        at.proposer("Nouvelle version", Proposition::MettreAJour);
        for i in 0..6 {
            at.dire(&format!("message {i}"));
        }
        assert_eq!(at.messages.len(), 3);
        assert!(at
            .messages
            .iter()
            .any(|m| m.action == Some(Proposition::MettreAJour)));
        at.proposer("Redémarrer ?", Proposition::Redemarrer);
        at.dire("encore");
        let actions: Vec<_> = at.messages.iter().filter_map(|m| m.action).collect();
        assert_eq!(
            actions,
            vec![Proposition::MettreAJour, Proposition::Redemarrer]
        );
        assert_eq!(at.messages.last().unwrap().texte, "encore");
    }

    #[test]
    fn la_lecture_avance_au_rythme_choisi_puis_s_arrete_au_repos() {
        let a = Apparence::default();
        let mut at = Atelier::nouveau(&a);
        assert_eq!(at.etat(), Etat::EnCours);
        assert_eq!(at.libelle_vitesse(&a), "1,7 pas/s");
        // 0,6 s par pas : après 0,5 s rien, après 0,7 s un pas.
        at.avancer(&a, 0.5);
        assert_eq!(at.session.t, 0);
        assert!((at.phase(&a) - 0.5 / 0.6).abs() < 1e-3);
        at.avancer(&a, 0.2);
        assert_eq!(at.session.t, 1);
        for _ in 0..100 {
            at.avancer(&a, 0.1);
        }
        assert_eq!(
            at.session.t, 3,
            "le demi-additionneur est au repos après trois pas, le temps s'arrête"
        );
        assert_eq!(at.etat(), Etat::Stable);
        assert_eq!(
            at.quete.as_ref().unwrap().vus.len(),
            1,
            "le cas a = 1, b = 0 est validé"
        );
        at.changer_vitesse(&a);
        assert_eq!(at.libelle_vitesse(&a), "4 pas/s");
        at.changer_vitesse(&a);
        assert_eq!(at.libelle_vitesse(&a), "128 pas/s");
        for attendu in ["2048 pas/s", "33 k pas/s", "max", "1 pas/s"] {
            at.changer_vitesse(&a);
            assert_eq!(at.libelle_vitesse(&a), attendu);
        }
    }

    #[test]
    fn une_image_tres_longue_ne_fait_pas_une_rafale_de_pas() {
        let a = Apparence::default();
        let mut at = Atelier::nouveau(&a);
        at.avancer(&a, 60.0);
        assert!(at.session.t <= 4);
    }

    #[test]
    fn on_remonte_le_temps_sans_toucher_au_present() {
        let a = Apparence::default();
        let mut at = Atelier::nouveau(&a);
        for _ in 0..3 {
            at.pas();
        }
        let s = Source::de(id(&at, "g4"));
        assert_eq!(at.valeur(s), Valeur::Un);
        at.regarder(1);
        assert_eq!(at.etat(), Etat::Passe(1));
        assert_eq!(at.t_affiche(), 1);
        assert_eq!(
            at.valeur(s),
            Valeur::X,
            "à t = 1 la somme n'était pas encore connue"
        );
        at.avancer(&a, 5.0);
        assert_eq!(
            at.session.t, 3,
            "le temps ne s'écoule pas pendant qu'on regarde le passé"
        );
        at.regarder(3);
        assert_eq!(at.etat(), Etat::Stable);
        at.regarder(1);
        at.lecture_ou_pause();
        assert_eq!(at.vue_t, None, "lecture ramène au présent");
    }

    #[test]
    fn recabler_remet_la_quete_a_zero_et_s_annule() {
        let a = Apparence::default();
        let mut at = Atelier::nouveau(&a);
        for _ in 0..3 {
            at.pas();
        }
        assert_eq!(at.quete.as_ref().unwrap().vus.len(), 1);
        let (g2, g4) = (id(&at, "g2"), id(&at, "g4"));
        at.selection = Some(Selection::Fil(Fil {
            puits: g4,
            entree: 0,
        }));
        assert_eq!(at.nom_selection().as_deref(), Some("fil n2"));
        assert!(at.supprimer_selection(&a));
        assert_eq!(at.selection, None);
        assert!(
            at.quete.as_ref().unwrap().vus.is_empty(),
            "le câblage a changé : les cas validés ne valent plus"
        );
        assert_eq!(
            at.valeur_lue(Fil {
                puits: g4,
                entree: 0
            }),
            Valeur::Z
        );
        assert!(at.annuler());
        assert_eq!(
            at.doc.source_de(Fil {
                puits: g4,
                entree: 0
            }),
            Some(Source::de(g2))
        );
        assert!(at.retablir());
        assert_eq!(
            at.doc.source_de(Fil {
                puits: g4,
                entree: 0
            }),
            None
        );
        assert!(!at.retablir());
    }

    #[test]
    fn les_interrupteurs_et_les_lampes_ne_se_suppriment_pas() {
        let a = Apparence::default();
        let mut at = Atelier::nouveau(&a);
        at.selectionner(id(&at, "a"));
        assert!(!at.supprimer_selection(&a));
        assert!(at.doc.elements.iter().any(|e| e.nom == "a"));
        assert!(at
            .messages
            .iter()
            .any(|m| m.texte.contains("restent en place")));
        let porte = at.poser_porte(Point::nouveau(300.0, 400.0));
        assert_eq!(at.nom_selection().as_deref(), Some("g6 · NAND"));
        assert!(at.apres_modification(&a));
        assert!(at.supprimer_selection(&a));
        assert!(at.doc.element(porte).is_none());
    }

    #[test]
    fn les_messages_s_effacent_tout_seuls_sauf_ceux_qui_attendent() {
        let a = Apparence::default();
        let mut at = Atelier::nouveau(&a);
        at.lecture = false;
        at.dire("Bonjour");
        at.dire("Bonjour");
        assert_eq!(
            at.messages.len(),
            2,
            "le même message n'est pas empilé deux fois"
        );
        at.avancer(&a, 30.0);
        assert_eq!(
            at.messages.len(),
            1,
            "le message d'accueil reste jusqu'à ce qu'on le ferme"
        );
        let accueil = at.messages[0].id;
        at.fermer_message(accueil);
        assert!(at.messages.is_empty());
    }

    fn jusqu_au_repos(at: &mut Atelier) {
        for _ in 0..200 {
            if at.session.stable() {
                return;
            }
            at.pas();
        }
        panic!("le circuit ne se stabilise pas");
    }

    #[test]
    fn regrouper_ne_change_pas_ce_que_calcule_le_circuit_et_s_annule() {
        let a = Apparence::default();
        let mut at = Atelier::nouveau(&a);
        let (ea, eb) = (id(&at, "a"), id(&at, "b"));
        let lampe = |at: &Atelier, nom: &str| {
            at.valeur_lue(Fil {
                puits: id(at, nom),
                entree: 0,
            })
        };
        let portes: Vec<Id> = at.doc.portes().map(|e| e.id).collect();
        at.selectionner_tous(portes);
        assert_eq!(at.nom_selection().as_deref(), Some("5 éléments"));
        let brique = at.regrouper_selection(&a).unwrap();
        assert_eq!(at.doc.elements.len(), 5);
        assert_eq!(at.projet.modeles.len(), 1);
        assert_eq!(
            at.projet.principal, at.doc,
            "le projet suit le plan affiché"
        );
        assert_eq!(at.session.nb_portes(), 5);
        assert_eq!(at.nom_selection().as_deref(), Some("bri1 · Brique"));
        // Les quatre cas : a = 1 au départ.
        for (bascule, s, c) in [
            (None, 1, 0),
            (Some(eb), 0, 1),
            (Some(ea), 1, 0),
            (Some(eb), 0, 0),
        ] {
            if let Some(e) = bascule {
                at.basculer(e);
            }
            jusqu_au_repos(&mut at);
            assert_eq!(lampe(&at, "s"), Valeur::de_bool(s == 1));
            assert_eq!(lampe(&at, "c"), Valeur::de_bool(c == 1));
        }
        assert!(
            at.quete.as_ref().unwrap().reussie(),
            "la quête se valide avec la brique"
        );
        // On entre dans la brique posée : son plan a cinq portes, et on le voit tourner dans le
        // plan du dessus.
        assert!(at.entrer(brique));
        assert!(at.session.exemplaire());
        assert_eq!(at.lieu(), Some("Brique"));
        assert_eq!(at.doc.portes().count(), 5);
        assert!(
            !at.peut_poser("Brique"),
            "une brique ne se pose pas dans elle-même"
        );
        // On y coupe la retenue : dehors, la lampe c ne s'allume plus.
        let g5 = id(&at, "g5");
        at.selectionner(g5);
        assert!(at.supprimer_selection(&a));
        assert!(at.sortir());
        assert_eq!(at.lieu(), None);
        assert_eq!(at.session.nb_portes(), 4);
        at.basculer(ea);
        at.basculer(eb);
        jusqu_au_repos(&mut at);
        assert_eq!(lampe(&at, "s"), Valeur::Zero);
        assert_ne!(lampe(&at, "c"), Valeur::Un);
        // Renommer la brique la renomme partout.
        assert!(at.renommer_modele("Brique", "Demi-additionneur"));
        assert_eq!(at.doc.element(brique).unwrap().modele, "Demi-additionneur");
        assert!(
            !at.retirer_modele("Demi-additionneur"),
            "elle est encore posée"
        );
    }

    #[test]
    fn annuler_un_regroupement_remet_les_portes() {
        let a = Apparence::default();
        let mut at = Atelier::nouveau(&a);
        let avant = at.doc.clone();
        let portes: Vec<Id> = at.doc.portes().map(|e| e.id).collect();
        at.selectionner_tous(portes);
        at.regrouper_selection(&a).unwrap();
        assert!(at.annuler());
        assert_eq!(at.doc, avant);
        assert_eq!(at.projet.principal, avant);
        assert_eq!(at.session.nb_portes(), 5);
    }

    #[test]
    fn l_ordinateur_tourne_dans_l_atelier_en_vitesse_rapide() {
        let mut a = Apparence::default();
        // Pas de limite de temps de calcul : le test ne doit pas dépendre de la machine.
        a.mouvement.budget_rapide = 100.0;
        let mut at = Atelier::nouveau(&a);
        assert!(at.ouvrir_ordinateur(
            "Ordinateur miniature",
            ce_circuit::ordinateur::programmes::CALCULATRICE,
            &a
        ));
        assert!(at.quete.is_none());
        assert!(
            at.rapide(&a).is_some(),
            "un circuit à horloge s'ouvre en vitesse rapide"
        );
        assert_eq!(at.etat(), Etat::EnCours);
        let ecran = at
            .doc
            .elements
            .iter()
            .find(|e| e.genre == Genre::Ecran)
            .unwrap()
            .id;
        assert!(at.taper(b'6'));
        assert!(at.taper(b'7'));
        let mut images = 0;
        while at.session.ecran(ecran) != "6*7=42" && images < 600 {
            at.avancer(&a, 1.0 / 60.0);
            images += 1;
        }
        assert_eq!(at.session.ecran(ecran), "6*7=42", "après {images} images");
        assert!(at.debit > 1000.0);
        // On descend jusqu'aux portes : ORDINATEUR, puis le processeur, puis l'unité de calcul.
        let pc = at.doc.nomme("pc").unwrap().id;
        assert!(at.entrer(pc));
        let cpu = at.doc.nomme("cpu").unwrap().id;
        assert!(at.entrer(cpu));
        let ual = at.doc.nomme("ual").unwrap().id;
        assert!(at.entrer(ual));
        assert_eq!(at.chemin.len(), 3);
        assert!(at.session.erreur().is_none());
        assert!(at.remonter(0));
        assert_eq!(
            at.session.ecran(ecran),
            "6*7=42",
            "en revenant, la simulation du plan principal a continué là où elle était"
        );
    }

    /// Ce que le plan du dessus voit aux broches de la brique posée `id` : la lecture de chaque
    /// entrée, puis de chaque sortie.
    fn broches_vues_de_dehors(at: &Atelier, id: Id) -> (Vec<Lecture>, Vec<Lecture>) {
        let e = at.doc.element(id).unwrap();
        let entrees = (0..e.broches_e.len())
            .map(|entree| at.lecture_lue(Fil { puits: id, entree }))
            .collect();
        let sorties = (0..e.broches_s.len())
            .map(|j| at.lecture(Source::nouvelle(id, j)))
            .collect();
        (entrees, sorties)
    }

    /// Les mêmes broches vues de l'intérieur de l'exemplaire ouvert : les interrupteurs et les
    /// lampes de son plan.
    fn broches_vues_de_dedans(at: &Atelier, brique: &Element) -> (Vec<Lecture>, Vec<Lecture>) {
        let entrees = brique
            .broches_e
            .iter()
            .map(|b| at.lecture(Source::de(b.origine)))
            .collect();
        let sorties = brique
            .broches_s
            .iter()
            .map(|b| {
                at.lecture_lue(Fil {
                    puits: b.origine,
                    entree: 0,
                })
            })
            .collect();
        (entrees, sorties)
    }

    #[test]
    fn ouvrir_une_brique_posee_montre_cet_exemplaire_en_marche() {
        let a = Apparence::default();
        let mut at = Atelier::nouveau(&a);
        let (ea, eb) = (id(&at, "a"), id(&at, "b"));
        let portes: Vec<Id> = at.doc.portes().map(|e| e.id).collect();
        at.selectionner_tous(portes);
        let brique = at.regrouper_selection(&a).unwrap();
        // a = 1, b = 1 : la somme vaut 0 et la retenue 1.
        at.basculer(eb);
        jusqu_au_repos(&mut at);
        let dehors = broches_vues_de_dehors(&at, brique);
        let element = at.doc.element(brique).unwrap().clone();
        assert!(at.entrer(brique));
        assert!(at.session.exemplaire());
        assert_eq!(broches_vues_de_dedans(&at, &element), dehors);
        let valeurs: Vec<Valeur> = at
            .doc
            .elements
            .iter()
            .filter(|e| e.genre == Genre::Entree)
            .map(|e| at.valeur(Source::de(e.id)))
            .collect();
        assert_eq!(
            valeurs,
            [Valeur::Un, Valeur::Un],
            "les entrées viennent de dehors"
        );
        for g in at.doc.portes() {
            assert!(
                at.valeur(Source::de(g.id)).est_binaire(),
                "{} n'est pas indéterminée",
                g.nom
            );
            assert!(at.session.en_attente(&at.doc).is_empty());
        }
        // Les interrupteurs d'un exemplaire ne se touchent pas : ils sont pilotés d'en haut.
        let dedans_a = at
            .doc
            .elements
            .iter()
            .find(|e| e.genre == Genre::Entree)
            .unwrap()
            .id;
        at.basculer(dedans_a);
        assert_eq!(at.valeur(Source::de(dedans_a)), Valeur::Un);
        assert!(at
            .messages
            .iter()
            .any(|m| m.texte.contains("plan du dessus")));
        // En remontant, la même simulation continue : b repasse à 0 et l'exemplaire suit.
        assert!(at.sortir());
        assert!(!at.session.exemplaire());
        at.basculer(eb);
        let dehors = {
            jusqu_au_repos(&mut at);
            broches_vues_de_dehors(&at, brique)
        };
        assert!(at.entrer(brique));
        assert_eq!(broches_vues_de_dedans(&at, &element), dehors);
        // Ouverte depuis la liste des briques, elle est simulée seule.
        assert!(at.remonter(0));
        assert!(at.entrer_dans("Brique"));
        assert!(!at.session.exemplaire());
        at.basculer(dedans_a);
        assert_eq!(at.valeur(Source::de(dedans_a)), Valeur::Un);
        let _ = ea;
    }

    #[test]
    fn dans_l_ordinateur_ouvert_on_voit_le_processeur_calculer() {
        let mut a = Apparence::default();
        a.mouvement.budget_rapide = 100.0;
        let mut at = Atelier::nouveau(&a);
        assert!(at.ouvrir_ordinateur(
            "Ordinateur miniature",
            ce_circuit::ordinateur::programmes::CALCULATRICE,
            &a
        ));
        let ecran = at.doc.nomme("ecran").unwrap().id;
        for _ in 0..30 {
            at.avancer(&a, 1.0 / 60.0);
        }
        // Le temps est arrêté le temps de comparer dehors et dedans.
        at.lecture = false;
        let pc = at.doc.nomme("pc").unwrap().clone();
        let dehors = broches_vues_de_dehors(&at, pc.id);
        assert!(at.entrer(pc.id));
        assert!(at.session.exemplaire());
        assert_eq!(broches_vues_de_dedans(&at, &pc), dehors);
        let cpu = at.doc.nomme("cpu").unwrap().clone();
        let dehors = broches_vues_de_dehors(&at, cpu.id);
        assert!(at.entrer(cpu.id));
        assert_eq!(broches_vues_de_dedans(&at, &cpu), dehors);
        assert_eq!(at.session.regard().unwrap().chemin, [pc.id, cpu.id]);
        let determinees =
            |at: &Atelier| -> Vec<Lecture> { at.doc.prises().map(|s| at.lecture(s)).collect() };
        let avant = determinees(&at);
        // Le processeur tourne : ses signaux ont des valeurs. (Le bus lu en mémoire peut
        // encore être indéterminé entre deux lectures, comme dans le plan du dessus.)
        let nettes = avant.iter().filter(|l| l.flous == 0).count();
        assert!(
            nettes * 10 >= avant.len() * 9,
            "{nettes} signaux déterminés sur {}",
            avant.len()
        );
        // On tape et on laisse tourner sans sortir du processeur : il calcule, et l'écran du
        // plan principal se remplit.
        at.lecture = true;
        assert!(at.taper(b'6'));
        assert!(at.taper(b'7'));
        let mut images = 0;
        while at.session.ecran(ecran) != "6*7=42" && images < 600 {
            at.avancer(&a, 1.0 / 60.0);
            images += 1;
        }
        assert_eq!(at.session.ecran(ecran), "6*7=42", "après {images} images");
        assert_ne!(
            determinees(&at),
            avant,
            "les signaux du processeur ont bougé"
        );
        assert!(at.remonter(0));
        assert!(!at.session.exemplaire());
        assert_eq!(at.session.ecran(ecran), "6*7=42");
    }
}
