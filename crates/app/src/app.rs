//! L'application : elle relie l'apparence, l'atelier, le plan et les panneaux, image après image.
//!
//! Deux écrans : l'accueil (menu, projets, parcours, paramètres) et l'atelier (le plan et ses
//! panneaux). Au lancement, l'accueil s'ouvre avec le dernier projet prêt à reprendre ; un
//! réglage permet de reprendre directement.

use std::path::Path;
use std::time::SystemTime;

use egui::Key;

use crate::accueil::{Accueil, Contexte, Demande, Rubrique};
use crate::apparence::{Apparence, Courbe, Mode, Surveillance};
use crate::brique::Projet;
use crate::calques::{self, Fantome};
use crate::dessin::{installer_polices, Animations};
use crate::disposition::{disposer, Disposition, Ouvertures};
use crate::document::{Document, Point};
use crate::etat::{Atelier, Proposition, ASTUCE_DU_PLAN};
use crate::exemples::EXEMPLES;
use crate::fenetres::Fenetres;
use crate::incident;
use crate::mise_a_jour::{self, Etat, MiseAJour, Nouvelle, Source};
use crate::panneaux::{Ouverture, Panneaux};
use crate::photo::{Photo, Suite};
use crate::plan::Plan;
use crate::quete::Quete;
use crate::raccourci::{Lieu, Raccourcis};
use crate::reglages::{Lancement, Reglages};
use crate::sauvegarde::{nom_de_fichier, Dossier, FicheProjet};

/// Secondes sans modification avant que le projet soit enregistré tout seul.
const DELAI_ENREGISTREMENT: f64 = 1.0;

/// L'écran affiché.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Ecran {
    /// L'accueil : menu, projets, parcours, paramètres.
    Accueil,
    /// Le plan et ses panneaux.
    Atelier,
}

/// Le dernier projet mis à la corbeille, de quoi le ramener.
struct Jete {
    /// Son nom dans la corbeille.
    corbeille: String,
    /// Son nom de fichier d'avant.
    fichier: String,
    /// Vrai s'il était le projet de l'atelier.
    courant: bool,
}

/// Ce qui était affiché juste avant un changement d'écran, de projet ou de thème, et qui
/// s'efface par-dessus la suite. Purement visuel : le nouvel écran répond tout de suite.
#[derive(Debug)]
struct Passage {
    /// L'image d'avant.
    fantome: Fantome,
    /// La durée du fondu, en secondes.
    duree: f32,
    /// Sa courbe.
    courbe: Courbe,
    /// L'échelle de l'image d'avant à la fin du fondu (1 : elle ne bouge pas).
    sortie: f32,
    /// L'échelle de ce qui arrive au début du fondu (1 : il ne bouge pas).
    entree: f32,
}

/// Ce qui, en changeant pendant une image, déclenche un fondu : l'écran, le thème et le projet
/// ouvert.
type Decor = (Ecran, Mode, u64);

/// L'application complète.
pub struct Application {
    /// L'écran affiché.
    pub ecran: Ecran,
    /// L'aide et l'éditeur de programme, par-dessus l'écran.
    pub fenetres: Fenetres,
    /// L'état de l'écran d'accueil.
    pub accueil: Accueil,
    /// Les choix de l'utilisateur, par-dessus l'apparence du fichier.
    pub reglages: Reglages,
    /// Les projets enregistrés, relus en arrivant sur l'accueil et après chaque changement.
    pub projets: Vec<FicheProjet>,
    /// L'apparence telle que lue dans `apparence.json`, avant les réglages.
    apparence_fichier: Apparence,
    /// Vrai si le projet de l'atelier est à reprendre (rouvert ou ouvert pendant la séance).
    reprise: bool,
    /// Le dernier projet mis à la corbeille.
    jete: Option<Jete>,
    /// Le titre de la fenêtre déjà envoyé.
    titre: String,
    /// Couleurs, formes, mouvements et seuils.
    pub apparence: Apparence,
    /// L'état logique.
    pub atelier: Atelier,
    /// Le plan interactif.
    pub plan: Plan,
    /// Les panneaux.
    pub panneaux: Panneaux,
    /// Les valeurs animées.
    pub anims: Animations,
    /// La dernière mise en page calculée.
    pub disposition: Option<Disposition>,
    /// Où les projets sont enregistrés ; `None` : rien n'est écrit sur le disque.
    pub dossier: Option<Dossier>,
    surveillance: Surveillance,
    polices: bool,
    mode_applique: Option<Mode>,
    erreur_montree: Option<String>,
    /// Ce qui est enregistré du projet de l'atelier ; `None` : à écrire dès que possible.
    enregistre: Option<Enregistre>,
    /// Instant de la première modification pas encore enregistrée.
    modifie_depuis: Option<f64>,
    /// Vrai après un échec d'écriture, pour ne le dire qu'une fois.
    echec_dit: bool,
    /// Version du parcours déjà écrite sur le disque.
    parcours_enregistre: u64,
    /// Vrai si l'avancement du disque est illisible et n'a pas pu être mis de côté : il n'est
    /// alors plus écrit, pour ne pas le remplacer par un avancement vide.
    parcours_fige: bool,
    /// Les mises à jour automatiques ; `None` : désactivées (essais, application sans réseau).
    pub mise_a_jour: Option<MiseAJour>,
    /// Le fondu en cours entre deux écrans, deux projets ou deux thèmes.
    passage: Option<Passage>,
    /// Le nombre de projets ouverts depuis le lancement.
    ouvertures: u64,
    /// L'instant de la première image dessinée, pour l'apparition du lancement.
    lancement: Option<f64>,
    /// Les raccourcis du bureau et du menu Démarrer ; `None` : ni Windows, ni lancement réel
    /// (essais).
    pub raccourcis: Option<Raccourcis>,
    /// Les raccourcis présents, relus au plus une fois par seconde, et quand.
    raccourcis_presents: ([bool; 2], Option<std::time::Instant>),
    /// La photo du plan en cours (« Exporter en image »).
    photo: Option<Photo>,
}

/// Ce qui est enregistré du projet de l'atelier.
struct Enregistre {
    /// La version de l'atelier déjà écrite.
    version: u64,
    /// Le fichier du projet, ou celui qu'il aura à sa première écriture.
    fichier: String,
    /// Le nom du projet quand ce fichier est devenu le sien : tant qu'il le garde, il y reste.
    nom: String,
    /// Faux tant que le projet n'a été ni lu ni écrit sur le disque.
    sur_disque: bool,
}

impl Enregistre {
    /// L'atelier tel qu'il est, rangé dans ce fichier.
    fn de(atelier: &Atelier, fichier: String, sur_disque: bool) -> Self {
        Enregistre {
            version: atelier.version,
            fichier,
            nom: atelier.projet.nom.clone(),
            sur_disque,
        }
    }
}

impl Application {
    /// L'application avec l'apparence donnée. `surveillance` relit le fichier d'apparence.
    pub fn nouvelle(apparence: Apparence, surveillance: Surveillance) -> Self {
        let atelier = Atelier::nouveau(&apparence);
        Application {
            ecran: Ecran::Atelier,
            accueil: Accueil::default(),
            reglages: Reglages::default(),
            projets: Vec::new(),
            apparence_fichier: apparence.clone(),
            reprise: true,
            jete: None,
            fenetres: Fenetres::default(),
            titre: String::new(),
            apparence,
            atelier,
            plan: Plan::default(),
            panneaux: Panneaux::default(),
            anims: Animations::default(),
            disposition: None,
            dossier: None,
            surveillance,
            polices: false,
            mode_applique: None,
            erreur_montree: None,
            enregistre: None,
            modifie_depuis: None,
            echec_dit: false,
            parcours_enregistre: 0,
            parcours_fige: false,
            mise_a_jour: None,
            passage: None,
            ouvertures: 0,
            lancement: None,
            raccourcis: None,
            raccourcis_presents: ([false; 2], None),
            photo: None,
        }
    }

