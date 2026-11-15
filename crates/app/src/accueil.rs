//! L'écran d'accueil : ce qu'on voit en ouvrant l'application, avant le plan.
//!
//! Un menu à gauche (Accueil, Parcours, Mes projets, Paramètres, Quitter) et, à droite, la
//! rubrique choisie :
//! - **Accueil** : reprendre le dernier projet (Entrée), la prochaine quête, l'avancement par
//!   chapitre, les projets récents ;
//! - **Parcours** : les chapitres, leurs quêtes, et la fiche de la quête choisie ;
//! - **Mes projets** : ouvrir, renommer, dupliquer, mettre à la corbeille, les exemples ;
//! - **Paramètres** : thème, taille du texte, animations, vitesse, lancement, raccourcis du
//!   bureau et du menu Démarrer (Windows), mises à jour, dossier des données, raccourcis clavier.
//!
//! L'accueil ne touche ni au disque ni à l'atelier : il renvoie des [`Demande`] que
//! l'application exécute. Tout est peint à la main, avec les outils des panneaux ; les couleurs
//! viennent de l'apparence et la mise en page de [`disposer_accueil`].

use std::collections::HashMap;
use std::time::SystemTime;

use egui::{emath::TSTransform, Align2, Key, Pos2, Rect, Sense, Shape, Stroke, StrokeKind, Vec2};

use crate::apparence::{Couleur, Courbe, Mode};
use crate::calques::{self, Fantome};
use crate::dessin::{halo, Animations, Icone, Police, Toile};
use crate::disposition::{disposer_accueil, DispositionAccueil};
use crate::document::{Document, Genre};
use crate::etat::{libelle_pas_par_seconde, Atelier, Proposition};
use crate::exemples::EXEMPLES;
use crate::panneaux::{Cadre, Ouverture, Panneaux};
use crate::quete::{Epreuve, Fiche, PARCOURS};
use crate::raccourci::Lieu;
use crate::reglages::{Lancement, Reglages, TAILLES_TEXTE};
use crate::sauvegarde::FicheProjet;
use crate::savoir::{self, Branche};

/// Les rubriques de l'accueil.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum Rubrique {
    /// Reprendre, prochaine quête, avancement, projets récents.
    #[default]
    Accueil,
    /// Les chapitres et les quêtes.
    Parcours,
    /// Les projets enregistrés.
    Projets,
    /// Les réglages.
    Parametres,
}

/// Ce que l'accueil demande à l'application.
#[derive(Clone, Debug, PartialEq)]
pub enum Demande {
    /// Revenir au projet chargé, sur le plan.
    Continuer,
    /// Ouvrir un projet, une quête, un exemple, ou un projet vide.
    Ouvrir(Ouverture),
    /// Renommer un projet enregistré.
    Renommer {
        /// Son nom de fichier.
        fichier: String,
        /// Le nouveau nom.
        nom: String,
    },
    /// Dupliquer un projet enregistré.
    Dupliquer(String),
    /// Mettre un projet enregistré à la corbeille.
    Jeter(String),
    /// Appliquer et enregistrer ces réglages.
    Reglages(Reglages),
    /// Chercher une nouvelle version tout de suite.
    ChercherMiseAJour,
    /// Poser (`poser`) ou retirer le raccourci de ce lieu.
    Raccourci {
        /// Le bureau ou le menu Démarrer.
        lieu: Lieu,
        /// Vrai pour le poser, faux pour le retirer.
        poser: bool,
    },
    /// Montrer le dossier des données dans l'explorateur de fichiers.
    OuvrirDossier,
    /// Effacer l'avancement dans le parcours (une copie est gardée dans le dossier).
    RecommencerParcours,
    /// Fermer l'application.
    Quitter,
    /// Une action choisie dans un message.
    Action(Proposition),
}

/// Ce que l'application fournit à l'accueil pour une image.
pub struct Contexte<'r> {
    /// Les projets enregistrés, le plus récent d'abord.
    pub projets: &'r [FicheProjet],
    /// Le nom de fichier du projet chargé dans l'atelier, s'il y en a un à reprendre.
    pub courant: Option<&'r str>,
    /// Les réglages en vigueur.
    pub reglages: &'r Reglages,
    /// Le dossier des données, écrit pour l'utilisateur (`None` : rien n'est enregistré).
    pub dossier: Option<&'r str>,
    /// L'état des mises à jour, en une phrase.
    pub mise_a_jour: &'r str,
    /// Les raccourcis présents (bureau, menu Démarrer) ; `Some(None)` : l'application tourne
    /// depuis l'archive ; `None` : pas de raccourcis ici (ailleurs qu'à Windows).
    pub raccourcis: Option<Option<[bool; 2]>>,
    /// L'heure, pour écrire « il y a 5 min ».
    pub maintenant: SystemTime,
}

/// Un nom de projet en cours de modification.
#[derive(Clone, Debug)]
struct Renommage {
    fichier: String,
    texte: String,
    /// Vrai à la première image : le champ prend le clavier.
    neuf: bool,
}

/// L'état propre à l'écran d'accueil.
#[derive(Clone, Debug, Default)]
pub struct Accueil {
    /// La rubrique affichée.
    pub rubrique: Rubrique,
    /// Le chapitre choisi dans le parcours (rang parmi les chapitres).
    pub chapitre: usize,
    /// La quête choisie dans le parcours (rang dans [`PARCOURS`]).
    pub quete: Option<usize>,
    /// Défilement du contenu, en points.
    pub defilement: f32,
    renommage: Option<Renommage>,
    /// Le projet dont on demande s'il faut le mettre à la corbeille.
    confirmation: Option<String>,
    /// Vrai après un premier clic sur « Recommencer le parcours » : le second confirme.
    recommencer_arme: bool,
    /// Vrai si une saisie s'est terminée à cette image : sa touche (Entrée, Échap) lui revient.
    saisie_finie: bool,
    reperes: Vec<(&'static str, usize, Rect)>,
    /// La rubrique d'avant, qui s'efface pendant que la nouvelle arrive.
    glisse: Option<Glisse>,
    /// L'instant où la page affichée est arrivée : ses barres d'avancement se remplissent.
    arrivee: Option<f64>,
    /// De 0 à 1, le remplissage des barres d'avancement depuis cette arrivée.
    plein: f32,
    /// Ce qui arrive en fondu dans une page (les quêtes d'un chapitre, la fiche d'une quête) :
    /// la valeur affichée et l'instant où elle a changé.
    arrivees: HashMap<&'static str, (usize, f64)>,
}

/// Le passage d'une rubrique à une autre.
#[derive(Clone, Debug)]
struct Glisse {
    /// Le contenu de la rubrique d'avant.
    fantome: Option<Fantome>,
    /// L'instant du changement.
    ne: f64,
    /// +1 si la nouvelle rubrique est plus bas dans le menu (elle monte), −1 sinon.
    sens: f32,
}

/// Les trois sortes de boutons.
#[derive(Clone, Copy, PartialEq, Eq)]
enum Sorte {
    /// L'action principale : fond d'accent.
    Principal,
    /// Une action secondaire : fond discret et bordure.
    Secondaire,
    /// Une action qui retire quelque chose.
    Danger,
}

/// Les chapitres du parcours, dans l'ordre, chacun avec les rangs de ses quêtes.
pub fn chapitres() -> Vec<(&'static str, Vec<usize>)> {
    let mut v: Vec<(&'static str, Vec<usize>)> = Vec::new();
    for (i, f) in PARCOURS.iter().enumerate() {
        match v.last_mut() {
            Some((nom, quetes)) if *nom == f.chapitre => quetes.push(i),
            _ => v.push((f.chapitre, vec![i])),
        }
    }
    v
}

/// « à l'instant », « il y a 5 min », « il y a 3 h », « hier », « il y a 4 jours »…
pub fn il_y_a(maintenant: SystemTime, quand: SystemTime) -> String {
    let s = maintenant.duration_since(quand).map_or(0, |d| d.as_secs());
    let (min, h, j) = (s / 60, s / 3600, s / 86_400);
    match () {
        _ if s < 60 => "à l'instant".to_owned(),
        _ if min < 60 => format!("il y a {min} min"),
        _ if h < 24 => format!("il y a {h} h"),
        _ if j < 2 => "hier".to_owned(),
        _ if j < 14 => format!("il y a {j} jours"),
        _ if j < 60 => format!("il y a {} semaines", j / 7),
        _ if j < 730 => format!("il y a {} mois", j / 30),
        _ => format!("il y a {} ans", j / 365),
    }
}

/// « 1 brique », « 3 briques ».
fn pluriel(n: usize, mot: &str) -> String {
    if n > 1 {
        format!("{n} {mot}s")
    } else {
        format!("{n} {mot}")
    }
}

/// Ce que vérifie une épreuve, en une phrase.
fn epreuve_lisible(e: Epreuve) -> &'static str {
    match e {
        Epreuve::Combinatoire => {
            "Vérifiée sur toute sa table de vérité, contre un corrigé en NAND."
        }
        Epreuve::Sequentielle => {
            "Circuit à mémoire : une longue suite d'actions est rejouée et comparée au corrigé."
        }
        Epreuve::Ordinateur { .. } => {
            "Un ordinateur complet : on tape au clavier, l'écran doit afficher la bonne réponse."
        }
        Epreuve::Systeme { .. } => {
            "Un grand projet : on tape au clavier, l'écran doit afficher la bonne réponse, après tout le voyage."
        }
    }
}

/// Une couleur de la palette avec une autre opacité.
fn voilee(c: Couleur, opacite: f32) -> Couleur {
    let [r, v, b, a] = c.0;
    Couleur([
        r,
        v,
        b,
        (f32::from(a) * opacite.clamp(0.0, 1.0)).round() as u8,
    ])
}

impl Accueil {
    /// Va à une rubrique, en haut de la page.
    pub fn aller(&mut self, rubrique: Rubrique) {
        self.rubrique = rubrique;
        self.defilement = 0.0;
        self.renommage = None;
        self.confirmation = None;
        self.arrivee = None;
    }

    /// Fait arriver en fondu et en glissant ce qui a été peint depuis `debut`, chaque fois que
    /// `valeur` change (une autre quête choisie, un autre chapitre). Visuel seulement.
    fn arriver(
        &mut self,
        k: &mut Cadre,
        horloge: f64,
        cle: &'static str,
        valeur: usize,
        debut: usize,
        glisse: Vec2,
    ) {
        let m = &k.a.mouvement;
        let e = self
            .arrivees
            .entry(cle)
            .or_insert((valeur, f64::NEG_INFINITY));
        if e.0 != valeur {
            *e = (valeur, horloge);
        }
        let t = m.avancement(horloge - e.1, m.rubrique, Courbe::Sortie);
        if t < 1.0 {
            calques::voiler(
                &k.t.p,
                debut,
                t,
                TSTransform::from_translation(glisse * (1.0 - t)),
            );
            k.anims.en_mouvement = true;
        }
    }

    /// Ouvre le parcours sur le chapitre de cette quête, la quête choisie.
    pub fn montrer_quete(&mut self, rang: usize) {
        self.aller(Rubrique::Parcours);
        self.quete = Some(rang);
        if let Some(f) = PARCOURS.get(rang) {
            self.chapitre = chapitres()
                .iter()
                .position(|(nom, _)| *nom == f.chapitre)
                .unwrap_or(0);
        }
    }

    /// Où se trouve, à l'écran, une commande dessinée à la dernière image : `"menu"`,
    /// `"continuer"`, `"commencer-le-parcours"`, `"prochaine-quete"`, `"projet-recent"`,
    /// `"chapitre"`, `"quete"`, `"commencer"`, `"reprendre"`, `"projet"`, `"renommer"`,
    /// `"dupliquer"`, `"supprimer"`, `"confirmer"`, `"garder"`, `"exemple"`, `"nouveau"`,
    /// `"champ-renommer"`, `"reglage-…"`, `"chercher"`, `"ouvrir-dossier"`, `"quitter"`.
    pub fn repere(&self, nom: &str, rang: usize) -> Option<Rect> {
        self.reperes
            .iter()
            .find(|(n, r, _)| *n == nom && *r == rang)
            .map(|(.., rect)| *rect)
    }

    /// Vrai si un nom de projet est en cours de saisie.
    pub fn en_saisie(&self) -> bool {
        self.renommage.is_some()
    }