    /// Vrai pendant un fondu entre deux écrans, deux projets ou deux thèmes.
    pub fn en_transition(&self) -> bool {
        self.passage.is_some()
    }

    /// Range les projets dans ce dossier, applique les réglages qui y sont enregistrés, et
    /// rouvre le projet de la dernière fois s'il y en a un.
    pub fn avec_dossier(mut self, dossier: Dossier) -> Self {
        let rapports = incident::dossier_des_rapports(dossier.racine());
        if incident::rapport_non_lu(&rapports).is_some() {
            incident::marquer_lus(&rapports);
            self.atelier.proposer(
                "L'application s'est arrêtée brusquement la dernière fois. Ton projet avait été enregistré juste avant ; un rapport décrit ce qui s'est passé.",
                Proposition::VoirLeRapport,
            );
        }
        let (reglages, complets) = dossier.reglages_lus();
        self.reglages = reglages;
        self.appliquer_reglages();
        if !complets {
            self.atelier.dire_longtemps(
                "reglages.json est mal écrit : ce qui a pu être lu est appliqué, le reste garde sa valeur par défaut.",
            );
        }
        let dernier = dossier.dernier().or_else(|| {
            Some(nom_de_fichier(&self.atelier.projet.nom)).filter(|f| dossier.existe(f))
        });
        self.reprise = dernier.is_some();
        if let Some(fichier) = dernier {
            match dossier.ouvrir(&fichier) {
                Ok(projet) => {
                    let invite = projet.quete.is_some();
                    self.atelier = Atelier::avec(projet, &self.apparence);
                    self.atelier.invite = invite;
                    self.enregistre = Some(Enregistre::de(&self.atelier, fichier, true));
                }
                Err(e) => {
                    self.reprise = false;
                    self.atelier.dire_longtemps(&format!(
                        "Le projet « {fichier} » n'a pas pu être rouvert : {e}"
                    ));
                    // Le plan de démonstration reste en mémoire sans jamais être écrit : ni
                    // sur le projet illisible, ni sur un projet qui porterait son nom.
                    let libre = nom_de_fichier(&dossier.nom_libre(&self.atelier.projet.nom));
                    self.enregistre = Some(Enregistre::de(&self.atelier, libre, false));
                }
            }
        } else {
            // Un projet tout neuf n'est écrit qu'à sa première modification.
            let fichier = nom_de_fichier(&self.atelier.projet.nom);
            self.enregistre = Some(Enregistre::de(&self.atelier, fichier, false));
        }
        match dossier.parcours() {
            Ok(Some(p)) => self.atelier.parcours = p,
            Ok(None) => {}
            // Illisible : la prochaine quête réussie écrirait un avancement vide par-dessus.
            // Le fichier est donc gardé sous un autre nom, où l'on peut encore le réparer.
            Err(e) => match dossier.garder_le_parcours_illisible() {
                Ok(garde) => self.atelier.dire_longtemps(&format!(
                    "Ton avancement n'a pas pu être relu ({e}) : il repart de zéro. L'ancien fichier est gardé sous le nom « {garde} »."
                )),
                Err(_) => {
                    self.parcours_fige = true;
                    self.atelier.dire_longtemps(&format!(
                        "Ton avancement n'a pas pu être relu ({e}). Il ne sera pas enregistré pendant cette séance, pour ne pas effacer parcours.json."
                    ));
                }
            },
        }
        self.parcours_enregistre = self.atelier.parcours.version;
        self.dossier = Some(dossier);
        self.lister_les_projets();
        self.relire_les_projets();
        self
    }

    /// L'écran du lancement : l'accueil, sauf si l'utilisateur préfère reprendre directement
    /// son dernier projet et qu'il y en a un.
    pub fn au_lancement(mut self) -> Self {
        if self.reglages.au_lancement == Lancement::Accueil || !self.reprise {
            if !self.reprise {
                // Rien à reprendre : l'accueil propose la première quête, l'astuce du plan de
                // démonstration n'a pas lieu d'être. Un message d'erreur, lui, reste.
                self.atelier.messages.retain(|m| m.texte != ASTUCE_DU_PLAN);
            }
            self.relire_les_projets();
            self.accueil.aller(Rubrique::Accueil);
            self.ecran = Ecran::Accueil;
        }
        self
    }

    /// L'apparence du fichier, avec les réglages de l'utilisateur par-dessus.
    fn appliquer_reglages(&mut self) {
        self.apparence = self.reglages.appliquer(&self.apparence_fichier);
    }

    /// Enregistre les réglages, s'il y a un dossier.
    fn enregistrer_reglages(&mut self) {
        if let Some(dossier) = &self.dossier {
            if let Err(e) = dossier.enregistrer_reglages(&self.reglages) {
                self.atelier
                    .dire(&format!("Tes réglages n'ont pas pu être enregistrés : {e}"));
            }
        }
    }

    /// Relit la liste des projets enregistrés, pour l'accueil.
    pub fn relire_les_projets(&mut self) {
        self.projets = self
            .dossier
            .as_ref()
            .map(Dossier::fiches)
            .unwrap_or_default();
    }

    /// Le nom de fichier du projet de l'atelier, s'il est à reprendre.
    pub fn courant(&self) -> Option<String> {
        self.reprise.then(|| self.fichier_courant())
    }

    /// Le fichier où le projet de l'atelier est enregistré, ou le sera : celui de son dernier
    /// enregistrement (même s'il a été renommé depuis), sinon celui de son nom.
    fn fichier_courant(&self) -> String {
        self.enregistre.as_ref().map_or_else(
            || nom_de_fichier(&self.atelier.projet.nom),
            |e| e.fichier.clone(),
        )
    }

    /// Montre l'accueil, après avoir enregistré le projet.
    pub fn ouvrir_accueil(&mut self) {
        self.panneaux.abandonner_saisie();
        self.enregistrer();
        self.relire_les_projets();
        self.accueil.aller(Rubrique::Accueil);
        self.ecran = Ecran::Accueil;
    }

    /// Revient au projet de l'atelier, s'il y en a un à reprendre.
    pub fn continuer(&mut self) {
        if self.reprise {
            self.ecran = Ecran::Atelier;
        }
    }

    fn lister_les_projets(&mut self) {
        let courant = self.fichier_courant();
        self.panneaux.projets = self
            .dossier
            .as_ref()
            .map(Dossier::lister)
            .unwrap_or_default()
            .into_iter()
            .filter(|n| *n != courant)
            .collect();
    }

    /// Enregistre le projet s'il a changé. Sans dossier, ne fait rien.
    pub fn enregistrer(&mut self) {
        let Some(dossier) = &self.dossier else {
            return;
        };
        let at = &mut self.atelier;
        if self
            .enregistre
            .as_ref()
            .is_some_and(|e| e.version == at.version)
        {
            self.modifie_depuis = None;
            return;
        }
        let voulu = at.projet.nom.clone();
        let resultat = match &self.enregistre {
            // Tant qu'il garde son nom, le projet reste dans son fichier, même si ce fichier
            // ne s'appelle pas tout à fait comme lui (copié ou renommé hors de l'application).
            Some(e) if e.sur_disque && e.nom == voulu => dossier
                .enregistrer_sous(&at.projet, &e.fichier)
                .map(|()| e.fichier.clone()),
            // Il a changé de nom : il quitte son ancien fichier, sans écraser un autre projet
            // qui porterait déjà ce nom.
            Some(e) if e.sur_disque => dossier.deplacer(&mut at.projet, &e.fichier),
            // Jamais écrit : il ne remplace pas un projet arrivé entre-temps sous son nom.
            _ => {
                if dossier.existe(&nom_de_fichier(&voulu)) {
                    at.projet.nom = dossier.nom_libre(&voulu);
                }
                dossier.enregistrer(&at.projet)
            }
        };
        if at.projet.nom != voulu {
            at.dire(&format!(
                "Un autre projet s'appelle déjà « {voulu} » : celui-ci devient « {} ».",
                at.projet.nom
            ));
        }
        match resultat {
            Ok(fichier) => {
                let _ = dossier.noter_dernier(&fichier);
                self.enregistre = Some(Enregistre::de(at, fichier, true));
                self.echec_dit = false;
                self.modifie_depuis = None;
            }
            Err(e) => {
                if !self.echec_dit {
                    at.dire_longtemps(&format!(
                        "Le projet n'a pas pu être enregistré dans {} : {e}",
                        dossier.racine().display()
                    ));
                }
                self.echec_dit = true;
                // Rien n'est noté comme enregistré : un nouvel essai aura lieu après le délai
                // habituel, et à la fermeture.
                self.modifie_depuis = Some(at.horloge);
            }
        }
        self.lister_les_projets();
    }