    /// Dessine l'accueil et renvoie ce qu'il demande.
    pub fn montrer(
        &mut self,
        ui: &mut egui::Ui,
        a: &crate::apparence::Apparence,
        at: &mut Atelier,
        anims: &mut Animations,
        cx: &Contexte,
        dt: f32,
    ) -> Vec<Demande> {
        let d = disposer_accueil(ui.ctx().content_rect(), &a.formes);
        let mut demandes = Vec::new();
        self.saisie_finie = false;
        let toile = Toile::nouvelle(ui.painter().with_clip_rect(d.ecran), a);
        let mut k = Cadre {
            ui,
            t: toile,
            anims,
            a,
            dt,
            obstacles: Vec::new(),
            decoupe: None,
            reperes: Vec::new(),
        };
        Self::fond(&k, &d);
        let m = &a.mouvement;
        let arrivee = *self.arrivee.get_or_insert(at.horloge);
        self.plein = m.avancement(at.horloge - arrivee, m.remplissage, Courbe::Sortie);
        if self.plein < 1.0 {
            k.anims.en_mouvement = true;
        }
        // La rubrique dessinée à cette image : un clic dans le menu ne la change qu'après, pour
        // que l'ancienne puisse s'effacer pendant que la nouvelle arrive.
        let dessinee = self.rubrique;
        let voulue = self.menu(&mut k, &d, at, cx, &mut demandes);

        // Le contenu de la rubrique défile dans sa zone.
        let zone = Rect::from_min_max(
            Pos2::new(d.contenu.left() - 12.0, d.ecran.top()),
            Pos2::new(d.contenu.right() + 12.0, d.ecran.bottom()),
        );
        let t = k.t.sur(1.0, zone);
        let fond = std::mem::replace(&mut k.t, t);
        k.decoupe = Some(zone);
        let y = d.contenu.top() - self.defilement;
        let debut = calques::repere(&k.t.p);
        let bas = match self.rubrique {
            Rubrique::Accueil => self.page_accueil(&mut k, &d, at, cx, y, &mut demandes),
            Rubrique::Parcours => self.page_parcours(&mut k, &d, at, cx, y, &mut demandes),
            Rubrique::Projets => self.page_projets(&mut k, &d, cx, y, &mut demandes),
            Rubrique::Parametres => self.page_parametres(&mut k, &d, cx, y, &mut demandes),
        };
        let hauteur = bas + self.defilement - zone.top() + 24.0;
        let barre = Rect::from_min_max(zone.min, Pos2::new(d.ecran.right(), zone.bottom()));
        self.defilement = k.defiler(barre, zone, hauteur, self.defilement, 1.0);
        let fin = calques::repere(&k.t.p);
        if let Some(g) = &self.glisse {
            let t = m.avancement(at.horloge - g.ne, m.rubrique, Courbe::Sortie);
            if t >= 1.0 {
                self.glisse = None;
            } else {
                let pas = Vec2::new(0.0, g.sens * m.glissement);
                calques::voiler(
                    &k.t.p,
                    debut,
                    t,
                    TSTransform::from_translation(pas * (1.0 - t)),
                );
                if let Some(f) = &g.fantome {
                    f.peindre(&k.t.p, (1.0 - t) * (1.0 - t), 1.0, -pas * t);
                }
                k.anims.en_mouvement = true;
            }
        }
        k.decoupe = None;
        k.t = fond;

        // Les messages (« … est dans la corbeille. Annuler ») montent du bas du contenu, pour ne
        // pas couvrir les boutons d'en-tête des rubriques.
        let mut choisie = None;
        Panneaux::messages(&mut k, at, d.contenu, 0.0, true, &mut choisie);
        if let Some(action) = choisie {
            demandes.push(Demande::Action(action));
        }
        self.clavier(&mut k, at, cx, &mut demandes);
        if let Some(rubrique) = voulue.filter(|r| *r != self.rubrique) {
            self.aller(rubrique);
        }
        if self.rubrique != dessinee {
            // Changement de rubrique : ce qu'on vient de peindre s'effacera sur la suivante.
            self.arrivee = Some(at.horloge);
            self.arrivees.clear();
            self.glisse = (!m.sobre).then(|| Glisse {
                fantome: Fantome::figer(&k.t.p, debut, fin, at.horloge, zone.center()),
                ne: at.horloge,
                sens: if self.rubrique as usize >= dessinee as usize {
                    1.0
                } else {
                    -1.0
                },
            });
            k.anims.en_mouvement = true;
        }
        self.reperes = std::mem::take(&mut k.reperes);
        demandes
    }

    /// Le fond : la couleur du plan, un halo et une trame de points.
    fn fond(k: &Cadre, d: &DispositionAccueil) {
        let c = k.t.c;
        k.t.p.rect_filled(d.ecran, 0.0, c.fond.c());
        let centre = d.contenu.center_top() + Vec2::new(0.0, 160.0);
        k.t.p.add(halo(
            Rect::from_center_size(centre, Vec2::new(1500.0, 1000.0)),
            voilee(c.fond_halo, 0.9),
        ));

        let pas = k.a.formes.pas_grille.max(12.0) * 1.25;
        let mut y = d.ecran.top() + pas / 2.0;
        while y < d.ecran.bottom() {
            let mut x = d.ecran.left() + pas / 2.0;
            while x < d.ecran.right() {
                k.t.p
                    .circle_filled(Pos2::new(x, y), 1.0, c.grille.alpha(0.55));
                x += pas;
            }
            y += pas;
        }
    }

    // --- Outils de dessin -------------------------------------------------------------------

    fn largeur_bouton(k: &Cadre, texte: &str, icone: Option<Icone>) -> f32 {
        k.t.largeur(texte, Police::CorpsDemi, k.a.typo.corps)
            + 32.0
            + if icone.is_some() { 24.0 } else { 0.0 }
    }

    /// Un bouton à texte (et icône facultative). Renvoie `true` s'il est cliqué.
    #[allow(clippy::too_many_arguments)]
    fn bouton(
        k: &mut Cadre,
        rect: Rect,
        repere: &'static str,
        rang: usize,
        texte: &str,
        icone: Option<Icone>,
        genre: Sorte,
        bulle: &str,
    ) -> bool {
        k.reperer(repere, rang, rect);
        let touche = k.toucher(
            rect,
            egui::Id::unique((repere, rang)),
            bulle,
            Sense::click(),
        );
        let c = k.t.c;
        let rayon = k.a.formes.rayon_bouton.min(rect.height() * 0.4);
        let encre = match genre {
            Sorte::Principal => {
                // Action principale : surface pleine d'accent, sans halo ni bordure.
                k.t.p.rect_filled(
                    touche.rect,
                    rayon,
                    c.accent.vers(c.accent_clair, touche.chaud * 0.18).c(),
                );
                k.t.reflet(touche.rect, rayon);
                c.sur_accent.c()
            }
            Sorte::Secondaire => {
                k.t.element(touche.rect, rayon, touche.chaud, false);
                c.texte.c()
            }
            Sorte::Danger => {
                k.t.p.rect_filled(
                    touche.rect,
                    rayon,
                    c.danger.vers(c.texte, touche.chaud * 0.12).c(),
                );
                c.sur_accent.c()
            }
        };
        let l_texte = k.t.largeur(texte, Police::CorpsDemi, k.a.typo.corps);
        let mut x = touche.rect.center().x - l_texte / 2.0;
        if let Some(i) = icone {
            x += 12.0;
            k.t.icone(i, Pos2::new(x - 14.0, touche.rect.center().y), 16.0, encre);
        }
        k.t.texte(
            Pos2::new(x, touche.rect.center().y),
            Align2::LEFT_CENTER,
            texte,
            Police::CorpsDemi,
            k.a.typo.corps,
            encre,
        );
        touche.clic
    }

    /// Un choix parmi plusieurs, côte à côte ; la pastille glisse vers le choix. Renvoie le
    /// rang cliqué, s'il est différent du choix actuel.
    fn segments(
        k: &mut Cadre,
        rect: Rect,
        repere: &'static str,
        options: &[String],
        choisi: usize,
    ) -> Option<usize> {
        let c = k.t.c;
        let n = options.len().max(1);
        k.t.p.rect(
            rect,
            k.a.formes.rayon_bouton,
            c.carte_2.c(),
            Stroke::new(1.0, c.bord.c()),
            StrokeKind::Inside,
        );
        let l = (rect.width() - 6.0) / n as f32;
        let x_choisi = k.anims.vers(
            egui::Id::unique(("segments", repere)),
            choisi as f32,
            k.dt,
            k.a.mouvement.lissage_ui,
        );
        // La pastille du choix : une surface surélevée qui glisse dans la piste.
        let pastille = Rect::from_min_size(
            Pos2::new(rect.left() + 3.0 + x_choisi * l, rect.top() + 3.0),
            Vec2::new(l, rect.height() - 6.0),
        );
        let rayon = (k.a.formes.rayon_bouton - 2.0).max(3.0);
        k.t.p.rect(
            pastille,
            rayon,
            match k.a.mode {
                crate::apparence::Mode::Sombre => c.enfonce.c(),
                crate::apparence::Mode::Clair => c.carte.c(),
            },
            Stroke::new(1.0, c.bord_fort.c()),
            StrokeKind::Inside,
        );
        k.t.reflet(pastille, rayon);
        let mut clic = None;
        for (i, texte) in options.iter().enumerate() {
            let r = Rect::from_min_size(
                Pos2::new(rect.left() + 3.0 + i as f32 * l, rect.top() + 3.0),
                Vec2::new(l, rect.height() - 6.0),
            );
            k.reperer(repere, i, r);
            let touche = k.toucher(r, egui::Id::unique((repere, i)), "", Sense::click());
            let dessus = (1.0 - (x_choisi - i as f32).abs()).clamp(0.0, 1.0);
            let encre = c
                .doux
                .vers(c.texte, touche.chaud * 0.8)
                .vers(c.texte, dessus);
            k.t.texte(
                r.center(),
                Align2::CENTER_CENTER,
                &k.t.abreger(texte, Police::CorpsDemi, k.a.typo.corps, l - 12.0),
                Police::CorpsDemi,
                k.a.typo.corps,
                encre.c(),
            );
            if touche.clic && i != choisi {
                clic = Some(i);
            }
        }
        clic
    }

    /// Une barre d'avancement.
    fn barre(t: &Toile, rect: Rect, part: f32, couleur: Couleur) {
        let c = t.c;
        let rayon = rect.height() / 2.0;
        t.p.rect_filled(rect, rayon, c.enfonce.c());
        if part > 0.0 {
            let mut plein = rect;
            plein.set_width((rect.width() * part.clamp(0.0, 1.0)).max(rect.height()));
            t.p.rect_filled(plein, rayon, couleur.c());
        }
    }

    /// Une étiquette arrondie (« Réussie », « Conseillée »…). Renvoie sa largeur.
    /// Colonnes de la liste des chapitres dans la carte d'avancement : trois si chacune garde
    /// la place d'un nom de chapitre et de son compte, deux sinon.
    fn colonnes_avancement(k: &Cadre, largeur_carte: f32) -> usize {
        let utile = largeur_carte - 48.0;
        let par_colonne = 160.0 * k.a.formes.echelle_texte;
        if utile >= 3.0 * par_colonne + 36.0 {
            3
        } else {
            2
        }
    }

    fn etiquette(t: &Toile, droite: Pos2, texte: &str, couleur: Couleur) -> f32 {
        let l = t.largeur_legende(texte) + 14.0;
        let r = Rect::from_min_size(Pos2::new(droite.x - l, droite.y - 10.0), Vec2::new(l, 20.0));
        t.p.rect(
            r,
            t.a.formes.rayon_petit + 2.0,
            couleur.alpha(0.12),
            Stroke::new(1.0, couleur.alpha(0.45)),
            StrokeKind::Inside,
        );
        t.legende(r.center(), Align2::CENTER_CENTER, texte, couleur.c());
        l
    }

    /// Un lien texte aligné à droite sur `droite` (« Tous les projets »). Renvoie `true` s'il est
    /// cliqué.
    fn lien(k: &mut Cadre, droite: Pos2, texte: &str, repere: &'static str) -> bool {
        let c = k.t.c;
        let l = k.t.largeur(texte, Police::CorpsDemi, k.a.typo.corps) + 8.0;
        let rect =
            Rect::from_min_size(Pos2::new(droite.x - l, droite.y - 12.0), Vec2::new(l, 24.0));
        k.reperer(repere, 0, rect);
        let touche = k.toucher(rect, egui::Id::unique(("lien", repere)), "", Sense::click());
        k.t.texte(
            rect.right_center(),
            Align2::RIGHT_CENTER,
            texte,
            Police::CorpsDemi,
            k.a.typo.corps,
            c.accent_clair.vers(c.texte, touche.chaud).c(),
        );
        touche.clic
    }

    /// Une touche de clavier dessinée (« Entrée », « Échap ») : petit cadre en relief, texte
    /// en chasse fixe. `gauche` est le milieu de son bord gauche. Renvoie sa largeur.
    pub(crate) fn touche_clavier(t: &Toile, gauche: Pos2, texte: &str) -> f32 {
        let c = t.c;
        let l = t.largeur(texte, Police::Mono, t.a.typo.legende) + 12.0;
        let r = Rect::from_min_size(Pos2::new(gauche.x, gauche.y - 10.0), Vec2::new(l, 20.0));
        let rayon = t.a.formes.rayon_petit;
        t.p.rect(
            r.translate(Vec2::new(0.0, 1.0)),
            rayon,
            c.bord.c(),
            Stroke::NONE,
            StrokeKind::Inside,
        );
        t.p.rect(
            r,
            rayon,
            c.carte_2.c(),
            Stroke::new(1.0, c.bord_fort.c()),
            StrokeKind::Inside,
        );
        t.texte(
            r.center(),
            Align2::CENTER_CENTER,
            texte,
            Police::Mono,
            t.a.typo.legende,
            c.doux.c(),
        );
        l
    }

    /// Le titre d'une rubrique et sa phrase d'introduction. Renvoie le bas.
    fn titre(k: &Cadre, x: f32, y: f32, l: f32, titre: &str, sous: &str) -> f32 {
        let c = k.t.c;
        k.t.texte(
            Pos2::new(x, y + 18.0),
            Align2::LEFT_CENTER,
            titre,
            Police::Titre,
            k.a.typo.affiche,
            c.texte.c(),
        );
        let h = k.t.paragraphe(
            Pos2::new(x, y + 44.0),
            l,
            sous,
            Police::Corps,
            k.a.typo.fort,
            c.doux.c(),
        );
        y + 44.0 + h + 22.0
    }