    /// Enregistre tout seul, un instant après la dernière modification.
    fn enregistrer_si_besoin(&mut self) {
        let Some(dossier) = &self.dossier else {
            return;
        };
        // Une quête réussie s'enregistre tout de suite : c'est rare et précieux.
        if self.parcours_enregistre != self.atelier.parcours.version && !self.parcours_fige {
            self.parcours_enregistre = self.atelier.parcours.version;
            if let Err(e) = dossier.enregistrer_parcours(&self.atelier.parcours) {
                self.atelier
                    .dire(&format!("Ton avancement n'a pas pu être enregistré : {e}"));
            }
        }
        let at = &self.atelier;
        if self
            .enregistre
            .as_ref()
            .is_some_and(|e| e.version == at.version)
        {
            self.modifie_depuis = None;
            return;
        }
        let depuis = *self.modifie_depuis.get_or_insert(at.horloge);
        if at.horloge - depuis >= DELAI_ENREGISTREMENT {
            self.enregistrer();
        }
    }

    /// Ouvre un autre projet à la place de celui qui est affiché.
    pub fn ouvrir(&mut self, quoi: Ouverture) {
        self.enregistrer();
        let a = &self.apparence;
        // Le fichier d'où vient le projet ouvert, s'il était déjà enregistré.
        let mut depuis = None;
        // Vrai pour un projet qui n'a pas besoin d'être écrit tant qu'on ne l'a pas modifié.
        let mut intact = false;
        let ouvert = match quoi {
            Ouverture::Nouveau => {
                let nom = match &self.dossier {
                    Some(d) => d.nom_libre("Nouveau projet"),
                    None => "Nouveau projet".to_owned(),
                };
                self.atelier
                    .ouvrir(Projet::nouveau(&nom, Document::nouveau()), a);
                self.atelier.dire(
                    "Un projet vide. Glisse des éléments depuis la palette, relie-les, puis regroupe-les en briques.",
                );
                true
            }
            Ouverture::Quete(cle) => match Quete::fiche(&cle) {
                Some(fiche) => {
                    let nom = match &self.dossier {
                        Some(d) => d.nom_libre(fiche.titre),
                        None => fiche.titre.to_owned(),
                    };
                    self.atelier.ouvrir_quete(fiche, &nom, a)
                }
                None => false,
            },
            Ouverture::Exemple(rang) => EXEMPLES.get(rang).is_some_and(|e| {
                // Une copie neuve, sans écraser celle qu'on a peut-être déjà modifiée.
                let nom = match &self.dossier {
                    Some(d) => d.nom_libre(e.nom),
                    None => e.nom.to_owned(),
                };
                intact = true;
                self.atelier.ouvrir_exemple(e, &nom, a)
            }),
            Ouverture::Projet(fichier) => match self.dossier.as_ref().map(|d| d.ouvrir(&fichier)) {
                Some(Ok(projet)) => {
                    self.atelier.ouvrir(projet, a);
                    depuis = Some(fichier);
                    true
                }
                Some(Err(e)) => {
                    self.atelier
                        .dire(&format!("« {fichier} » n'a pas pu être ouvert : {e}"));
                    false
                }
                None => false,
            },
        };
        if ouvert {
            match depuis {
                // Un projet enregistré reste dans son fichier, et rien n'est écrit avant sa
                // prochaine modification. Copié ou renommé hors de l'application, il a pris le
                // nom de ce fichier (`Dossier::ouvrir`) : il ne remplace aucun autre projet.
                Some(fichier) => {
                    if let Some(dossier) = &self.dossier {
                        let _ = dossier.noter_dernier(&fichier);
                    }
                    self.enregistre = Some(Enregistre::de(&self.atelier, fichier, true));
                    self.lister_les_projets();
                }
                // Un exemple se retrouve toujours dans la liste des exemples : il n'est écrit
                // que si on le modifie. Le regarder dix fois ne laisse pas dix copies.
                None if intact => {
                    let fichier = nom_de_fichier(&self.atelier.projet.nom);
                    self.enregistre = Some(Enregistre::de(&self.atelier, fichier, false));
                    self.lister_les_projets();
                }
                // Un nouveau projet, sous un nom libre, est enregistré tout de suite.
                None => {
                    self.enregistre = None;
                    self.enregistrer();
                }
            }
            self.modifie_depuis = None;
            self.panneaux.defilement_infos = 0.0;
            self.panneaux.oublier_les_places();
            self.reprise = true;
            self.ouvertures += 1;
            self.ecran = Ecran::Atelier;
            // Le message qui proposait de ramener un projet jeté est parti avec l'ancien.
            self.jete = None;
        }
    }

    /// L'application lancée par eframe : polices installées avant la première image, apparence
    /// lue dans `apparence.json` s'il existe.
    pub fn pour_eframe(cc: &eframe::CreationContext<'_>) -> Self {
        let mut surveillance = Surveillance::nouvelle();
        let apparence = surveillance.relire(0.0).unwrap_or_default();
        installer_polices(&cc.egui_ctx);
        let mut app = Application::nouvelle(apparence, surveillance);
        if let Some(dossier) = Dossier::par_defaut() {
            app = app.avec_dossier(dossier);
        }
        app.polices = true;
        app = app.au_lancement();
        // Recherche d'une nouvelle version en arrière-plan, sauf si l'utilisateur l'a coupée.
        mise_a_jour::retirer_une_vieille_copie_de_secours();
        let mut maj = MiseAJour::nouvelle(Source::officielle());
        if app.reglages.mises_a_jour_automatiques
            && std::env::var_os("CE_SANS_MISE_A_JOUR").is_none()
        {
            maj.rechercher(false);
        }
        app.mise_a_jour = Some(maj);
        // Installée par l'installateur : la version affichée dans les Paramètres de Windows suit
        // les mises à jour automatiques, qui ne repassent pas par l'installateur.
        crate::installation::tenir_la_version_a_jour(env!("CARGO_PKG_VERSION"));
        if std::env::var_os("CE_SANS_RACCOURCI").is_none() {
            if let Some(raccourcis) = Raccourcis::du_systeme() {
                app = app.avec_raccourcis(raccourcis);
            }
        }
        app
    }

    /// Gère ces raccourcis : au premier lancement, les pose sur le bureau et dans le menu
    /// Démarrer (une seule fois, retenu dans les réglages) ; ensuite, refait ceux qui mènent à
    /// un autre exécutable. Sans dossier de données, rien ne serait retenu : rien n'est posé.
    pub fn avec_raccourcis(mut self, raccourcis: Raccourcis) -> Self {
        let proposes = self.reglages.raccourcis_proposes
            || self.dossier.is_none()
            || crate::installation::par_l_installateur();
        let bilan = raccourcis.au_lancement(proposes);
        if bilan.proposes {
            self.reglages.raccourcis_proposes = true;
            self.enregistrer_reglages();
        }
        if let Some(texte) = bilan.message() {
            if bilan.erreurs.is_empty() {
                self.atelier.dire(&texte);
            } else {
                self.atelier.dire_longtemps(&texte);
            }
        }
        self.raccourcis = Some(raccourcis);
        self.relire_les_raccourcis(true);
        self
    }