    /// Le plan d'un projet en petit : les éléments, les fils, et le courant qui y passe.
    fn miniature(t: &Toile, rect: Rect, doc: &Document, horloge: f64, anime: bool) {
        let c = t.c;
        t.p.rect(
            rect,
            t.a.formes.rayon_bouton + 2.0,
            c.fond.c(),
            Stroke::new(1.0, c.bord.c()),
            StrokeKind::Inside,
        );
        let pas = 18.0;
        let mut y = rect.top() + pas;
        while y < rect.bottom() - 4.0 {
            let mut x = rect.left() + pas;
            while x < rect.right() - 4.0 {
                t.p.circle_filled(Pos2::new(x, y), 0.9, c.grille.c());
                x += pas;
            }
            y += pas;
        }
        if doc.elements.is_empty() {
            t.texte(
                rect.center(),
                Align2::CENTER_CENTER,
                "Plan vide",
                Police::Corps,
                t.a.typo.corps,
                c.doux.c(),
            );
            return;
        }
        let (mut min, mut max) = (Pos2::new(f32::MAX, f32::MAX), Pos2::new(f32::MIN, f32::MIN));
        for e in &doc.elements {
            let demi = Vec2::new(e.largeur(), e.hauteur()) / 2.0;
            let p = Pos2::new(e.pos.x, e.pos.y);
            min = min.min(p - demi);
            max = max.max(p + demi);
        }
        let boite = Rect::from_min_max(min, max);
        let place = rect.shrink(22.0);
        let echelle = (place.width() / boite.width().max(1.0))
            .min(place.height() / boite.height().max(1.0))
            .min(0.9);
        let vers = |x: f32, y: f32| place.center() + (Pos2::new(x, y) - boite.center()) * echelle;
        let t = t.sur(1.0, rect.shrink(2.0));
        let ecart = 22.0;
        let mut rang = 0.0_f64;
        for e in &doc.elements {
            let n = e.entrees.len() as f32;
            for (i, source) in e.entrees.iter().enumerate() {
                let Some(s) = source else { continue };
                let Some(se) = doc.element(s.id) else {
                    continue;
                };
                let ns = se.broches_s.len().max(1) as f32;
                let a = vers(
                    se.pos.x + se.largeur() / 2.0,
                    se.pos.y + (s.sortie as f32 - (ns - 1.0) / 2.0) * ecart,
                );
                let b = vers(
                    e.pos.x - e.largeur() / 2.0,
                    e.pos.y + (i as f32 - (n - 1.0) / 2.0) * ecart,
                );
                let mx = (a.x + b.x) / 2.0;
                let points = [a, Pos2::new(mx, a.y), Pos2::new(mx, b.y), b];
                t.p.line(points.to_vec(), Stroke::new(4.0, c.un.alpha(0.12)));
                t.p.line(points.to_vec(), Stroke::new(1.6, c.un.alpha(0.75)));
                if anime {
                    // Une perle de courant qui parcourt le fil.
                    rang += 1.0;
                    let f = ((horloge * 0.3 + rang * 0.37).fract()) as f32;
                    let longueurs: Vec<f32> =
                        points.windows(2).map(|w| (w[1] - w[0]).length()).collect();
                    let total: f32 = longueurs.iter().sum();
                    let mut reste = f * total;
                    for (w, l) in points.windows(2).zip(&longueurs) {
                        if reste <= *l {
                            let p = w[0] + (w[1] - w[0]) * (reste / l.max(0.001));
                            t.p.circle(p, 2.4, c.perle.c(), Stroke::new(1.0, c.un.c()));
                            break;
                        }
                        reste -= l;
                    }
                }
            }
        }
        for e in &doc.elements {
            let centre = vers(e.pos.x, e.pos.y);
            let taille = Vec2::new(e.largeur(), e.hauteur()) * echelle;
            let r = Rect::from_center_size(centre, taille);
            let (fond, bord) = match e.genre {
                Genre::Brique => (c.carte_2.vers(c.accent, 0.25), c.accent_clair),
                Genre::Entree | Genre::Sortie => (c.carte_2, c.un),
                _ => (c.carte_2, c.bord.vers(c.texte, 0.25)),
            };
            t.p.rect(
                r,
                (8.0 * echelle).max(2.0),
                fond.c(),
                Stroke::new(1.2, bord.c()),
                StrokeKind::Inside,
            );
            let cote = taille.x.min(taille.y);
            if e.genre == Genre::Nand && cote > 14.0 {
                t.icone(Icone::Nand, centre, cote * 0.55, c.texte.alpha(0.8));
            }
        }
    }

    // --- Le menu ----------------------------------------------------------------------------

    fn menu(
        &mut self,
        k: &mut Cadre,
        d: &DispositionAccueil,
        at: &Atelier,
        cx: &Contexte,
        demandes: &mut Vec<Demande>,
    ) -> Option<Rubrique> {
        let r = d.menu;
        let c = k.t.c;
        let mut voulue = None;
        k.t.carte(r, k.a.formes.rayon_carte);

        // Le logo : une NAND sur la couleur du 1, qui respire doucement.
        let cote = 36.0;
        let logo = if d.compact {
            Rect::from_center_size(Pos2::new(r.center().x, r.top() + 34.0), Vec2::splat(cote))
        } else {
            Rect::from_min_size(
                Pos2::new(r.left() + 16.0, r.top() + 16.0),
                Vec2::splat(cote),
            )
        };
        let souffle = k.a.mouvement.pouls(at.horloge, 2.5);
        k.t.p.add(halo(
            logo.expand(12.0 + 4.0 * souffle),
            voilee(c.un, 0.10 + 0.06 * souffle),
        ));
        k.t.p.rect_filled(logo, 10.0, c.un.c());
        k.t.reflet(logo, 10.0);
        k.t.icone(
            Icone::Nand,
            logo.center() + Vec2::new(0.5, 0.0),
            22.0,
            c.sur_un.c(),
        );
        if !d.compact {
            // Le nom entier, quitte à le rapetisser quand le texte est très grand.
            let x = logo.right() + 12.0;
            let dispo = r.right() - 16.0 - x;
            for (texte, police, taille, dy, couleur) in [
                (
                    "Computer Engineer",
                    Police::CorpsDemi,
                    k.a.typo.fort,
                    -8.0,
                    c.texte,
                ),
                (
                    "Du NAND à l'ordinateur",
                    Police::Corps,
                    k.a.typo.petit,
                    9.0,
                    c.discret,
                ),
            ] {
                let largeur = k.t.largeur(texte, police, taille);
                let taille = if largeur > dispo {
                    taille * dispo / largeur * 0.98
                } else {
                    taille
                };
                k.t.texte(
                    Pos2::new(x, logo.center().y + dy),
                    Align2::LEFT_CENTER,
                    texte,
                    police,
                    taille,
                    couleur.c(),
                );
            }
        }

        let entrees: [(Rubrique, &str, Icone); 4] = [
            (Rubrique::Accueil, "Accueil", Icone::Maison),
            (Rubrique::Parcours, "Parcours", Icone::Quete),
            (Rubrique::Projets, "Mes projets", Icone::Dossier),
            (Rubrique::Parametres, "Paramètres", Icone::Engrenage),
        ];
        let mut y = r.top() + 76.0;
        k.t.separateur(r.left() + 1.0, r.right() - 1.0, y - 12.0);
        let (x, l) = (r.left() + 10.0, r.width() - 20.0);
        // La pastille de la rubrique affichée glisse d'une entrée à l'autre.
        let m = &k.a.mouvement;
        let ici = entrees
            .iter()
            .position(|(rubrique, ..)| *rubrique == self.rubrique)
            .unwrap_or(0);
        let pastille = k.anims.transition(
            egui::Id::unique("menu-pastille"),
            ici as f32,
            k.dt,
            m.duree(m.rubrique),
            m.courbe_panneau,
        );
        // Entrée choisie : surface enfoncée et trait d'accent à gauche.
        let choisie = Rect::from_min_size(Pos2::new(x, y + pastille * 38.0), Vec2::new(l, 36.0));
        k.t.p
            .rect_filled(choisie, k.a.formes.rayon_bouton, c.enfonce.c());
        if !d.compact {
            k.t.p.rect_filled(
                Rect::from_min_size(
                    Pos2::new(choisie.left(), choisie.top() + 9.0),
                    Vec2::new(2.5, 18.0),
                ),
                1.25,
                c.accent_clair.c(),
            );
        }
        for (i, (rubrique, nom, icone)) in entrees.into_iter().enumerate() {
            let rect = Rect::from_min_size(Pos2::new(x, y), Vec2::new(l, 36.0));
            y += 38.0;
            k.reperer("menu", i, rect);
            let touche = k.toucher(
                rect,
                egui::Id::unique(("menu", i)),
                if d.compact { nom } else { "" },
                Sense::click(),
            );
            let allume = (1.0 - (pastille - i as f32).abs()).clamp(0.0, 1.0);
            let survol = touche.chaud * (1.0 - allume);
            if survol > 0.01 {
                k.t.p
                    .rect_filled(touche.rect, k.a.formes.rayon_bouton, c.survol.alpha(survol));
            }
            let encre = c.doux.vers(c.texte, allume.max(touche.chaud)).c();
            let encre_icone = c
                .discret
                .vers(c.texte, touche.chaud)
                .vers(c.accent_clair, allume)
                .c();
            if d.compact {
                k.t.icone(icone, touche.rect.center(), 18.0, encre_icone);
            } else {
                k.t.icone(
                    icone,
                    Pos2::new(touche.rect.left() + 20.0, rect.center().y),
                    17.0,
                    encre_icone,
                );
                k.t.texte(
                    Pos2::new(touche.rect.left() + 40.0, rect.center().y),
                    Align2::LEFT_CENTER,
                    nom,
                    Police::CorpsMoyen,
                    k.a.typo.fort,
                    encre,
                );
            }
            if touche.clic {
                voulue = Some(rubrique);
            }
        }

        // En bas : reprendre le projet, quitter, la version.
        let mut bas = r.bottom() - 16.0;
        if !d.compact {
            k.t.texte(
                Pos2::new(x + 8.0, bas - 8.0),
                Align2::LEFT_CENTER,
                &k.t.abreger(
                    &format!("Version {} · {}", env!("CARGO_PKG_VERSION"), cx.mise_a_jour),
                    Police::Corps,
                    k.a.typo.legende,
                    l - 16.0,
                ),
                Police::Corps,
                k.a.typo.legende,
                c.discret.c(),
            );
            bas -= 24.0;
        }
        let rect = Rect::from_min_max(Pos2::new(x, bas - 34.0), Pos2::new(x + l, bas));
        if d.compact {
            if k.bouton_icone(
                rect,
                "quitter-icone",
                Icone::Sortie,
                crate::panneaux::Style::Nu,
                "Quitter",
                true,
            ) {
                demandes.push(Demande::Quitter);
            }
            k.reperer("quitter", 0, rect);
        } else if Self::bouton(
            k,
            rect,
            "quitter",
            0,
            "Quitter",
            Some(Icone::Sortie),
            Sorte::Secondaire,
            "Tout est déjà enregistré",
        ) {
            demandes.push(Demande::Quitter);
        }
        bas -= 40.0;
        if cx.courant.is_some() {
            let rect = Rect::from_min_max(Pos2::new(x, bas - 34.0), Pos2::new(x + l, bas));
            let bulle = format!("Revenir à « {} » (Entrée sur l'accueil)", at.projet.nom);
            if d.compact {
                if k.bouton_icone(
                    rect,
                    "continuer-icone",
                    Icone::Lecture,
                    crate::panneaux::Style::Plein,
                    &bulle,
                    true,
                ) {
                    demandes.push(Demande::Continuer);
                }
                k.reperer("continuer", 1, rect);
            } else if Self::bouton(
                k,
                rect,
                "continuer",
                1,
                "Continuer",
                Some(Icone::Lecture),
                Sorte::Secondaire,
                &bulle,
            ) {
                demandes.push(Demande::Continuer);
            }
        }
        voulue
    }

    // --- Accueil ----------------------------------------------------------------------------

    fn page_accueil(
        &mut self,
        k: &mut Cadre,
        d: &DispositionAccueil,
        at: &Atelier,
        cx: &Contexte,
        y: f32,
        demandes: &mut Vec<Demande>,
    ) -> f32 {
        let c = k.t.c;
        let (x, l) = (d.contenu.left(), d.contenu.width());
        let g = k.a.formes.gouttiere + 6.0;
        let faites = PARCOURS.iter().filter(|f| at.parcours.faite(f.cle)).count();
        let prochaine = at.parcours.prochaine();
        let (titre, sous) = if cx.courant.is_some() {
            (
                "Bon retour",
                "Ton travail est enregistré automatiquement. Reprends là où tu t'étais arrêté, ou choisis une autre quête.",
            )
        } else {
            (
                "Bienvenue",
                "Construis un ordinateur entier à partir d'une seule porte, la NAND. Chaque quête réussie devient une brique pour les suivantes.",
            )
        };
        let mut y = Self::titre(k, x, y, l.min(760.0), titre, sous);

        // La grande carte : reprendre le projet, ou commencer le parcours.
        let h = 256.0;
        let rv = k.a.formes.rayon_vitrine;
        let carte = Rect::from_min_size(Pos2::new(x, y), Vec2::new(l, h));
        k.t.carte(carte, rv);
        let l_texte = if d.large { l * 0.48 } else { l - 48.0 };
        let (tx, ty) = (carte.left() + 28.0, carte.top() + 26.0);
        let apercu = Rect::from_min_max(
            Pos2::new(carte.left() + 28.0 + l_texte + 12.0, carte.top() + 12.0),
            Pos2::new(carte.right() - 12.0, carte.bottom() - 12.0),
        );
        let anime = !k.a.mouvement.sobre;
        if cx.courant.is_some() {
            let fiche = cx
                .projets
                .iter()
                .find(|f| Some(f.fichier.as_str()) == cx.courant);
            k.rubrique(tx, ty, l_texte, "Reprendre", "");
            k.t.texte(
                Pos2::new(tx, ty + 42.0),
                Align2::LEFT_CENTER,
                &k.t.abreger(&at.projet.nom, Police::Titre, k.a.typo.grand, l_texte),
                Police::Titre,
                k.a.typo.grand,
                c.texte.c(),
            );
            let quete = at
                .projet
                .quete
                .as_deref()
                .and_then(|cle| PARCOURS.iter().position(|f| f.cle == cle));
            let ligne = match quete {
                Some(i) => format!(
                    "Quête {} · {} — {}",
                    i + 1,
                    PARCOURS[i].titre,
                    PARCOURS[i].chapitre
                ),
                None => "Projet libre".to_owned(),
            };
            k.t.texte(
                Pos2::new(tx, ty + 74.0),
                Align2::LEFT_CENTER,
                &k.t.abreger(&ligne, Police::CorpsMoyen, k.a.typo.corps, l_texte),
                Police::CorpsMoyen,
                k.a.typo.corps,
                c.doux.c(),
            );
            let mut details = vec![
                pluriel(at.projet.principal.elements.len(), "élément"),
                pluriel(at.projet.modeles.len(), "brique"),
            ];
            if let Some(quand) = fiche.and_then(|f| f.modifie) {
                details.push(format!("modifié {}", il_y_a(cx.maintenant, quand)));
            }
            k.t.texte(
                Pos2::new(tx, ty + 96.0),
                Align2::LEFT_CENTER,
                &details.join(" · "),
                Police::Corps,
                k.a.typo.petit,
                c.discret.c(),
            );
            let lb = Self::largeur_bouton(k, "Continuer", Some(Icone::Lecture)) + 24.0;
            let b = Rect::from_min_size(
                Pos2::new(tx, carte.bottom() - 28.0 - 38.0),
                Vec2::new(lb, 38.0),
            );
            if Self::bouton(
                k,
                b,
                "continuer",
                0,
                "Continuer",
                Some(Icone::Lecture),
                Sorte::Principal,
                "",
            ) {
                demandes.push(Demande::Continuer);
            }
            // Le raccourci, montré comme une touche de clavier.
            let fin = k.t.texte(
                Pos2::new(b.right() + 14.0, b.center().y),
                Align2::LEFT_CENTER,
                "ou",
                Police::Corps,
                k.a.typo.petit,
                c.discret.c(),
            );
            Self::touche_clavier(&k.t, Pos2::new(fin.right() + 6.0, b.center().y), "Entrée");
            if d.large {
                Self::miniature(&k.t, apercu, &at.projet.principal, at.horloge, anime);
            }
        } else {
            k.rubrique(tx, ty, l_texte, "Premiers pas", "");
            let titre = match prochaine {
                Some(f) => f.titre,
                None => "Le parcours est terminé",
            };
            k.t.texte(
                Pos2::new(tx, ty + 46.0),
                Align2::LEFT_CENTER,
                &k.t.abreger(titre, Police::Titre, k.a.typo.grand, l_texte),
                Police::Titre,
                k.a.typo.grand,
                c.texte.c(),
            );
            if let Some(f) = prochaine {
                let bas_texte = carte.bottom() - 28.0 - 38.0 - 14.0;
                let zone = Rect::from_min_max(
                    Pos2::new(tx, ty + 72.0),
                    Pos2::new(tx + l_texte, bas_texte),
                );
                k.t.sur(1.0, zone).paragraphe(
                    zone.min,
                    l_texte,
                    f.consigne,
                    Police::Corps,
                    k.a.typo.fort,
                    c.doux.c(),
                );
                let lb = Self::largeur_bouton(k, "Commencer la quête", Some(Icone::Lecture)) + 24.0;
                let b = Rect::from_min_size(
                    Pos2::new(tx, carte.bottom() - 28.0 - 38.0),
                    Vec2::new(lb, 38.0),
                );
                if Self::bouton(
                    k,
                    b,
                    "commencer-le-parcours",
                    0,
                    "Commencer la quête",
                    Some(Icone::Lecture),
                    Sorte::Principal,
                    "",
                ) {
                    demandes.push(Demande::Ouvrir(Ouverture::Quete(f.cle.to_owned())));
                }
            }
            if d.large {
                Self::miniature(
                    &k.t,
                    apercu,
                    &Document::demi_additionneur(),
                    at.horloge,
                    anime,
                );
            }
        }
        y += h + g;

        // Deux cartes : la prochaine quête, l'avancement.
        // L'avancement donne sa hauteur aux deux cartes : une ligne de 30 points par chapitre
        // (deux ou trois colonnes selon la place), sous le grand chiffre.
        let l_avancement = if d.large { l - (l - g) * 0.46 - g } else { l };
        let lignes_chap = chapitres()
            .len()
            .div_ceil(Self::colonnes_avancement(k, l_avancement)) as f32;
        let h2 = (22.0 + 92.0 + lignes_chap * 30.0 + 14.0).max(224.0);
        let (ca, cb) = if d.large {
            let la = (l - g) * 0.46;
            (
                Rect::from_min_size(Pos2::new(x, y), Vec2::new(la, h2)),
                Rect::from_min_size(Pos2::new(x + la + g, y), Vec2::new(l - la - g, h2)),
            )
        } else {
            (
                Rect::from_min_size(Pos2::new(x, y), Vec2::new(l, h2)),
                Rect::from_min_size(Pos2::new(x, y + h2 + g), Vec2::new(l, h2)),
            )
        };
        y = cb.bottom() + g;

        k.t.carte(ca, rv);
        let (ax, ay, al) = (ca.left() + 24.0, ca.top() + 22.0, ca.width() - 48.0);
        // Au premier lancement, la grande carte présente déjà la première quête : celle-ci
        // explique plutôt comment on joue.
        let premiere_fois = cx.courant.is_none() && faites == 0;
        match prochaine.filter(|_| !premiere_fois) {
            None if premiere_fois => {
                k.rubrique(ax, ay, al, "Comment ça marche", "");
                if Self::lien(
                    k,
                    Pos2::new(ax + al, ay + 8.0),
                    "Voir le parcours",
                    "voir-le-parcours",
                ) {
                    self.montrer_quete(0);
                }
                let etapes = [
                    "Lis la consigne : elle dit quoi construire, avec un indice.",
                    "Glisse des NAND sur le plan et relie-les en tirant des fils.",
                    "Clique sur les interrupteurs pour essayer, puis sur Vérifier.",
                    "Réussie, la quête devient une brique pour les suivantes.",
                ];
                // Les étapes se partagent la hauteur de la carte.
                let mut ye = ay + 32.0;
                let pas = ((ca.bottom() - 18.0 - ye) / etapes.len() as f32).min(42.0);
                for (i, etape) in etapes.iter().enumerate() {
                    let rond = Pos2::new(ax + 12.0, ye + 10.0);
                    k.t.p
                        .circle(rond, 11.0, c.enfonce.c(), Stroke::new(1.0, c.bord_fort.c()));
                    k.t.texte(
                        rond,
                        Align2::CENTER_CENTER,
                        &(i + 1).to_string(),
                        Police::MonoGras,
                        k.a.typo.petit,
                        c.texte.c(),
                    );
                    k.t.texte(
                        Pos2::new(ax + 34.0, ye + 10.0),
                        Align2::LEFT_CENTER,
                        &k.t.abreger(etape, Police::Corps, k.a.typo.fort, al - 34.0),
                        Police::Corps,
                        k.a.typo.fort,
                        c.texte.alpha(0.88),
                    );
                    ye += pas;
                }
            }
            Some(f) => {
                let rang = PARCOURS.iter().position(|g| g.cle == f.cle).unwrap_or(0);
                k.rubrique(ax, ay, al, "Prochaine quête", "");
                k.t.texte(
                    Pos2::new(ax + al, ay + 8.0),
                    Align2::RIGHT_CENTER,
                    f.chapitre,
                    Police::CorpsDemi,
                    k.a.typo.petit,
                    c.accent_clair.c(),
                );
                k.t.texte(
                    Pos2::new(ax, ay + 40.0),
                    Align2::LEFT_CENTER,
                    &k.t.abreger(
                        &format!("{}. {}", rang + 1, f.titre),
                        Police::Titre,
                        k.a.typo.titre,
                        al,
                    ),
                    Police::Titre,
                    k.a.typo.titre,
                    c.texte.c(),
                );
                let zone = Rect::from_min_max(
                    Pos2::new(ax, ay + 60.0),
                    Pos2::new(ax + al, ca.bottom() - 22.0 - 34.0 - 10.0),
                );
                k.t.sur(1.0, zone).paragraphe(
                    zone.min,
                    al,
                    f.consigne,
                    Police::Corps,
                    k.a.typo.fort,
                    c.doux.c(),
                );
                let b = Rect::from_min_size(
                    Pos2::new(ax, ca.bottom() - 22.0 - 34.0),
                    Vec2::new(Self::largeur_bouton(k, "Commencer", None) + 16.0, 34.0),
                );
                // Secondaire : l'action principale de la page est en haut (Continuer).
                if Self::bouton(
                    k,
                    b,
                    "prochaine-quete",
                    0,
                    "Commencer",
                    None,
                    if cx.courant.is_some() {
                        Sorte::Secondaire
                    } else {
                        Sorte::Principal
                    },
                    "",
                ) {
                    demandes.push(Demande::Ouvrir(Ouverture::Quete(f.cle.to_owned())));
                }
                let texte = "Voir le parcours";
                let lb = Self::largeur_bouton(k, texte, Some(Icone::Fleche));
                let b2 =
                    Rect::from_min_size(Pos2::new(b.right() + 8.0, b.top()), Vec2::new(lb, 34.0));
                if Self::bouton(
                    k,
                    b2,
                    "voir-le-parcours",
                    0,
                    texte,
                    Some(Icone::Fleche),
                    Sorte::Secondaire,
                    "",
                ) {
                    self.montrer_quete(rang);
                }
            }
            None => {
                k.rubrique(ax, ay, al, "Parcours", "");
                k.t.paragraphe(
                    Pos2::new(ax, ay + 32.0),
                    al,
                    "Toutes les quêtes sont réussies, jusqu'à l'ordinateur complet. Tes briques t'attendent dans un projet libre.",
                    Police::Corps,
                    k.a.typo.fort,
                    c.doux.c(),
                );
            }
        }

        k.t.carte(cb, rv);
        let (bx, by, bl) = (cb.left() + 24.0, cb.top() + 22.0, cb.width() - 48.0);
        k.rubrique(bx, by, bl, "Ton avancement", "");
        let fin = k.t.texte(
            Pos2::new(bx, by + 44.0),
            Align2::LEFT_CENTER,
            &format!("{faites}"),
            Police::Titre,
            k.a.typo.affiche,
            c.texte.c(),
        );
        k.t.texte(
            Pos2::new(fin.right() + 8.0, by + 48.0),
            Align2::LEFT_CENTER,
            &format!("quêtes réussies sur {}", PARCOURS.len()),
            Police::CorpsMoyen,
            k.a.typo.fort,
            c.doux.c(),
        );
        Self::barre(
            &k.t,
            Rect::from_min_size(Pos2::new(bx, by + 72.0), Vec2::new(bl, 6.0)),
            self.plein * faites as f32 / PARCOURS.len().max(1) as f32,
            c.un,
        );
        let chap = chapitres();
        let colonnes = Self::colonnes_avancement(k, cb.width());
        let lc = (bl - 18.0 * (colonnes - 1) as f32) / colonnes as f32;
        let lignes = chap.len().div_ceil(colonnes);
        let hl = ((cb.bottom() - 14.0 - (by + 92.0)) / lignes.max(1) as f32).clamp(26.0, 34.0);
        for (i, (nom, quetes)) in chap.iter().enumerate() {
            let (col, lig) = (i / lignes, i % lignes);
            let px = bx + col as f32 * (lc + 18.0);
            let py = by + 92.0 + lig as f32 * hl;
            let n = quetes.len();
            let f = quetes
                .iter()
                .filter(|q| at.parcours.faite(PARCOURS[**q].cle))
                .count();
            let compte = format!("{f}/{n}");
            let lcompte = k.t.largeur(&compte, Police::Mono, k.a.typo.legende);
            k.t.texte(
                Pos2::new(px, py + 8.0),
                Align2::LEFT_CENTER,
                &k.t.abreger(nom, Police::Corps, k.a.typo.petit, lc - lcompte - 8.0),
                Police::Corps,
                k.a.typo.petit,
                if f == n {
                    c.un.c()
                } else {
                    c.texte.alpha(0.85)
                },
            );
            k.t.texte(
                Pos2::new(px + lc, py + 8.0),
                Align2::RIGHT_CENTER,
                &compte,
                Police::Mono,
                k.a.typo.legende,
                c.doux.c(),
            );
            Self::barre(
                &k.t,
                Rect::from_min_size(Pos2::new(px, py + 19.0), Vec2::new(lc, 4.0)),
                self.plein * f as f32 / n.max(1) as f32,
                if f == n { c.un } else { c.accent_clair },
            );
        }

        // Les projets récents, et de quoi en commencer un.
        k.rubrique(x + 4.0, y, l - 8.0, "Projets récents", "");
        if Self::lien(
            k,
            Pos2::new(x + l, y + 8.0),
            "Tous les projets",
            "tous-les-projets",
        ) {
            self.aller(Rubrique::Projets);
        }
        y += 30.0;
        let recents: Vec<&FicheProjet> = cx
            .projets
            .iter()
            .filter(|f| Some(f.fichier.as_str()) != cx.courant && !f.illisible)
            .take(3)
            .collect();
        let n = 4;
        let lt = (l - g * (n - 1) as f32) / n as f32;
        let ht = 96.0;
        for i in 0..n {
            let r = Rect::from_min_size(Pos2::new(x + i as f32 * (lt + g), y), Vec2::new(lt, ht));
            let fiche = recents.get(i).copied();
            if fiche.is_none() && i != recents.len() {
                continue;
            }
            let (repere, rang) = match fiche {
                Some(_) => ("projet-recent", i),
                None => ("nouveau", 0),
            };
            k.reperer(repere, rang, r);
            let touche = k.toucher(r, egui::Id::unique((repere, rang)), "", Sense::click());
            match fiche {
                Some(f) => {
                    k.t.carte(touche.rect, rv);
                    if touche.chaud > 0.01 {
                        k.t.p.rect(
                            touche.rect,
                            rv,
                            c.survol.alpha(0.6 * touche.chaud),
                            Stroke::new(1.0, c.bord_fort.alpha(touche.chaud)),
                            StrokeKind::Inside,
                        );
                    }
                    let quete = f
                        .quete
                        .as_deref()
                        .and_then(|q| PARCOURS.iter().find(|p| p.cle == q).map(|p| p.titre));
                    k.t.icone(
                        if quete.is_some() {
                            Icone::Quete
                        } else {
                            Icone::Brique
                        },
                        touche.rect.left_top() + Vec2::new(26.0, 28.0),
                        17.0,
                        c.discret.vers(c.accent_clair, touche.chaud).c(),
                    );
                    k.t.texte(
                        touche.rect.left_top() + Vec2::new(46.0, 28.0),
                        Align2::LEFT_CENTER,
                        &k.t.abreger(&f.nom, Police::CorpsDemi, k.a.typo.fort, lt - 60.0),
                        Police::CorpsDemi,
                        k.a.typo.fort,
                        c.texte.c(),
                    );
                    k.t.texte(
                        touche.rect.left_top() + Vec2::new(18.0, 56.0),
                        Align2::LEFT_CENTER,
                        &k.t.abreger(
                            quete.unwrap_or("Projet libre"),
                            Police::Corps,
                            k.a.typo.petit,
                            lt - 36.0,
                        ),
                        Police::Corps,
                        k.a.typo.petit,
                        c.doux.c(),
                    );
                    if let Some(quand) = f.modifie {
                        k.t.texte(
                            touche.rect.left_top() + Vec2::new(18.0, 76.0),
                            Align2::LEFT_CENTER,
                            &il_y_a(cx.maintenant, quand),
                            Police::Corps,
                            k.a.typo.legende,
                            c.discret.c(),
                        );
                    }
                    if touche.clic {
                        demandes.push(Demande::Ouvrir(Ouverture::Projet(f.fichier.clone())));
                    }
                }
                None => {
                    // Une case en pointillés : la place d'un projet qui n'existe pas encore.
                    k.t.p
                        .rect_filled(touche.rect, rv, c.survol.alpha(0.5 * touche.chaud));
                    k.t.p.extend(Shape::dashed_line(
                        &crate::dessin::fermer(crate::dessin::contour_arrondi(
                            touche.rect.shrink(0.5),
                            rv,
                        )),
                        Stroke::new(1.0, c.bord_fort.vers(c.doux, touche.chaud).c()),
                        5.0,
                        4.0,
                    ));
                    k.t.icone(
                        Icone::Plus,
                        Pos2::new(touche.rect.center().x, touche.rect.top() + 36.0),
                        18.0,
                        c.doux.vers(c.texte, touche.chaud).c(),
                    );
                    k.t.texte(
                        Pos2::new(touche.rect.center().x, touche.rect.top() + 66.0),
                        Align2::CENTER_CENTER,
                        "Nouveau projet",
                        Police::CorpsMoyen,
                        k.a.typo.corps,
                        c.doux.vers(c.texte, touche.chaud).c(),
                    );
                    if touche.clic {
                        demandes.push(Demande::Ouvrir(Ouverture::Nouveau));
                    }
                }
            }
        }
        y + ht
    }