    /// Relit les raccourcis présents, au plus une fois par seconde sauf si `tout_de_suite`.
    fn relire_les_raccourcis(&mut self, tout_de_suite: bool) {
        let Some(raccourcis) = &self.raccourcis else {
            return;
        };
        let (_, lu) = self.raccourcis_presents;
        if tout_de_suite || lu.is_none_or(|t| t.elapsed().as_secs_f32() >= 1.0) {
            self.raccourcis_presents = (raccourcis.presents(), Some(std::time::Instant::now()));
        }
    }

    /// Efface l'avancement dans le parcours, après en avoir gardé une copie dans le dossier
    /// des données. Refusé si l'avancement enregistré n'a pas pu être relu : il n'est alors
    /// jamais touché.
    pub fn recommencer_le_parcours(&mut self) {
        if self.parcours_fige {
            self.atelier.dire(
                "Ton avancement enregistré n'a pas pu être relu : il n'est pas touché. Mets l'application à jour ou regarde le dossier de tes données.",
            );
            return;
        }
        let mut garde = None;
        if let Some(dossier) = &self.dossier {
            match dossier.garder_le_parcours_avant_recommencement() {
                Ok(g) => garde = g,
                Err(e) => {
                    self.atelier.dire(&format!(
                        "Le parcours n'a pas été effacé : sa copie de sécurité n'a pas pu être écrite ({e})."
                    ));
                    return;
                }
            }
        }
        let version = self.atelier.parcours.version + 1;
        self.atelier.parcours = crate::parcours::Parcours::default();
        self.atelier.parcours.version = version;
        self.atelier.dire(&match garde {
            Some(g) => format!(
                "Le parcours recommence à zéro. L'ancien avancement est gardé dans « {g} »."
            ),
            None => "Le parcours recommence à zéro.".to_owned(),
        });
    }

    /// Cherche une nouvelle version tout de suite, à la demande de l'utilisateur.
    pub fn rechercher_une_mise_a_jour(&mut self) {
        let Some(maj) = &mut self.mise_a_jour else {
            self.atelier
                .dire("Les mises à jour automatiques sont désactivées ici.");
            return;
        };
        match maj.etat.clone() {
            // Déjà installée : il ne reste qu'à redémarrer.
            Etat::Installee(v) => self
                .atelier
                .proposer(&texte_installee(&v), Proposition::Redemarrer),
            Etat::Installation => self
                .atelier
                .dire("La nouvelle version est déjà en train de s'installer…"),
            _ => {
                maj.rechercher(true);
                self.atelier.dire("Recherche d'une nouvelle version…");
            }
        }
    }

    /// Répond à une action choisie dans un message.
    fn agir(&mut self, action: Proposition) {
        match action {
            Proposition::MettreAJour => {
                let Some(maj) = &mut self.mise_a_jour else {
                    return;
                };
                maj.installer();
                match maj.etat.clone() {
                    Etat::Installation => self.atelier.dire(
                        "Téléchargement de la nouvelle version, puis vérification de sa signature…",
                    ),
                    Etat::Installee(v) => self
                        .atelier
                        .proposer(&texte_installee(&v), Proposition::Redemarrer),
                    _ => self.atelier.dire(
                        "Une recherche de version est en cours : la mise à jour sera proposée à nouveau dans un instant.",
                    ),
                }
            }
            Proposition::Redemarrer => {
                self.enregistrer();
                let erreur = mise_a_jour::redemarrer();
                self.atelier.dire_longtemps(&format!(
                    "Le redémarrage n'a pas pu se faire ({erreur}). Ferme et relance l'application : la nouvelle version est en place."
                ));
            }
            Proposition::Restaurer => self.restaurer(),
            Proposition::VoirLeRapport => {
                if let Some(dossier) = &self.dossier {
                    let rapports = incident::dossier_des_rapports(dossier.racine());
                    if let Err(e) = incident::ouvrir_dans_l_explorateur(&rapports) {
                        self.atelier.dire_longtemps(&format!(
                            "Le dossier n'a pas pu être ouvert ({e}) : il se trouve dans {}",
                            rapports.display()
                        ));
                    }
                }
            }
        }
    }

    /// Ramène de la corbeille le dernier projet qu'on y a mis.
    fn restaurer(&mut self) {
        let (Some(jete), Some(dossier)) = (self.jete.take(), &self.dossier) else {
            return;
        };
        let fichier = jete.fichier;
        match dossier.restaurer(&jete.corbeille, &fichier) {
            Ok(retrouve) => {
                if jete.courant && retrouve == fichier {
                    // L'atelier l'a gardé en mémoire : on peut de nouveau le reprendre.
                    self.reprise = true;
                    let _ = dossier.noter_dernier(&retrouve);
                }
                self.atelier
                    .dire(&format!("« {retrouve} » est de retour dans tes projets."));
            }
            Err(e) => self
                .atelier
                .dire_longtemps(&format!("« {fichier} » n'a pas pu être ramené : {e}")),
        }
        self.relire_les_projets();
        self.lister_les_projets();
    }

    /// L'état des mises à jour, en quelques mots, pour l'accueil.
    fn etat_de_la_mise_a_jour(&self) -> String {
        let Some(maj) = &self.mise_a_jour else {
            return "mises à jour désactivées ici".to_owned();
        };
        // Court : la phrase suit « Version 0.1.0 · » au bas du menu de l'accueil.
        match &maj.etat {
            Etat::Repos if self.reglages.mises_a_jour_automatiques => {
                "mises à jour automatiques".to_owned()
            }
            Etat::Repos => "mises à jour manuelles".to_owned(),
            Etat::Recherche { .. } => "recherche en cours…".to_owned(),
            Etat::Disponible(n) => format!("{} disponible", n.version),
            Etat::Installation => "installation en cours…".to_owned(),
            Etat::Installee(v) => format!("{v} prête au redémarrage"),
        }
    }