    // --- Parcours ---------------------------------------------------------------------------

    fn page_parcours(
        &mut self,
        k: &mut Cadre,
        d: &DispositionAccueil,
        at: &Atelier,
        cx: &Contexte,
        y: f32,
        demandes: &mut Vec<Demande>,
    ) -> f32 {
        let c = k.t.c;
        let (x, l) = (d.contenu.left(), d.contenu.width());
        let g = k.a.formes.gouttiere + 4.0;
        let faites = PARCOURS.iter().filter(|f| at.parcours.faite(f.cle)).count();
        let y = Self::titre(
            k,
            x,
            y,
            l.min(760.0),
            "Parcours",
            &format!(
                "{faites} quêtes réussies sur {}. Le parcours principal mène à l'ordinateur complet, et des branches à d'autres grands éléments. Chaque quête réussie te donne une brique pour les suivantes.",
                PARCOURS.len()
            ),
        );
        let chap = chapitres();
        self.chapitre = self.chapitre.min(chap.len().saturating_sub(1));
        let prochaine = at.parcours.prochaine().map(|f| f.cle);

        // Les chapitres.
        let lg = if d.large { 268.0 } else { 220.0 };
        let mut yg = y;
        let mut branche: Option<Branche> = None;
        for (i, (nom, quetes)) in chap.iter().enumerate() {
            // Au début de chaque branche, son nom : le parcours principal, puis les branches.
            let ici_branche = savoir::chapitre(nom).map_or(Branche::Tronc, |c| c.branche);
            if branche != Some(ici_branche) {
                if branche.is_some() {
                    yg += 8.0;
                }
                branche = Some(ici_branche);
                let couleur = if ici_branche == Branche::Tronc {
                    c.doux
                } else {
                    c.accent_clair
                };
                k.t.texte(
                    Pos2::new(x + 4.0, yg + 9.0),
                    Align2::LEFT_CENTER,
                    &k.t.abreger(
                        &ici_branche.nom().to_uppercase(),
                        Police::CorpsDemi,
                        k.a.typo.legende,
                        lg - 8.0,
                    ),
                    Police::CorpsDemi,
                    k.a.typo.legende,
                    couleur.c(),
                );
                yg += 26.0;
            }
            let r = Rect::from_min_size(Pos2::new(x, yg), Vec2::new(lg, 60.0));
            yg += 60.0 + 4.0;
            k.reperer("chapitre", i, r);
            let touche = k.toucher(r, egui::Id::unique(("chapitre", i)), "", Sense::click());
            let ici = i == self.chapitre;
            let choisi = k.anims.vers(
                egui::Id::unique(("chapitre-choisi", i)),
                if ici { 1.0 } else { 0.0 },
                k.dt,
                k.a.mouvement.lissage_ui,
            );
            let n = quetes.len();
            let f = quetes
                .iter()
                .filter(|q| at.parcours.faite(PARCOURS[**q].cle))
                .count();
            let rv = k.a.formes.rayon_carte;
            k.t.p.rect(
                touche.rect,
                rv,
                c.carte
                    .vers(c.survol, touche.chaud)
                    .vers(c.enfonce, choisi)
                    .alpha(0.85 + 0.15 * choisi.max(touche.chaud)),
                Stroke::new(
                    1.0,
                    c.bord
                        .vers(c.bord_fort, touche.chaud)
                        .vers(c.accent_clair.vers(c.bord, 0.4), choisi)
                        .c(),
                ),
                StrokeKind::Inside,
            );
            if choisi > 0.01 {
                k.t.p.rect_filled(
                    Rect::from_min_size(
                        Pos2::new(touche.rect.left(), touche.rect.top() + 14.0),
                        Vec2::new(2.5, (touche.rect.height() - 28.0) * choisi),
                    ),
                    1.25,
                    c.accent_clair.c(),
                );
            }
            k.t.legende(
                Pos2::new(r.left() + 16.0, r.top() + 15.0),
                Align2::LEFT_CENTER,
                &format!("CHAPITRE {}", i + 1),
                c.discret.c(),
            );
            if f == n {
                k.t.icone(
                    Icone::Coche,
                    Pos2::new(r.right() - 22.0, r.top() + 15.0),
                    16.0,
                    c.un.c(),
                );
            }
            k.t.texte(
                Pos2::new(r.left() + 16.0, r.top() + 32.0),
                Align2::LEFT_CENTER,
                &k.t.abreger(nom, Police::CorpsDemi, k.a.typo.corps, lg - 32.0),
                Police::CorpsDemi,
                k.a.typo.corps,
                c.texte.c(),
            );
            let compte = format!("{f}/{n}");
            k.t.texte(
                Pos2::new(r.right() - 16.0, r.top() + 47.0),
                Align2::RIGHT_CENTER,
                &compte,
                Police::Mono,
                k.a.typo.legende,
                c.doux.c(),
            );
            Self::barre(
                &k.t,
                Rect::from_min_size(
                    Pos2::new(r.left() + 16.0, r.top() + 45.5),
                    Vec2::new(lg - 32.0 - 40.0, 4.0),
                ),
                self.plein * f as f32 / n.max(1) as f32,
                if f == n { c.un } else { c.accent_clair },
            );
            if touche.clic && !ici {
                self.chapitre = i;
                self.quete = None;
            }
        }

        // Les quêtes du chapitre.
        let Some((nom, quetes)) = chap.get(self.chapitre) else {
            return yg;
        };
        let reste = l - lg - g;
        let l_fiche = if d.large { (reste - g) * 0.47 } else { 0.0 };
        let lq = if d.large { reste - l_fiche - g } else { reste };
        let xq = x + lg + g;
        if self
            .quete
            .is_none_or(|q| PARCOURS.get(q).is_none_or(|f| f.chapitre != *nom))
        {
            // Par défaut : la quête conseillée du chapitre, sinon sa première.
            self.quete = quetes
                .iter()
                .copied()
                .find(|q| Some(PARCOURS[*q].cle) == prochaine)
                .or_else(|| quetes.first().copied());
        }
        let mut yq = y;
        let debut_quetes = calques::repere(&k.t.p);
        // Ce qu'on apprend dans ce chapitre.
        if let Some(presentation) = savoir::chapitre(nom) {
            yq = Self::presentation_du_chapitre(k, presentation, xq, yq, lq);
        }
        for &q in quetes {
            let f = &PARCOURS[q];
            let r = Rect::from_min_size(Pos2::new(xq, yq), Vec2::new(lq, 56.0));
            yq += 56.0 + 4.0;
            k.reperer("quete", q, r);
            let touche = k.toucher(r, egui::Id::unique(("quete", q)), "", Sense::click());
            let choisie = k.anims.vers(
                egui::Id::unique(("quete-choisie", q)),
                if self.quete == Some(q) { 1.0 } else { 0.0 },
                k.dt,
                k.a.mouvement.lissage_ui,
            );
            let faite = at.parcours.faite(f.cle);
            let conseillee = prochaine == Some(f.cle);
            let rv = k.a.formes.rayon_carte;
            k.t.p.rect(
                touche.rect,
                rv,
                c.carte
                    .vers(c.survol, touche.chaud)
                    .vers(c.enfonce, choisie)
                    .alpha(0.85 + 0.15 * choisie.max(touche.chaud)),
                Stroke::new(
                    1.0,
                    c.bord
                        .vers(c.bord_fort, touche.chaud)
                        .vers(c.accent_clair.vers(c.bord, 0.4), choisie)
                        .c(),
                ),
                StrokeKind::Inside,
            );
            let rond = Pos2::new(r.left() + 28.0, r.center().y);
            if faite {
                k.t.p.circle_filled(rond, 13.0, c.un.alpha(0.14));
                k.t.icone(Icone::Coche, rond, 15.0, c.un.c());
            } else {
                k.t.p.circle_stroke(
                    rond,
                    13.0,
                    Stroke::new(
                        1.0,
                        if conseillee {
                            c.accent_clair.c()
                        } else {
                            c.bord.c()
                        },
                    ),
                );
                k.t.texte(
                    rond,
                    Align2::CENTER_CENTER,
                    &(q + 1).to_string(),
                    Police::Mono,
                    k.a.typo.petit,
                    if conseillee {
                        c.accent_clair.c()
                    } else {
                        c.doux.c()
                    },
                );
            }
            let l_etiquette = match (faite, conseillee) {
                (true, _) => Self::etiquette(
                    &k.t,
                    Pos2::new(r.right() - 14.0, r.center().y),
                    "Réussie",
                    c.un,
                ),
                (false, true) => Self::etiquette(
                    &k.t,
                    Pos2::new(r.right() - 14.0, r.center().y),
                    "Conseillée",
                    c.accent_clair,
                ),
                _ => 0.0,
            };
            let lt = lq - 60.0 - l_etiquette - 24.0;
            k.t.texte(
                Pos2::new(r.left() + 58.0, r.center().y - 9.0),
                Align2::LEFT_CENTER,
                &k.t.abreger(f.titre, Police::CorpsDemi, k.a.typo.fort, lt),
                Police::CorpsDemi,
                k.a.typo.fort,
                c.texte.c(),
            );
            let gain = if matches!(f.epreuve, Epreuve::Systeme { .. }) {
                "Grand projet : plusieurs grandes pièces".to_owned()
            } else if f.epreuve.ordinateur() {
                "Épreuve finale : un ordinateur".to_owned()
            } else {
                format!("Brique gagnée : {}", f.brique)
            };
            k.t.texte(
                Pos2::new(r.left() + 58.0, r.center().y + 11.0),
                Align2::LEFT_CENTER,
                &k.t.abreger(&gain, Police::Corps, k.a.typo.petit, lt),
                Police::Corps,
                k.a.typo.petit,
                c.doux.c(),
            );
            if touche.clic {
                self.quete = Some(q);
            }
        }

        // Les quêtes d'un autre chapitre arrivent en glissant.
        let glissement = k.a.mouvement.glissement;
        self.arriver(
            k,
            at.horloge,
            "quetes",
            self.chapitre,
            debut_quetes,
            Vec2::new(0.0, glissement),
        );

        // La fiche de la quête choisie.
        let (xf, yf, lf) = if d.large {
            (xq + lq + g, y, l_fiche)
        } else {
            (xq, yq + 8.0, lq)
        };
        let debut_fiche = calques::repere(&k.t.p);
        let bas_fiche = match self.quete.and_then(|q| PARCOURS.get(q).map(|f| (q, f))) {
            Some((q, f)) => self.fiche_de_quete(k, at, cx, q, f, xf, yf, lf, demandes),
            None => yf,
        };
        self.arriver(
            k,
            at.horloge,
            "fiche",
            self.quete.unwrap_or(usize::MAX),
            debut_fiche,
            Vec2::new(glissement, 0.0),
        );
        yg.max(yq).max(bas_fiche)
    }