    /// Exécute ce que l'accueil demande.
    fn executer(&mut self, demande: Demande, ctx: &egui::Context) {
        match demande {
            Demande::Continuer => self.continuer(),
            Demande::Ouvrir(quoi) => self.ouvrir(quoi),
            Demande::Renommer { fichier, nom } => {
                if self.courant().as_deref() == Some(fichier.as_str()) {
                    // Le projet de l'atelier : l'enregistrement renomme aussi son fichier.
                    if self.atelier.renommer_projet(&nom) {
                        self.enregistrer();
                    }
                } else if let Some(dossier) = &self.dossier {
                    match dossier.renommer(&fichier, &nom) {
                        Ok(nouveau) if nouveau != fichier && nouveau != nom_de_fichier(&nom) => {
                            self.atelier.dire(&format!(
                                "Ce nom était pris : le projet s'appelle « {nouveau} »."
                            ))
                        }
                        Ok(_) => {}
                        Err(e) => self
                            .atelier
                            .dire(&format!("« {fichier} » n'a pas pu être renommé : {e}")),
                    }
                }
                self.relire_les_projets();
                self.lister_les_projets();
            }
            Demande::Dupliquer(fichier) => {
                if self.courant().as_deref() == Some(fichier.as_str()) {
                    self.enregistrer();
                }
                if let Some(dossier) = &self.dossier {
                    match dossier.dupliquer(&fichier) {
                        Ok(copie) => self
                            .atelier
                            .dire(&format!("Copie enregistrée : « {copie} »")),
                        Err(e) => self
                            .atelier
                            .dire(&format!("« {fichier} » n'a pas pu être dupliqué : {e}")),
                    }
                }
                self.relire_les_projets();
                self.lister_les_projets();
            }
            Demande::Jeter(fichier) => {
                let etait_courant = self.courant().as_deref() == Some(fichier.as_str());
                if etait_courant {
                    self.enregistrer();
                }
                if let Some(dossier) = &self.dossier {
                    match dossier.jeter(&fichier) {
                        Ok(corbeille) => {
                            if etait_courant {
                                // Il reste en mémoire, mais ne se reprend plus et ne
                                // s'enregistre plus : l'atelier n'est pas modifié d'ici.
                                self.reprise = false;
                            }
                            self.jete = Some(Jete {
                                corbeille,
                                fichier: fichier.clone(),
                                courant: etait_courant,
                            });
                            self.atelier.proposer(
                                &format!("« {fichier} » est dans la corbeille."),
                                Proposition::Restaurer,
                            );
                        }
                        Err(e) => self.atelier.dire(&format!(
                            "« {fichier} » n'a pas pu être mis à la corbeille : {e}"
                        )),
                    }
                }
                self.relire_les_projets();
                self.lister_les_projets();
            }
            Demande::Reglages(reglages) => {
                let vitesse = reglages.vitesse != self.reglages.vitesse;
                self.reglages = reglages;
                self.appliquer_reglages();
                // Le projet ouvert suit, sauf s'il tourne en vitesse rapide (un ordinateur) :
                // on ne le ralentit pas dans son dos.
                if vitesse && self.atelier.vitesse < self.apparence.mouvement.vitesses.len() {
                    self.atelier.vitesse = self.apparence.mouvement.vitesse_initiale;
                }
                self.enregistrer_reglages();
            }
            Demande::ChercherMiseAJour => self.rechercher_une_mise_a_jour(),
            Demande::Raccourci { lieu, poser } => self.changer_un_raccourci(lieu, poser),
            Demande::OuvrirDossier => {
                if let Some(dossier) = &self.dossier {
                    if let Err(e) = montrer_le_dossier(dossier.racine()) {
                        self.atelier.dire(&format!(
                            "Le dossier {} n'a pas pu être ouvert : {e}",
                            dossier.racine().display()
                        ));
                    }
                }
            }
            Demande::RecommencerParcours => self.recommencer_le_parcours(),
            Demande::Quitter => {
                self.enregistrer();
                ctx.send_viewport_cmd(egui::ViewportCommand::Close);
            }
            Demande::Action(action) => self.agir(action),
        }
    }

    /// Pose ou retire le raccourci de ce lieu, choisi dans Paramètres.
    fn changer_un_raccourci(&mut self, lieu: Lieu, poser: bool) {
        let Some(raccourcis) = &self.raccourcis else {
            return;
        };
        let fait = if poser {
            raccourcis.poser(lieu)
        } else {
            raccourcis.retirer(lieu)
        };
        match fait {
            Ok(()) if poser => self
                .atelier
                .dire(&format!("Raccourci ajouté {}.", lieu.dans_une_phrase())),
            Ok(()) => self
                .atelier
                .dire(&format!("Raccourci retiré {}.", lieu.d_ou())),
            Err(e) => self.atelier.dire_longtemps(&format!(
                "Le raccourci {} n'a pas pu être {} : {e}",
                lieu.dans_une_phrase(),
                if poser { "posé" } else { "retiré" }
            )),
        }
        self.relire_les_raccourcis(true);
    }

    /// Une image de l'accueil.
    fn image_accueil(&mut self, ui: &mut egui::Ui, dt: f32) {
        let ctx = ui.ctx().clone();
        self.atelier.laisser_passer(dt);
        self.anims.en_mouvement = false;
        let courant = self.courant();
        let dossier = self
            .dossier
            .as_ref()
            .map(|d| d.racine().display().to_string());
        let maj = self.etat_de_la_mise_a_jour();
        if self.accueil.rubrique == Rubrique::Parametres {
            // Un raccourci supprimé à la main pendant que Paramètres est ouvert s'y voit vite.
            self.relire_les_raccourcis(false);
        }
        let raccourcis = self.raccourcis.as_ref().map(|r| {
            if r.depuis_l_archive() {
                None
            } else {
                Some(self.raccourcis_presents.0)
            }
        });
        let cx = Contexte {
            projets: &self.projets,
            courant: courant.as_deref(),
            reglages: &self.reglages,
            dossier: dossier.as_deref(),
            mise_a_jour: &maj,
            raccourcis,
            maintenant: SystemTime::now(),
        };
        let demandes = self.accueil.montrer(
            ui,
            &self.apparence,
            &mut self.atelier,
            &mut self.anims,
            &cx,
            dt,
        );
        for demande in demandes {
            self.executer(demande, &ctx);
        }
        self.suivre_la_mise_a_jour();
        self.enregistrer_si_besoin();
        let at = &self.atelier;
        if self.anims.en_mouvement || !self.apparence.mouvement.sobre {
            ctx.request_repaint();
        } else if at
            .messages
            .iter()
            .any(|m| m.duree.is_some() || at.horloge - m.ne < 0.5)
        {
            ctx.request_repaint_after(std::time::Duration::from_millis(100));
        } else if self.surveillance.chemin().is_some()
            || self.mise_a_jour.as_ref().is_some_and(MiseAJour::occupee)
        {
            ctx.request_repaint_after(std::time::Duration::from_millis(500));
        }
    }

    /// Met le titre de la fenêtre à jour : le nom du projet ouvert.
    fn titrer(&mut self, ctx: &egui::Context) {
        let titre = match self.ecran {
            Ecran::Accueil => "Computer Engineer".to_owned(),
            Ecran::Atelier => format!("{} · Computer Engineer", self.atelier.projet.nom),
        };
        if titre != self.titre {
            ctx.send_viewport_cmd(egui::ViewportCommand::Title(titre.clone()));
            self.titre = titre;
        }
    }

    /// Relève le résultat d'une recherche ou d'une installation et le dit à l'utilisateur.
    fn suivre_la_mise_a_jour(&mut self) {
        let Some(maj) = &mut self.mise_a_jour else {
            return;
        };
        let Some(nouvelle) = maj.relever() else {
            return;
        };
        let actuelle = maj.source.version.clone();
        let at = &mut self.atelier;
        match nouvelle {
            Nouvelle::Recherche {
                resultat: Ok(Some(n)),
                ..
            } => {
                let resume = resume_des_notes(&n.notes);
                let texte = if resume.is_empty() {
                    format!(
                        "La version {} de Computer Engineer est disponible.",
                        n.version
                    )
                } else {
                    format!(
                        "La version {} de Computer Engineer est disponible : {resume}",
                        n.version
                    )
                };
                at.proposer(&texte, Proposition::MettreAJour);
            }
            Nouvelle::Recherche {
                demandee: true,
                resultat: Ok(None),
            } => at.dire(&format!("Tu as la dernière version ({actuelle}).")),
            Nouvelle::Recherche {
                demandee: true,
                resultat: Err(e),
            } => at.dire_longtemps(&format!(
                "Impossible de chercher une nouvelle version : {e}."
            )),
            // Recherche automatique sans réseau : rien à dire.
            Nouvelle::Recherche { .. } => {}
            Nouvelle::Installation(Ok(Some(v))) => {
                at.proposer(&texte_installee(&v), Proposition::Redemarrer)
            }
            Nouvelle::Installation(Ok(None)) => {
                at.dire(&format!("Tu as déjà la dernière version ({actuelle})."));
            }
            Nouvelle::Installation(Err(e)) => {
                at.dire_longtemps(&format!("La mise à jour n'a pas pu s'installer : {e}."))
            }
        }
    }

    /// Une image : le temps passe, les gestes sont traités, tout est redessiné.
    pub fn image(&mut self, ui: &mut egui::Ui) {
        let ctx = ui.ctx().clone();
        if !self.polices {
            // Les polices ne sont disponibles qu'à l'image suivante.
            installer_polices(&ctx);
            self.polices = true;
            ui.painter()
                .rect_filled(ctx.content_rect(), 0.0, self.apparence.palette().fond.c());
            ctx.request_repaint();
            return;
        }
        let dt = ui.input(|i| i.stable_dt).clamp(0.0, 0.1);
        // Tout ce qui sera peint à partir d'ici forme l'image : de quoi la fondre avec la
        // suivante quand l'écran, le projet ou le thème change.
        let debut = calques::repere(ui.painter());

        // Apparence relue à chaud ; les réglages de l'utilisateur (dont le thème choisi dans
        // l'application) l'emportent sur le fichier.
        if let Some(nouvelle) = self.surveillance.relire(self.atelier.horloge) {
            self.apparence_fichier = nouvelle;
            self.appliquer_reglages();
            self.mode_applique = None;
            self.atelier.dire("Apparence rechargée.");
        }
        if self.surveillance.erreur != self.erreur_montree {
            self.erreur_montree = self.surveillance.erreur.clone();
            if let Some(e) = &self.erreur_montree {
                self.atelier.dire(&format!("Apparence non relue : {e}"));
            }
        }
        if self.mode_applique != Some(self.apparence.mode) {
            let apparence = self.apparence.clone();
            ctx.all_styles_mut(|style| crate::panneaux::style_egui(&apparence, style));
            self.mode_applique = Some(self.apparence.mode);
        }
        self.titrer(&ctx);
        let avant = self.decor();
        match self.ecran {
            Ecran::Accueil => self.image_accueil(ui, dt),
            Ecran::Atelier => self.image_atelier(ui, dt),
        }
        self.fondre(ui, debut, avant);
        // F1 partout ; Ctrl+P, Ctrl+E et Ctrl+T dans l'atelier, sauf pendant une saisie.
        let (f1, ctrl_p, ctrl_e, ctrl_t) = ui.input(|i| {
            (
                i.key_pressed(Key::F1),
                i.modifiers.command && i.key_pressed(Key::P),
                i.modifiers.command && i.key_pressed(Key::E),
                i.modifiers.command && i.key_pressed(Key::T),
            )
        });
        if f1 {
            self.fenetres.basculer_aide();
        }
        let saisie = self.panneaux.en_saisie() || ctx.egui_wants_keyboard_input();
        if ctrl_p && self.ecran == Ecran::Atelier && !self.panneaux.en_saisie() {
            self.fenetres.ouvrir_l_editeur(&mut self.atelier);
        }
        if ctrl_e && self.ecran == Ecran::Atelier && !saisie {
            self.fenetres.basculer_analyse(&self.atelier);
        }
        if ctrl_t && self.ecran == Ecran::Atelier && !saisie {
            self.fenetres.basculer_banc(&self.atelier);
        }
        let atelier = self.ecran == Ecran::Atelier;
        if self.photo.is_some() {
            // Pendant une photo, le plan est seul à l'écran.
            return;
        }
        self.fenetres
            .montrer(&ctx, &mut self.atelier, &self.apparence, atelier);
        if std::mem::take(&mut self.fenetres.exporter) {
            self.exporter_en_verilog();
        }
    }

    /// Écrit le plan affiché en Verilog dans le dossier `exports` des données, et le montre.
    fn exporter_en_verilog(&mut self) {
        let at = &self.atelier;
        let nom = at
            .lieu()
            .map_or_else(|| at.projet.nom.clone(), str::to_owned);
        let texte = match crate::export::verilog(&at.projet, &at.doc, &nom) {
            Ok(t) => t,
            Err(e) => {
                self.atelier
                    .dire_longtemps(&format!("Ce plan ne peut pas être exporté : {e}."));
                return;
            }
        };
        let Some(dossier) = &self.dossier else {
            self.atelier
                .dire_longtemps("Le dossier des données est introuvable : rien n'a été exporté.");
            return;
        };
        let dossier = dossier.racine().join("exports");
        let fichier = dossier.join(format!("{}.v", crate::export::identifiant(&nom)));
        let ecrit = std::fs::create_dir_all(&dossier)
            .and_then(|()| std::fs::write(&fichier, texte))
            .and_then(|()| montrer_le_dossier(&dossier));
        match ecrit {
            Ok(()) => self.atelier.dire_longtemps(&format!(
                "Exporté en Verilog : {}, dans le dossier « exports » de tes données.",
                fichier
                    .file_name()
                    .map_or_else(String::new, |n| n.to_string_lossy().into_owned())
            )),
            Err(e) => self
                .atelier
                .dire_longtemps(&format!("L'export en Verilog a échoué : {e}")),
        }
    }

    /// Commence une photo du plan : les panneaux s'effacent, le circuit est cadré sur toute
    /// la fenêtre, sans sélection.
    fn commencer_la_photo(&mut self, ctx: &egui::Context) {
        if self.dossier.is_none() {
            self.atelier
                .dire_longtemps("Le dossier des données est introuvable : rien n'a été exporté.");
            return;
        }
        let selection = self.atelier.selection.take();
        self.photo = Some(Photo::nouvelle(self.plan.cam, selection));
        self.plan.cadrer(true);
        ctx.request_repaint();
    }

    /// Une image de plus pendant la photo : la capture est demandée, puis découpée et écrite
    /// dans le dossier `exports` quand elle arrive.
    fn avancer_la_photo(&mut self, ctx: &egui::Context, ecran: egui::Rect) {
        let zone = self.plan.zone_du_circuit(&self.atelier);
        let Some(photo) = &mut self.photo else {
            return;
        };
        ctx.request_repaint();
        let texte = match photo.avancer(zone) {
            Suite::Attendre => return,
            Suite::Capturer(rappel) => {
                ctx.send_viewport_cmd(egui::ViewportCommand::Screenshot(rappel));
                return;
            }
            Suite::Renoncer => {
                "La capture du plan n'est pas arrivée : rien n'a été exporté.".to_owned()
            }
            Suite::Terminer(image, zone) => self.ecrire_la_photo(&image, ecran, zone),
        };
        if let Some(photo) = self.photo.take() {
            self.plan.cam = photo.camera;
            self.atelier.selection = photo.selection;
        }
        self.atelier.dire_longtemps(&texte);
    }

    /// Écrit la photo découpée en PNG dans `exports`, montre le dossier, et dit ce qui s'est
    /// passé.
    fn ecrire_la_photo(
        &self,
        image: &egui::ColorImage,
        ecran: egui::Rect,
        zone: egui::Rect,
    ) -> String {
        let Some(png) = crate::photo::decouper(image, ecran, zone) else {
            return "Le circuit est hors de l'écran : rien n'a été exporté.".to_owned();
        };
        let Some(dossier) = &self.dossier else {
            return "Le dossier des données est introuvable : rien n'a été exporté.".to_owned();
        };
        let at = &self.atelier;
        let nom = at
            .lieu()
            .map_or_else(|| at.projet.nom.clone(), str::to_owned);
        let dossier = dossier.racine().join("exports");
        let fichier = dossier.join(format!("{}.png", crate::export::identifiant(&nom)));
        let ecrit = std::fs::create_dir_all(&dossier)
            .map_err(|e| e.to_string())
            .and_then(|()| png.save(&fichier).map_err(|e| e.to_string()));
        match ecrit {
            Ok(()) => {
                // L'image est écrite : ne pas pouvoir montrer le dossier n'est pas un échec.
                let _ = montrer_le_dossier(&dossier);
                format!(
                    "Plan exporté en image : {}, dans le dossier « exports » de tes données.",
                    fichier
                        .file_name()
                        .map_or_else(String::new, |n| n.to_string_lossy().into_owned())
                )
            }
            Err(e) => format!("L'export en image a échoué : {e}"),
        }
    }

    /// Ce qui déclenche un fondu en changeant.
    fn decor(&self) -> Decor {
        (self.ecran, self.apparence.mode, self.ouvertures)
    }