    /// La carte qui présente un chapitre : sa branche, ce qu'on y apprend. Renvoie le bas.
    fn presentation_du_chapitre(
        k: &mut Cadre,
        p: &savoir::Chapitre,
        x: f32,
        y: f32,
        l: f32,
    ) -> f32 {
        let c = k.t.c;
        let (ix, il) = (x + 18.0, l - 36.0);
        let texte = if p.branche == Branche::Tronc {
            p.apprend.to_owned()
        } else {
            format!("{} {}", p.branche.but(), p.apprend)
        };
        let h_texte =
            k.t.p
                .layout(
                    texte.clone(),
                    Police::Corps.taille(k.a.typo.corps * k.a.formes.echelle_texte),
                    c.texte.c(),
                    il,
                )
                .size()
                .y;
        let carte =
            Rect::from_min_size(Pos2::new(x, y), Vec2::new(l, 16.0 + 22.0 + h_texte + 14.0));
        k.reperer("presentation-chapitre", 0, carte);
        // Un encart : fond légèrement teinté, filet d'accent à gauche.
        k.t.p.rect(
            carte,
            k.a.formes.rayon_carte,
            c.accent.alpha(0.06),
            Stroke::new(1.0, c.accent_clair.alpha(0.18)),
            StrokeKind::Inside,
        );
        k.t.p.rect_filled(
            Rect::from_min_size(
                carte.left_top() + Vec2::new(0.0, 12.0),
                Vec2::new(2.5, carte.height() - 24.0),
            ),
            1.25,
            c.accent_clair.alpha(0.8),
        );
        k.t.icone(
            Icone::Infos,
            Pos2::new(ix + 7.0, carte.top() + 16.0 + 8.0),
            14.0,
            c.accent_clair.c(),
        );
        k.t.legende(
            Pos2::new(ix + 22.0, carte.top() + 16.0 + 8.0),
            Align2::LEFT_CENTER,
            "CE QUE TU APPRENDS ICI",
            c.accent_clair.c(),
        );
        k.t.paragraphe(
            Pos2::new(ix, carte.top() + 16.0 + 22.0),
            il,
            &texte,
            Police::Corps,
            k.a.typo.corps,
            c.texte.alpha(0.9),
        );
        carte.bottom() + 10.0
    }

    /// La fiche d'une quête : consigne, brique gagnée, épreuve, commencer ou reprendre.
    #[allow(clippy::too_many_arguments)]
    fn fiche_de_quete(
        &mut self,
        k: &mut Cadre,
        at: &Atelier,
        cx: &Contexte,
        q: usize,
        f: &'static Fiche,
        x: f32,
        y: f32,
        l: f32,
        demandes: &mut Vec<Demande>,
    ) -> f32 {
        let c = k.t.c;
        let (ix, il) = (x + 24.0, l - 48.0);
        // Hauteur du contenu, mesurée avant de dessiner la carte dessous.
        let mesure_sur = |largeur: f32, taille: f32, police: Police, texte: &str| {
            k.t.p
                .layout(
                    texte.to_owned(),
                    police.taille(taille * k.a.formes.echelle_texte),
                    c.texte.c(),
                    largeur,
                )
                .size()
                .y
        };
        let mesure =
            |taille: f32, police: Police, texte: &str| mesure_sur(il, taille, police, texte);
        let ty = k.a.typo.clone();
        let h_titre = mesure(ty.titre, Police::Titre, f.titre);
        let h_consigne = mesure(ty.fort, Police::Corps, f.consigne);
        // La petite fiche « Comprendre » : six paragraphes titrés.
        let fiche = savoir::savoir(f.cle);
        let h_savoir = fiche.map_or(0.0, |s| {
            26.0 + s
                .rubriques()
                .iter()
                .map(|(_, t)| 18.0 + mesure_sur(il - 12.0, ty.corps, Police::Corps, t) + 10.0)
                .sum::<f32>()
                + 8.0
        });
        let epreuve = epreuve_lisible(f.epreuve);
        let h_epreuve = mesure(ty.petit, Police::Corps, epreuve);
        let enregistre = cx
            .projets
            .iter()
            .find(|p| p.quete.as_deref() == Some(f.cle) && !p.illisible);
        let dans_l_atelier = cx.courant.is_some() && at.projet.quete.as_deref() == Some(f.cle);
        let reprendre = dans_l_atelier || enregistre.is_some();
        let h = 22.0
            + 24.0
            + h_titre
            + 14.0
            + h_consigne
            + 16.0
            + h_savoir
            + 48.0
            + h_epreuve
            + 18.0
            + 38.0
            + if reprendre { 46.0 } else { 0.0 }
            + 22.0;
        let carte = Rect::from_min_size(Pos2::new(x, y), Vec2::new(l, h));
        k.t.carte(carte, k.a.formes.rayon_vitrine);
        let mut y = carte.top() + 22.0;
        let faite = at.parcours.faite(f.cle);
        k.rubrique(
            ix,
            y,
            il,
            &format!("Quête {} sur {}", q + 1, PARCOURS.len()),
            "",
        );
        if faite {
            Self::etiquette(&k.t, Pos2::new(ix + il, y + 8.0), "Réussie", c.un);
        }
        y += 24.0;
        y += k.t.paragraphe(
            Pos2::new(ix, y),
            il,
            f.titre,
            Police::Titre,
            k.a.typo.titre,
            c.texte.c(),
        ) + 14.0;
        y += k.t.paragraphe(
            Pos2::new(ix, y),
            il,
            f.consigne,
            Police::Corps,
            k.a.typo.fort,
            c.texte.alpha(0.88),
        ) + 16.0;
        // Comprendre ce qu'on construit.
        if let Some(s) = fiche {
            let debut = y;
            k.reperer(
                "comprendre-fiche",
                0,
                Rect::from_min_size(Pos2::new(ix, y), Vec2::new(il, h_savoir)),
            );
            k.t.icone(
                Icone::Infos,
                Pos2::new(ix + 7.0, y + 8.0),
                14.0,
                c.accent_clair.c(),
            );
            k.t.legende(
                Pos2::new(ix + 22.0, y + 8.0),
                Align2::LEFT_CENTER,
                "COMPRENDRE",
                c.accent_clair.c(),
            );
            y += 26.0;
            for (titre, texte) in s.rubriques() {
                k.t.texte(
                    Pos2::new(ix + 12.0, y + 7.0),
                    Align2::LEFT_CENTER,
                    titre,
                    Police::CorpsDemi,
                    k.a.typo.petit,
                    c.texte.c(),
                );
                y += 18.0;
                y += k.t.paragraphe(
                    Pos2::new(ix + 12.0, y),
                    il - 12.0,
                    texte,
                    Police::Corps,
                    k.a.typo.corps,
                    c.doux.c(),
                ) + 10.0;
            }
            k.t.p.line_segment(
                [
                    Pos2::new(ix + 1.0, debut + 24.0),
                    Pos2::new(ix + 1.0, y - 10.0),
                ],
                Stroke::new(2.0, c.accent_clair.alpha(0.35)),
            );
            y += 8.0;
        }
        // Ce qu'on gagne et comment c'est vérifié.
        let gain = Rect::from_min_size(Pos2::new(ix, y), Vec2::new(il, 40.0));
        k.t.element(gain, k.a.formes.rayon_bouton + 2.0, 0.0, false);
        k.t.icone(
            if f.epreuve.ordinateur() {
                Icone::Ecran
            } else {
                Icone::Brique
            },
            Pos2::new(gain.left() + 20.0, gain.center().y),
            16.0,
            c.un.c(),
        );
        let texte = match (f.epreuve.ordinateur(), faite) {
            (true, _) if matches!(f.epreuve, Epreuve::Systeme { .. }) => {
                "Tu construis un grand projet qui tourne".to_owned()
            }
            (true, _) => "Tu construis un ordinateur qui tourne".to_owned(),
            (false, true) => format!("Brique gagnée : « {} »", f.brique),
            (false, false) => format!("Tu gagnes la brique « {} »", f.brique),
        };
        k.t.texte(
            Pos2::new(gain.left() + 38.0, gain.center().y),
            Align2::LEFT_CENTER,
            &k.t.abreger(&texte, Police::CorpsMoyen, k.a.typo.corps, il - 50.0),
            Police::CorpsMoyen,
            k.a.typo.corps,
            c.texte.c(),
        );
        y += 48.0;
        y += k.t.paragraphe(
            Pos2::new(ix, y),
            il,
            epreuve,
            Police::Corps,
            k.a.typo.petit,
            c.discret.c(),
        ) + 18.0;

        // Les boutons.
        if reprendre {
            let nom = if dans_l_atelier {
                at.projet.nom.clone()
            } else {
                enregistre.map(|p| p.nom.clone()).unwrap_or_default()
            };
            let r = Rect::from_min_size(Pos2::new(ix, y), Vec2::new(il, 38.0));
            if Self::bouton(
                k,
                r,
                "reprendre",
                0,
                &k.t.abreger(
                    &format!("Reprendre « {nom} »"),
                    Police::CorpsDemi,
                    k.a.typo.fort,
                    il - 70.0,
                ),
                Some(Icone::Lecture),
                Sorte::Principal,
                "Ton travail sur cette quête, là où tu l'as laissé",
            ) {
                demandes.push(if dans_l_atelier {
                    Demande::Continuer
                } else {
                    Demande::Ouvrir(Ouverture::Projet(
                        enregistre.map(|p| p.fichier.clone()).unwrap_or_default(),
                    ))
                });
            }
            y += 46.0;
        }
        let r = Rect::from_min_size(Pos2::new(ix, y), Vec2::new(il, 38.0));
        let (texte, genre, bulle) = if reprendre {
            (
                "Recommencer dans un nouveau projet",
                Sorte::Secondaire,
                "Ton travail précédent reste dans Mes projets",
            )
        } else if faite {
            ("Rejouer cette quête", Sorte::Principal, "")
        } else {
            ("Commencer cette quête", Sorte::Principal, "")
        };
        if Self::bouton(
            k,
            r,
            "commencer",
            0,
            texte,
            if reprendre {
                None
            } else {
                Some(Icone::Lecture)
            },
            genre,
            bulle,
        ) {
            demandes.push(Demande::Ouvrir(Ouverture::Quete(f.cle.to_owned())));
        }
        carte.bottom()
    }

    // --- Projets ----------------------------------------------------------------------------