    /// Les transitions d'écran : l'apparition du lancement, puis, à chaque changement d'écran,
    /// de projet ou de thème, l'image d'avant qui s'efface par-dessus la nouvelle. Rien ici ne
    /// touche à l'état ni aux zones cliquables.
    fn fondre(&mut self, ui: &egui::Ui, debut: usize, avant: Decor) {
        let p = ui.painter();
        let m = &self.apparence.mouvement;
        let maintenant = self.atelier.horloge;
        let centre = ui.ctx().content_rect().center();
        let mut vivant = false;

        // Au lancement, l'écran monte du fond.
        let ne = *self.lancement.get_or_insert(maintenant);
        let age = maintenant - ne;
        if !m.sobre && age < f64::from(m.ecran) {
            let t = m.avancement(age, m.ecran, m.courbe_ecran);
            let echelle = 1.0 + m.zoom_ecran * (1.0 - t);
            calques::voiler(p, debut, t, calques::zoom_autour(centre, echelle));
            vivant = true;
        }

        // Le fondu en cours : ce qui arrive se pose, l'image d'avant s'efface par-dessus.
        if let Some(passage) = &self.passage {
            let age = maintenant - passage.fantome.ne;
            if m.sobre || age >= f64::from(passage.duree) {
                self.passage = None;
            } else {
                let t = m.avancement(age, passage.duree, passage.courbe);
                let entree = passage.entree + (1.0 - passage.entree) * t;
                calques::voiler(p, debut, 1.0, calques::zoom_autour(centre, entree));
                let sortie = 1.0 + (passage.sortie - 1.0) * t;
                passage
                    .fantome
                    .peindre(p, 1.0 - t, sortie, egui::Vec2::ZERO);
                vivant = true;
            }
        }

        // Quelque chose vient de changer : l'image qu'on vient de peindre est celle d'avant.
        let apres = self.decor();
        if apres != avant && !m.sobre {
            let fin = calques::repere(p);
            let (duree, courbe, sortie, entree) = if apres.0 != avant.0 {
                // On s'enfonce dans l'atelier, ou on en ressort.
                let z = match apres.0 {
                    Ecran::Atelier => m.zoom_ecran,
                    Ecran::Accueil => -m.zoom_ecran,
                };
                (m.ecran, m.courbe_ecran, 1.0 + z, 1.0 - z)
            } else if apres.2 != avant.2 {
                (m.ecran, m.courbe_ecran, 1.0, 1.0)
            } else {
                (m.theme, Courbe::Douce, 1.0, 1.0)
            };
            self.passage = Fantome::figer(p, debut, fin, maintenant, centre)
                .filter(|_| duree > 0.0)
                .map(|fantome| Passage {
                    fantome,
                    duree,
                    courbe,
                    sortie,
                    entree,
                });
            vivant |= self.passage.is_some();
        }
        if vivant {
            ui.ctx().request_repaint();
        }
    }

    /// Une image de l'atelier : le plan et ses panneaux.
    fn image_atelier(&mut self, ui: &mut egui::Ui, dt: f32) {
        let ctx = ui.ctx().clone();
        if self.disposition.is_none() {
            // Petite fenêtre au lancement : on laisse la place au plan, les panneaux restent
            // à un clic dans la barre du haut.
            let taille = ctx.content_rect().size();
            self.atelier.volets.infos &= taille.x >= 1280.0;
            self.atelier.volets.temps &= taille.y >= 780.0;
        }
        self.atelier.avancer(&self.apparence, dt);
        self.anims.en_mouvement = false;

        // Ouverture animée des panneaux, puis mise en page.
        let m = &self.apparence.mouvement;
        let (duree, courbe) = (m.duree(m.panneau), m.courbe_panneau);
        let v = self.atelier.volets;
        let mut ouverture = |nom: &str, ouvert: bool| {
            self.anims
                .transition(
                    egui::Id::unique(("ouverture", nom)),
                    if ouvert { 1.0 } else { 0.0 },
                    dt,
                    duree,
                    courbe,
                )
                .clamp(0.0, 1.0)
        };
        let o = Ouvertures {
            briques: ouverture("briques", v.briques),
            quete: ouverture("quete", v.quete),
            infos: ouverture("infos", v.infos),
            temps: ouverture("temps", v.temps),
        };
        let d = disposer(
            ctx.content_rect(),
            &self.apparence.formes,
            o,
            self.panneaux.contenus,
        );
        self.disposition = Some(d);

        if self.photo.is_some() {
            // Le plan seul, sur toute la fenêtre, le temps de la photo.
            let seul = Disposition {
                libre: d.ecran,
                ..d
            };
            self.plan.montrer(
                ui,
                &self.apparence,
                &mut self.atelier,
                &mut self.anims,
                &seul,
                dt,
            );
            self.avancer_la_photo(&ctx, d.ecran);
            return;
        }
        self.plan.montrer(
            ui,
            &self.apparence,
            &mut self.atelier,
            &mut self.anims,
            &d,
            dt,
        );
        let demandes = self.panneaux.montrer(
            ui,
            &self.apparence,
            &mut self.atelier,
            &mut self.plan,
            &mut self.anims,
            &d,
            o,
            dt,
        );
        if demandes.changer_mode {
            self.changer_mode();
        }
        if let Some(quoi) = demandes.ouvrir {
            self.ouvrir(quoi);
        }
        if demandes.accueil {
            self.ouvrir_accueil();
        }
        if let Some(action) = demandes.action {
            self.agir(action);
        }
        if std::mem::take(&mut self.panneaux.photo_demandee) {
            self.commencer_la_photo(&ctx);
        }
        self.suivre_la_mise_a_jour();
        self.clavier(ui);
        // On vient d'entrer dans une brique ou d'en sortir : le plan garde l'image qu'il quitte.
        self.plan
            .quitter(ui.painter(), &self.apparence, &self.atelier);
        self.enregistrer_si_besoin();

        // On ne redessine en continu que si quelque chose bouge.
        let at = &self.atelier;
        let vivant = self.anims.en_mouvement
            || self.plan.en_mouvement
            || (at.lecture && at.vue_t.is_none() && !at.session.stable())
            || (at.invite && !self.apparence.mouvement.sobre)
            || at
                .cas_neuf
                .as_ref()
                .is_some_and(|(_, t0)| at.horloge - t0 < 1.0);
        if vivant {
            ctx.request_repaint();
        } else if at
            .messages
            .iter()
            // Les messages qui s'effacent, et ceux qui viennent d'arriver (le temps de les
            // dessiner et de leur apparition) : sans cela, un message posé par la mise à jour
            // pendant que rien ne bouge n'apparaîtrait qu'au prochain mouvement de souris.
            .any(|m| m.duree.is_some() || at.horloge - m.ne < 0.5)
            || self.modifie_depuis.is_some()
        {
            ctx.request_repaint_after(std::time::Duration::from_millis(100));
        } else if self.surveillance.chemin().is_some()
            || self.mise_a_jour.as_ref().is_some_and(MiseAJour::occupee)
        {
            ctx.request_repaint_after(std::time::Duration::from_millis(500));
        }
    }

    /// Passe du sombre au clair, ou l'inverse, et le retient pour les prochains lancements.
    pub fn changer_mode(&mut self) {
        self.reglages.theme = Some(match self.apparence.mode {
            Mode::Sombre => Mode::Clair,
            Mode::Clair => Mode::Sombre,
        });
        self.appliquer_reglages();
        self.enregistrer_reglages();
    }

    fn clavier(&mut self, ui: &egui::Ui) {
        if self.ecran != Ecran::Atelier {
            return;
        }
        // Pendant qu'on écrit dans un champ, les touches vont au champ.
        if self.panneaux.en_saisie() || ui.ctx().egui_wants_keyboard_input() {
            return;
        }
        let a = &self.apparence;
        // Un clavier sélectionné sur le plan reçoit ce qui est tapé.
        let vers_le_circuit = self.atelier.clavier_selectionne().is_some();
        if vers_le_circuit {
            let (texte, entree, retour) = ui.input(|i| {
                let texte: String = i
                    .events
                    .iter()
                    .filter_map(|e| match e {
                        egui::Event::Text(t) => Some(t.as_str()),
                        _ => None,
                    })
                    .collect();
                (
                    texte,
                    i.key_pressed(Key::Enter),
                    i.key_pressed(Key::Backspace),
                )
            });
            for c in texte.chars() {
                if let Ok(code) = u8::try_from(u32::from(c)) {
                    self.atelier.taper(code);
                }
            }
            if entree {
                self.atelier.taper(b'\n');
            }
            if retour {
                self.atelier.taper(8);
            }
        }
        let k = ui.input(|i| {
            let cmd = i.modifiers.command;
            let fleche = [
                (Key::ArrowLeft, -1.0, 0.0),
                (Key::ArrowRight, 1.0, 0.0),
                (Key::ArrowUp, 0.0, -1.0),
                (Key::ArrowDown, 0.0, 1.0),
            ]
            .into_iter()
            .find(|(k, ..)| i.key_pressed(*k))
            .map(|(_, x, y)| (x, y));
            Touches {
                espace: !vers_le_circuit && i.key_pressed(Key::Space),
                suppr: i.key_pressed(Key::Delete)
                    || (!vers_le_circuit && i.key_pressed(Key::Backspace)),
                annuler: cmd && !i.modifiers.shift && i.key_pressed(Key::Z),
                retablir: cmd
                    && (i.key_pressed(Key::Y) || (i.modifiers.shift && i.key_pressed(Key::Z))),
                tout: cmd && i.key_pressed(Key::A),
                regrouper: cmd && i.key_pressed(Key::G),
                dupliquer: cmd && i.key_pressed(Key::D),
                retourner: !cmd && !vers_le_circuit && i.key_pressed(Key::R),
                cadrer: !cmd && !vers_le_circuit && i.key_pressed(Key::F),
                echap: i.key_pressed(Key::Escape),
                fleche,
                // Le système change Ctrl+C, Ctrl+X et Ctrl+V en événements à part ; Ctrl+V
                // n'arrive que si le presse-papiers du système contient du texte.
                copier: (cmd && i.key_pressed(Key::C))
                    || i.events.iter().any(|e| matches!(e, egui::Event::Copy)),
                couper: (cmd && i.key_pressed(Key::X))
                    || i.events.iter().any(|e| matches!(e, egui::Event::Cut)),
                coller: (cmd && i.key_pressed(Key::V))
                    .then(String::new)
                    .or_else(|| {
                        i.events.iter().find_map(|e| match e {
                            egui::Event::Paste(t) => Some(t.clone()),
                            egui::Event::PasteImage(_) => Some(String::new()),
                            _ => None,
                        })
                    }),
            }
        });
        if k.copier || k.couper {
            let fait = if k.couper {
                self.atelier.couper_selection(a)
            } else {
                self.atelier.copier_selection()
            };
            if let Some(pp) = self
                .atelier
                .presse_papiers
                .as_ref()
                .filter(|_| fait || k.couper)
            {
                ui.ctx().copy_text(pp.en_texte());
            }
        }
        if let Some(texte) = k.coller {
            // Un circuit copié par l'application, peut-être avant de la relancer, ou reçu
            // sous forme de texte ; sinon, ce qui a été copié ici.
            if let Some(pp) = crate::etat::PressePapiers::de_texte(&texte) {
                let meme = self
                    .atelier
                    .presse_papiers
                    .as_ref()
                    .is_some_and(|p| p.elements == pp.elements && p.modeles == pp.modeles);
                if !meme {
                    self.atelier.presse_papiers = Some(pp);
                }
            }
            self.atelier.coller(a);
        }
        if k.espace {
            self.atelier.lecture_ou_pause();
        }
        if k.suppr {
            self.atelier.supprimer_selection(a);
        }
        if k.annuler {
            self.atelier.annuler();
        }
        if k.retablir {
            self.atelier.retablir();
        }
        if k.tout {
            let tous = self.atelier.doc.elements.iter().map(|e| e.id).collect();
            self.atelier.selectionner_tous(tous);
        }
        if k.regrouper {
            self.atelier.regrouper_selection(a);
        }
        if k.dupliquer {
            self.atelier.dupliquer_selection(a);
        }
        if k.retourner && self.atelier.selection.is_some() {
            self.atelier.retourner_selection(a);
        }
        if k.cadrer {
            self.plan.cadrer(false);
        }
        if k.echap {
            // Échap lâche la sélection ; sans sélection, referme la brique ouverte.
            if self.atelier.selection.is_some() {
                self.atelier.selection = None;
            } else {
                self.atelier.sortir();
            }
        }
        if let Some((dx, dy)) = k.fleche {
            let ids = self.atelier.elements_selectionnes().to_vec();
            let pas = a.manipulation.pas_clavier;
            for id in &ids {
                if let Some(e) = self.atelier.doc.element_mut(*id) {
                    e.pos = Point::nouveau(e.pos.x + dx * pas, e.pos.y + dy * pas);
                }
            }
            if !ids.is_empty() {
                self.atelier.apres_modification(a);
            }
        }
    }
}

/// Les raccourcis clavier relevés pendant une image.
struct Touches {
    espace: bool,
    suppr: bool,
    annuler: bool,
    retablir: bool,
    tout: bool,
    regrouper: bool,
    dupliquer: bool,
    retourner: bool,
    cadrer: bool,
    echap: bool,
    fleche: Option<(f32, f32)>,
    copier: bool,
    couper: bool,
    /// Ctrl+V, avec le texte du presse-papiers du système (vide s'il n'y en a pas).
    coller: Option<String>,
}

impl eframe::App for Application {
    fn ui(&mut self, ui: &mut egui::Ui, _frame: &mut eframe::Frame) {
        self.image(ui);
    }

    fn on_exit(&mut self) {
        self.enregistrer();
    }

    fn clear_color(&self, _visuals: &egui::Visuals) -> [f32; 4] {
        self.apparence.palette().fond.c().to_normalized_gamma_f32()
    }
}

/// Montre un dossier dans l'explorateur de fichiers du système.
fn montrer_le_dossier(chemin: &Path) -> std::io::Result<()> {
    let programme = if cfg!(windows) {
        "explorer"
    } else if cfg!(target_os = "macos") {
        "open"
    } else {
        "xdg-open"
    };
    std::fs::create_dir_all(chemin)?;
    let mut commande = std::process::Command::new(programme);
    commande.arg(chemin);
    #[cfg(windows)]
    {
        // Jamais de fenêtre de console ouverte par l'application (CREATE_NO_WINDOW).
        use std::os::windows::process::CommandExt;
        commande.creation_flags(0x0800_0000);
    }
    let mut enfant = commande.spawn()?;
    // On attend l'explorateur à côté, pour ne pas laisser de processus zombie.
    std::thread::spawn(move || {
        let _ = enfant.wait();
    });
    Ok(())
}

/// Le message qui propose de redémarrer sur la version installée.
fn texte_installee(version: &str) -> String {
    format!(
        "La version {version} est installée et vérifiée. Elle démarre au prochain lancement, ou tout de suite."
    )
}

/// La première ligne utile des nouveautés d'une version, assez courte pour un message.
fn resume_des_notes(notes: &str) -> String {
    let ligne = notes
        .lines()
        .map(|l| l.trim().trim_start_matches(['#', '-', '*', ' ']).trim())
        .find(|l| !l.is_empty())
        .unwrap_or_default();
    if ligne.chars().count() > 140 {
        let coupe: String = ligne.chars().take(139).collect();
        format!("{coupe}…")
    } else {
        ligne.to_owned()
    }
}