    fn page_projets(
        &mut self,
        k: &mut Cadre,
        d: &DispositionAccueil,
        cx: &Contexte,
        y: f32,
        demandes: &mut Vec<Demande>,
    ) -> f32 {
        let c = k.t.c;
        let (x, l) = (d.contenu.left(), d.contenu.width());
        let n = cx.projets.len();
        let sous = match (n, cx.dossier) {
            (_, None) => "Les projets ne sont pas enregistrés ici.".to_owned(),
            (0, _) => "Aucun projet enregistré pour l'instant.".to_owned(),
            (n, _) => format!(
                "{} · enregistrés automatiquement, une seconde après chaque modification.",
                pluriel(n, "projet")
            ),
        };
        // Les boutons d'en-tête, à droite du titre.
        let nouveau = "Nouveau projet";
        let l_nouveau = Self::largeur_bouton(k, nouveau, Some(Icone::Plus));
        let dossier = "Ouvrir le dossier";
        let l_dossier = Self::largeur_bouton(k, dossier, Some(Icone::Dossier));
        let mut xb = x + l - l_nouveau;
        if Self::bouton(
            k,
            Rect::from_min_size(Pos2::new(xb, y), Vec2::new(l_nouveau, 44.0)),
            "nouveau",
            1,
            nouveau,
            Some(Icone::Plus),
            Sorte::Principal,
            "Un plan vide, avec toutes tes briques",
        ) {
            demandes.push(Demande::Ouvrir(Ouverture::Nouveau));
        }
        if cx.dossier.is_some() {
            xb -= l_dossier + 10.0;
            if Self::bouton(
                k,
                Rect::from_min_size(Pos2::new(xb, y), Vec2::new(l_dossier, 44.0)),
                "ouvrir-dossier",
                0,
                dossier,
                Some(Icone::Dossier),
                Sorte::Secondaire,
                cx.dossier.unwrap_or_default(),
            ) {
                demandes.push(Demande::OuvrirDossier);
            }
        }
        let mut y = Self::titre(k, x, y, (xb - x - 16.0).max(200.0), "Mes projets", &sous);

        if n == 0 {
            let r = Rect::from_min_size(Pos2::new(x, y), Vec2::new(l, 120.0));
            k.t.p.rect(
                r,
                k.a.formes.rayon_vitrine,
                c.carte_2.alpha(0.6),
                Stroke::new(1.0, c.bord.c()),
                StrokeKind::Inside,
            );
            k.t.icone(
                Icone::Dossier,
                Pos2::new(r.center().x, r.top() + 40.0),
                26.0,
                c.doux.c(),
            );
            k.t.texte(
                Pos2::new(r.center().x, r.top() + 80.0),
                Align2::CENTER_CENTER,
                "Commence une quête ou un projet vide : il apparaîtra ici.",
                Police::Corps,
                k.a.typo.fort,
                c.doux.c(),
            );
            y = r.bottom() + 20.0;
        }

        let hr = 64.0;
        for (i, f) in cx.projets.iter().enumerate() {
            let r = Rect::from_min_size(Pos2::new(x, y), Vec2::new(l, hr));
            y += hr + 6.0;
            let courant = Some(f.fichier.as_str()) == cx.courant;
            if self.confirmation.as_deref() == Some(f.fichier.as_str()) {
                self.confirmer_suppression(k, r, f, demandes);
                continue;
            }
            k.reperer("projet", i, r);
            let touche = k.toucher(r, egui::Id::unique(("projet", i)), "", Sense::click());
            let rayon = k.a.formes.rayon_carte;
            k.t.p.rect(
                touche.rect,
                rayon,
                if courant {
                    c.carte.vers(c.accent, 0.10).c()
                } else {
                    c.carte.vers(c.survol, touche.chaud).c()
                },
                Stroke::new(
                    1.0,
                    if courant {
                        c.accent_clair.alpha(0.6)
                    } else {
                        c.bord.vers(c.bord_fort, touche.chaud).c()
                    },
                ),
                StrokeKind::Inside,
            );
            k.t.reflet(touche.rect, rayon);
            if courant {
                let barre = Rect::from_min_max(
                    Pos2::new(r.left() + 1.0, r.top() + 14.0),
                    Pos2::new(r.left() + 3.5, r.bottom() - 14.0),
                );
                k.t.p.rect_filled(barre, 1.25, c.accent_clair.c());
            }
            let tuile =
                Rect::from_center_size(Pos2::new(r.left() + 36.0, r.center().y), Vec2::splat(40.0));
            k.t.p.rect(
                tuile,
                k.a.formes.rayon_bouton + 2.0,
                c.enfonce.c(),
                Stroke::new(1.0, c.trait_.c()),
                StrokeKind::Inside,
            );
            let quete = f
                .quete
                .as_deref()
                .and_then(|q| PARCOURS.iter().find(|p| p.cle == q));
            k.t.icone(
                if f.illisible {
                    Icone::Verrou
                } else if quete.is_some() {
                    Icone::Quete
                } else {
                    Icone::Brique
                },
                tuile.center(),
                20.0,
                if f.illisible {
                    c.danger.c()
                } else {
                    c.accent_clair.c()
                },
            );
            // Les actions, à droite.
            let b = 32.0;
            let mut xa = r.right() - 14.0 - b;
            let actions: [(&'static str, Icone, &str); 3] = [
                ("supprimer", Icone::Poubelle, "Mettre à la corbeille"),
                ("dupliquer", Icone::Dupliquer, "Dupliquer"),
                ("renommer", Icone::Crayon, "Renommer"),
            ];
            for (nom, icone, bulle) in actions {
                let rect =
                    Rect::from_min_size(Pos2::new(xa, r.center().y - b / 2.0), Vec2::splat(b));
                xa -= b + 4.0;
                k.reperer(nom, i, rect);
                if k.bouton_icone(
                    rect,
                    &format!("{nom}-{}", f.fichier),
                    icone,
                    crate::panneaux::Style::Nu,
                    bulle,
                    !f.illisible || nom == "supprimer",
                ) {
                    match nom {
                        "supprimer" => self.confirmation = Some(f.fichier.clone()),
                        "dupliquer" => demandes.push(Demande::Dupliquer(f.fichier.clone())),
                        _ => {
                            self.renommage = Some(Renommage {
                                fichier: f.fichier.clone(),
                                texte: f.nom.clone(),
                                neuf: true,
                            });
                        }
                    }
                }
            }
            if let Some(quand) = f.modifie {
                k.t.texte(
                    Pos2::new(xa + b - 8.0, r.center().y),
                    Align2::RIGHT_CENTER,
                    &il_y_a(cx.maintenant, quand),
                    Police::Corps,
                    k.a.typo.petit,
                    c.discret.c(),
                );
            }
            let x_texte = tuile.right() + 16.0;
            let l_texte = (xa + b - 8.0 - 110.0 - x_texte).max(80.0);
            let en_saisie = self
                .renommage
                .as_ref()
                .is_some_and(|e| e.fichier == f.fichier);
            if en_saisie {
                let champ = Rect::from_min_size(
                    Pos2::new(x_texte - 8.0, r.top() + 10.0),
                    Vec2::new(l_texte.min(420.0), 30.0),
                );
                self.champ_renommer(k, champ, demandes);
            } else {
                // L'étiquette « En cours » suit le nom : sa place est réservée avant d'abréger.
                let l_tag = if courant {
                    k.t.largeur_legende("En cours") + 14.0
                } else {
                    0.0
                };
                let fin = k.t.texte(
                    Pos2::new(x_texte, r.center().y - 10.0),
                    Align2::LEFT_CENTER,
                    &k.t.abreger(
                        &f.nom,
                        Police::CorpsDemi,
                        k.a.typo.fort,
                        l_texte - l_tag - 12.0,
                    ),
                    Police::CorpsDemi,
                    k.a.typo.fort,
                    c.texte.c(),
                );
                if courant {
                    Self::etiquette(
                        &k.t,
                        Pos2::new(fin.right() + 10.0 + l_tag, r.center().y - 10.0),
                        "En cours",
                        c.accent_clair,
                    );
                }
            }
            let sous = if f.illisible {
                "Fichier illisible : il ne peut pas être ouvert.".to_owned()
            } else {
                let mut morceaux = vec![match quete {
                    Some(q) => format!("Quête : {}", q.titre),
                    None => "Projet libre".to_owned(),
                }];
                morceaux.push(if f.elements == 0 {
                    "plan vide".to_owned()
                } else {
                    pluriel(f.elements, "élément")
                });
                if f.briques > 0 {
                    morceaux.push(pluriel(f.briques, "brique"));
                }
                morceaux.join(" · ")
            };
            k.t.texte(
                Pos2::new(x_texte, r.center().y + 13.0),
                Align2::LEFT_CENTER,
                &k.t.abreger(&sous, Police::Corps, k.a.typo.petit, l_texte),
                Police::Corps,
                k.a.typo.petit,
                if f.illisible {
                    c.danger.c()
                } else {
                    c.doux.c()
                },
            );
            if touche.clic && !en_saisie && !f.illisible {
                demandes.push(if courant {
                    Demande::Continuer
                } else {
                    Demande::Ouvrir(Ouverture::Projet(f.fichier.clone()))
                });
            }
        }

        // Les exemples tout faits.
        y += 18.0;
        y += k.rubrique(x + 4.0, y, l - 8.0, "Exemples tout faits", "") + 4.0;
        for (i, e) in EXEMPLES.iter().enumerate() {
            let r = Rect::from_min_size(Pos2::new(x, y), Vec2::new(l, 60.0));
            y += 60.0 + 6.0;
            k.reperer("exemple", i, r);
            let touche = k.toucher(r, egui::Id::unique(("exemple", i)), "", Sense::click());
            k.t.element(touche.rect, k.a.formes.rayon_carte, touche.chaud, false);
            k.t.icone(
                Icone::Ecran,
                Pos2::new(r.left() + 36.0, r.center().y),
                20.0,
                c.un.c(),
            );
            k.t.texte(
                Pos2::new(r.left() + 73.0, r.center().y - 9.0),
                Align2::LEFT_CENTER,
                e.nom,
                Police::CorpsDemi,
                k.a.typo.fort,
                c.texte.c(),
            );
            k.t.texte(
                Pos2::new(r.left() + 73.0, r.center().y + 11.0),
                Align2::LEFT_CENTER,
                &k.t.abreger(e.resume, Police::Corps, k.a.typo.petit, l - 200.0),
                Police::Corps,
                k.a.typo.petit,
                c.doux.c(),
            );
            k.t.icone(
                Icone::Fleche,
                Pos2::new(r.right() - 30.0, r.center().y),
                18.0,
                c.doux.vers(c.texte, touche.chaud).c(),
            );
            if touche.clic {
                demandes.push(Demande::Ouvrir(Ouverture::Exemple(i)));
            }
        }
        y
    }

    /// La ligne d'un projet qu'on s'apprête à mettre à la corbeille.
    fn confirmer_suppression(
        &mut self,
        k: &mut Cadre,
        r: Rect,
        f: &FicheProjet,
        demandes: &mut Vec<Demande>,
    ) {
        let c = k.t.c;
        k.couvrir(r, egui::Id::unique(("confirmation", &f.fichier)));
        k.t.p.rect(
            r,
            k.a.formes.rayon_carte,
            c.danger.alpha(0.10),
            Stroke::new(1.0, c.danger.alpha(0.7)),
            StrokeKind::Inside,
        );
        k.t.icone(
            Icone::Poubelle,
            Pos2::new(r.left() + 36.0, r.center().y),
            20.0,
            c.danger.c(),
        );
        let garder = "Garder";
        let lg = Self::largeur_bouton(k, garder, None);
        let jeter = "Mettre à la corbeille";
        let lj = Self::largeur_bouton(k, jeter, None);
        let bj = Rect::from_min_size(
            Pos2::new(r.right() - 14.0 - lj, r.center().y - 17.0),
            Vec2::new(lj, 34.0),
        );
        let bg = Rect::from_min_size(
            Pos2::new(bj.left() - 8.0 - lg, r.center().y - 17.0),
            Vec2::new(lg, 34.0),
        );
        k.t.texte(
            Pos2::new(r.left() + 66.0, r.center().y),
            Align2::LEFT_CENTER,
            &k.t.abreger(
                &format!("Mettre « {} » à la corbeille ?", f.nom),
                Police::CorpsDemi,
                k.a.typo.fort,
                bg.left() - r.left() - 80.0,
            ),
            Police::CorpsDemi,
            k.a.typo.fort,
            c.texte.c(),
        );
        if Self::bouton(k, bg, "garder", 0, garder, None, Sorte::Secondaire, "") {
            self.confirmation = None;
        }
        if Self::bouton(
            k,
            bj,
            "confirmer",
            0,
            jeter,
            None,
            Sorte::Danger,
            "Il reste récupérable : « Annuler » dans le message qui suit",
        ) {
            self.confirmation = None;
            demandes.push(Demande::Jeter(f.fichier.clone()));
        }
    }

    /// Le champ où l'on écrit le nouveau nom d'un projet. Entrée valide, Échap abandonne.
    fn champ_renommer(&mut self, k: &mut Cadre, rect: Rect, demandes: &mut Vec<Demande>) {
        let c = k.t.c;
        let Some(edition) = self.renommage.as_mut() else {
            return;
        };
        k.reperer("champ-renommer", 0, rect);
        let cle = egui::Id::unique(("renommer", edition.fichier.as_str()));
        let rayon = k.a.formes.rayon_bouton;
        k.t.p.rect_stroke(
            rect.expand(2.0),
            rayon + 2.0,
            Stroke::new(2.0, c.accent_clair.alpha(0.35)),
            StrokeKind::Inside,
        );
        k.t.p.rect(
            rect,
            rayon,
            c.fond.c(),
            Stroke::new(1.0, c.accent_clair.c()),
            StrokeKind::Inside,
        );
        let reponse = k.ui.put(
            rect.shrink2(Vec2::new(8.0, 1.0)),
            egui::TextEdit::singleline(&mut edition.texte)
                .id(cle)
                .font(Police::CorpsDemi.taille(k.a.typo.fort * k.a.formes.echelle_texte))
                .text_color(c.texte.c())
                .frame(egui::Frame::NONE),
        );
        if edition.neuf {
            edition.neuf = false;
            reponse.request_focus();
            let fin = edition.texte.chars().count();
            let mut etat = egui::TextEdit::load_state(k.ui.ctx(), cle).unwrap_or_default();
            etat.cursor
                .set_char_range(Some(egui::text::CCursorRange::two(
                    egui::text::CCursor::new(0),
                    egui::text::CCursor::new(fin),
                )));
            etat.store(k.ui.ctx(), cle);
            return;
        }
        if reponse.lost_focus() {
            let abandon = k.ui.input(|i| i.key_pressed(Key::Escape));
            self.saisie_finie = true;
            if let Some(e) = self.renommage.take() {
                let nom = e.texte.trim();
                if !abandon && !nom.is_empty() {
                    demandes.push(Demande::Renommer {
                        fichier: e.fichier,
                        nom: nom.to_owned(),
                    });
                }
            }
        }
    }

    // --- Paramètres -------------------------------------------------------------------------

    fn page_parametres(
        &mut self,
        k: &mut Cadre,
        d: &DispositionAccueil,
        cx: &Contexte,
        y: f32,
        demandes: &mut Vec<Demande>,
    ) -> f32 {
        let c = k.t.c;
        let (x, l) = (d.contenu.left(), d.contenu.width());
        let g = k.a.formes.gouttiere + 6.0;
        let y = Self::titre(
            k,
            x,
            y,
            l.min(760.0),
            "Paramètres",
            "Chaque choix s'applique et s'enregistre tout de suite.",
        );
        let (l1, x2) = if d.large {
            ((l - g) / 2.0, x + (l - g) / 2.0 + g)
        } else {
            (l, x)
        };
        let r = cx.reglages;
        let mut nouveau = r.clone();
        let a = k.a;
        // Une ligne de réglage : un titre, une phrase, un choix. Hauteur fixe.
        const LIGNE: f32 = 98.0;
        let ligne = |k: &mut Cadre,
                     x: f32,
                     y: f32,
                     l: f32,
                     titre: &str,
                     phrase: &str,
                     repere: &'static str,
                     options: &[String],
                     choisi: usize|
         -> Option<usize> {
            k.t.texte(
                Pos2::new(x, y + 10.0),
                Align2::LEFT_CENTER,
                titre,
                Police::CorpsDemi,
                k.a.typo.fort,
                k.t.c.texte.c(),
            );
            k.t.texte(
                Pos2::new(x, y + 31.0),
                Align2::LEFT_CENTER,
                &k.t.abreger(phrase, Police::Corps, k.a.typo.petit, l),
                Police::Corps,
                k.a.typo.petit,
                k.t.c.doux.c(),
            );
            Self::segments(
                k,
                Rect::from_min_size(Pos2::new(x, y + 46.0), Vec2::new(l, 42.0)),
                repere,
                options,
                choisi,
            )
        };
        let section = |k: &mut Cadre, x: f32, y: f32, l: f32, h: f32, titre: &str| {
            let r = Rect::from_min_size(Pos2::new(x, y), Vec2::new(l, h));
            k.t.carte(r, k.a.formes.rayon_vitrine);
            k.rubrique(x + 24.0, y + 20.0, l - 48.0, titre, "");
        };

        // Colonne 1 : apparence, simulation, démarrage.
        let (ix, il) = (x + 24.0, l1 - 48.0);
        let mut y1 = y;
        section(k, x, y1, l1, 56.0 + 3.0 * LIGNE + 8.0, "Apparence");
        y1 += 56.0;
        let options = vec!["Sombre".to_owned(), "Clair".to_owned()];
        let choisi = usize::from(a.mode == Mode::Clair);
        if let Some(i) = ligne(
            k,
            ix,
            y1,
            il,
            "Thème",
            "Néon sur fond nuit, ou version claire.",
            "reglage-theme",
            &options,
            choisi,
        ) {
            nouveau.theme = Some(if i == 1 { Mode::Clair } else { Mode::Sombre });
        }
        y1 += LIGNE;
        let options: Vec<String> = TAILLES_TEXTE.iter().map(|(_, n)| (*n).to_owned()).collect();
        if let Some(i) = ligne(
            k,
            ix,
            y1,
            il,
            "Taille du texte",
            "Pour tous les panneaux et menus.",
            "reglage-texte",
            &options,
            r.rang_taille_texte(),
        ) {
            nouveau.taille_texte = TAILLES_TEXTE[i].0;
        }
        y1 += LIGNE;
        let options = vec!["Complètes".to_owned(), "Réduites".to_owned()];
        if let Some(i) = ligne(
            k,
            ix,
            y1,
            il,
            "Animations",
            "Réduites : sans transitions, perles de courant ni pulsations.",
            "reglage-animations",
            &options,
            usize::from(a.mouvement.sobre),
        ) {
            nouveau.animations_sobres = Some(i == 1);
        }
        y1 += LIGNE + 8.0 + g;

        section(
            k,
            x,
            y1,
            l1,
            56.0 + 2.0 * LIGNE + 8.0,
            "Simulation et démarrage",
        );
        y1 += 56.0;
        let options: Vec<String> = a
            .mouvement
            .vitesses
            .iter()
            .map(|d| libelle_pas_par_seconde(1.0 / d.max(0.001)))
            .collect();
        if !options.is_empty() {
            if let Some(i) = ligne(
                k,
                ix,
                y1,
                il,
                "Vitesse au départ",
                "Pas de simulation par seconde à l'ouverture d'un projet.",
                "reglage-vitesse",
                &options,
                a.mouvement
                    .vitesse_initiale
                    .min(options.len().saturating_sub(1)),
            ) {
                nouveau.vitesse = Some(i);
            }
        }
        y1 += LIGNE;
        let options = vec![
            "Montrer l'accueil".to_owned(),
            "Reprendre le projet".to_owned(),
        ];
        if let Some(i) = ligne(
            k,
            ix,
            y1,
            il,
            "Au lancement",
            "L'accueil, ou directement le dernier projet ouvert.",
            "reglage-lancement",
            &options,
            usize::from(r.au_lancement == Lancement::Reprendre),
        ) {
            nouveau.au_lancement = if i == 1 {
                Lancement::Reprendre
            } else {
                Lancement::Accueil
            };
        }
        y1 += LIGNE + 8.0 + g;

        // Les raccourcis du bureau et du menu Démarrer (Windows seulement).
        match cx.raccourcis {
            Some(Some(presents)) => {
                section(
                    k,
                    x,
                    y1,
                    l1,
                    56.0 + 2.0 * LIGNE + 8.0,
                    "Raccourcis pour lancer l'application",
                );
                y1 += 56.0;
                let options = vec!["Oui".to_owned(), "Non".to_owned()];
                for (lieu, titre, phrase, repere) in [
                    (
                        Lieu::Bureau,
                        "Sur le bureau",
                        "Une icône sur le bureau, qui suit les mises à jour.",
                        "reglage-raccourci-bureau",
                    ),
                    (
                        Lieu::MenuDemarrer,
                        "Dans le menu Démarrer",
                        "Pour la trouver en tapant son nom après la touche Windows.",
                        "reglage-raccourci-demarrer",
                    ),
                ] {
                    let present = presents[lieu.rang()];
                    if let Some(i) = ligne(
                        k,
                        ix,
                        y1,
                        il,
                        titre,
                        phrase,
                        repere,
                        &options,
                        usize::from(!present),
                    ) {
                        if (i == 0) != present {
                            demandes.push(Demande::Raccourci {
                                lieu,
                                poser: i == 0,
                            });
                        }
                    }
                    y1 += LIGNE;
                }
                y1 += 8.0 + g;
            }
            Some(None) => {
                let h_texte =
                    k.t.p
                        .layout(
                            crate::raccourci::MESSAGE_ARCHIVE.to_owned(),
                            Police::Corps.taille(13.0 * k.a.formes.echelle_texte),
                            c.doux.c(),
                            il,
                        )
                        .size()
                        .y;
                section(
                    k,
                    x,
                    y1,
                    l1,
                    56.0 + h_texte + 24.0,
                    "Raccourcis pour lancer l'application",
                );
                y1 += 56.0;
                y1 += k.t.paragraphe(
                    Pos2::new(ix, y1),
                    il,
                    crate::raccourci::MESSAGE_ARCHIVE,
                    Police::Corps,
                    13.0,
                    c.doux.c(),
                ) + 24.0
                    + g;
            }
            None => {}
        }

        // Colonne 2 : mises à jour, données, raccourcis.
        let (ix, il) = (x2 + 24.0, l1 - 48.0);
        let mut y2 = if d.large { y } else { y1 };
        section(k, x2, y2, l1, 56.0 + LIGNE + 72.0, "Mises à jour");
        y2 += 56.0;
        let options = vec!["Oui".to_owned(), "Non".to_owned()];
        if let Some(i) = ligne(
            k,
            ix,
            y2,
            il,
            "Chercher au lancement",
            "Les versions sont signées et vérifiées avant d'être installées.",
            "reglage-mises-a-jour",
            &options,
            usize::from(!r.mises_a_jour_automatiques),
        ) {
            nouveau.mises_a_jour_automatiques = i == 0;
        }
        y2 += LIGNE + 4.0;
        k.t.texte(
            Pos2::new(ix, y2 + 10.0),
            Align2::LEFT_CENTER,
            &format!("Version installée : {}", env!("CARGO_PKG_VERSION")),
            Police::CorpsDemi,
            k.a.typo.fort,
            c.texte.c(),
        );
        k.t.texte(
            Pos2::new(ix, y2 + 30.0),
            Align2::LEFT_CENTER,
            &k.t.abreger(cx.mise_a_jour, Police::Corps, k.a.typo.petit, il),
            Police::Corps,
            k.a.typo.petit,
            c.doux.c(),
        );
        let texte = "Chercher maintenant";
        let lb = Self::largeur_bouton(k, texte, None);
        if Self::bouton(
            k,
            Rect::from_min_size(Pos2::new(ix + il - lb, y2), Vec2::new(lb, 40.0)),
            "chercher",
            0,
            texte,
            None,
            Sorte::Secondaire,
            "",
        ) {
            demandes.push(Demande::ChercherMiseAJour);
        }
        y2 += 68.0 + g;

        let chemin = cx
            .dossier
            .unwrap_or("Aucun : rien n'est écrit sur le disque");
        let h_chemin =
            k.t.p
                .layout(
                    chemin.to_owned(),
                    Police::Mono.taille(k.a.typo.petit * k.a.formes.echelle_texte),
                    c.texte.c(),
                    il,
                )
                .size()
                .y;
        section(k, x2, y2, l1, 56.0 + 44.0 + h_chemin + 70.0, "Mes données");
        y2 += 56.0;
        y2 += k.t.paragraphe(
            Pos2::new(ix, y2),
            il,
            "Projets, avancement du parcours et réglages sont enregistrés automatiquement dans ce dossier :",
            Police::Corps,
            k.a.typo.corps,
            c.doux.c(),
        ) + 8.0;
        y2 += k.t.paragraphe(
            Pos2::new(ix, y2),
            il,
            chemin,
            Police::Mono,
            k.a.typo.petit,
            c.texte.c(),
        ) + 12.0;
        if cx.dossier.is_some() {
            let texte = "Ouvrir le dossier";
            let lb = Self::largeur_bouton(k, texte, Some(Icone::Dossier));
            if Self::bouton(
                k,
                Rect::from_min_size(Pos2::new(ix, y2), Vec2::new(lb, 40.0)),
                "ouvrir-dossier",
                1,
                texte,
                Some(Icone::Dossier),
                Sorte::Secondaire,
                "",
            ) {
                demandes.push(Demande::OuvrirDossier);
            }
        }
        // Recommencer le parcours : deux clics, pour ne jamais effacer par mégarde.
        let texte = if self.recommencer_arme {
            "Confirmer : tout recommencer"
        } else {
            "Recommencer le parcours"
        };
        let lb = Self::largeur_bouton(k, texte, None);
        if Self::bouton(
            k,
            Rect::from_min_size(Pos2::new(ix + il - lb, y2), Vec2::new(lb, 40.0)),
            "recommencer-parcours",
            0,
            texte,
            None,
            if self.recommencer_arme {
                Sorte::Danger
            } else {
                Sorte::Secondaire
            },
            "Efface les quêtes réussies et les briques gagnées. Une copie est gardée dans le dossier de tes données.",
        ) {
            if self.recommencer_arme {
                demandes.push(Demande::RecommencerParcours);
            }
            self.recommencer_arme = !self.recommencer_arme;
        }
        y2 += 40.0 + 22.0 + g;

        let raccourcis: [(&str, &str); 9] = [
            ("Espace", "Lecture ou pause"),
            ("Ctrl + Z / Ctrl + Y", "Annuler / rétablir"),
            ("Ctrl + A", "Tout sélectionner"),
            ("Ctrl + G", "Regrouper en brique"),
            ("Ctrl + D", "Dupliquer la sélection"),
            ("Suppr", "Supprimer la sélection"),
            ("F", "Recadrer le plan"),
            ("Échap", "Lâcher la sélection, sortir d'une brique"),
            ("Molette", "Zoomer autour du pointeur"),
        ];
        section(
            k,
            x2,
            y2,
            l1,
            56.0 + raccourcis.len() as f32 * 28.0 + 12.0,
            "Raccourcis clavier",
        );
        y2 += 56.0;
        for (touche, effet) in raccourcis {
            let lt = k.t.largeur(touche, Police::Mono, k.a.typo.petit) + 14.0;
            let r = Rect::from_min_size(Pos2::new(ix, y2), Vec2::new(lt, 22.0));
            k.t.p.rect(
                r,
                k.a.formes.rayon_petit,
                c.enfonce.c(),
                Stroke::new(1.0, c.bord_fort.c()),
                StrokeKind::Inside,
            );
            k.t.p.line_segment(
                [
                    Pos2::new(r.left() + 3.0, r.bottom() - 1.0),
                    Pos2::new(r.right() - 3.0, r.bottom() - 1.0),
                ],
                Stroke::new(1.0, c.bord_fort.c()),
            );
            k.t.texte(
                r.center(),
                Align2::CENTER_CENTER,
                touche,
                Police::Mono,
                k.a.typo.petit,
                c.texte.c(),
            );
            k.t.texte(
                Pos2::new(ix + 170.0, r.center().y),
                Align2::LEFT_CENTER,
                &k.t.abreger(effet, Police::Corps, k.a.typo.corps, il - 170.0),
                Police::Corps,
                k.a.typo.corps,
                c.doux.c(),
            );
            y2 += 28.0;
        }
        y2 += 12.0;

        if nouveau != *r {
            demandes.push(Demande::Reglages(nouveau));
        }
        y1.max(y2)
    }

    // --- Clavier ----------------------------------------------------------------------------

    fn clavier(&mut self, k: &mut Cadre, at: &Atelier, cx: &Contexte, demandes: &mut Vec<Demande>) {
        if self.renommage.is_some() || self.saisie_finie || k.ui.ctx().egui_wants_keyboard_input() {
            return;
        }
        let (entree, echap) =
            k.ui.input(|i| (i.key_pressed(Key::Enter), i.key_pressed(Key::Escape)));
        if echap {
            if self.confirmation.take().is_none() {
                if self.rubrique != Rubrique::Accueil {
                    self.aller(Rubrique::Accueil);
                } else if cx.courant.is_some() {
                    demandes.push(Demande::Continuer);
                }
            }
        } else if entree && self.rubrique == Rubrique::Accueil {
            if cx.courant.is_some() {
                demandes.push(Demande::Continuer);
            } else if let Some(f) = at.parcours.prochaine() {
                demandes.push(Demande::Ouvrir(Ouverture::Quete(f.cle.to_owned())));
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::Duration;

    #[test]
    fn les_durees_s_ecrivent_en_francais() {
        let t = SystemTime::UNIX_EPOCH + Duration::from_secs(10_000_000);
        let avant = |s: u64| t - Duration::from_secs(s);
        assert_eq!(il_y_a(t, avant(5)), "à l'instant");
        assert_eq!(il_y_a(t, avant(5 * 60)), "il y a 5 min");
        assert_eq!(il_y_a(t, avant(3 * 3600)), "il y a 3 h");
        assert_eq!(il_y_a(t, avant(30 * 3600)), "hier");
        assert_eq!(il_y_a(t, avant(4 * 86_400)), "il y a 4 jours");
        assert_eq!(il_y_a(t, avant(21 * 86_400)), "il y a 3 semaines");
        assert_eq!(il_y_a(t, avant(90 * 86_400)), "il y a 3 mois");
        // Une horloge qui recule ne fait pas paniquer.
        assert_eq!(il_y_a(avant(60), t), "à l'instant");
    }

    #[test]
    fn les_chapitres_couvrent_tout_le_parcours_dans_l_ordre() {
        let chap = chapitres();
        assert_eq!(
            chap.len(),
            crate::savoir::CHAPITRES.len(),
            "huit chapitres au parcours principal, un par branche"
        );
        let rangs: Vec<usize> = chap.iter().flat_map(|(_, q)| q.iter().copied()).collect();
        assert_eq!(rangs, (0..PARCOURS.len()).collect::<Vec<_>>());
        let noms: std::collections::HashSet<_> = chap.iter().map(|(n, _)| *n).collect();
        assert_eq!(noms.len(), chap.len(), "un chapitre n'apparaît qu'une fois");
    }
}
