//! Les panneaux posés au-dessus du plan : barre du haut, briques, quête, infos, chronogramme,
//! outils de vue, messages. Tout est peint à la main dans les rectangles fournis par la
//! [`Disposition`] ; les couleurs et dimensions viennent de l'[`Apparence`].

use egui::{Align2, CursorIcon, Key, Pos2, Rect, Sense, Stroke, StrokeKind, Vec2};

use crate::apparence::{Apparence, Courbe, Mode};
use crate::brique::groupable;
use crate::dessin::{Animations, Icone, Police, Toile};
use crate::disposition::{Contenus, Disposition, Ouvertures};
use crate::document::{Broche, Element, Fil, Genre, Id, Profondeur, Source, DEMI_PERIODE_DEFAUT};
use crate::economie::{self, Ecart, Origine};
use crate::etat::{Atelier, Etat, Proposition, Selection};
use crate::exemples::EXEMPLES;
use crate::logique::{self, Indice};
use crate::parcours::Parcours;
use crate::plan::Plan;
use crate::quete::{Quete, PARCOURS};
use crate::savoir;
use crate::simulation::{symbole, Lecture};

/// Un projet que les panneaux demandent d'ouvrir à la place de celui qui est affiché.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Ouverture {
    /// Un projet vide.
    Nouveau,
    /// Un des exemples tout faits, par son rang dans [`EXEMPLES`].
    Exemple(usize),
    /// Un projet enregistré, par son nom.
    Projet(String),
    /// Une quête du parcours, par sa clé, dans un projet neuf.
    Quete(String),
    /// La solution d'une quête, par sa clé : la plus petite construction connue, à regarder.
    Solution(String),
}

/// Ce que les panneaux demandent à l'application et qu'ils ne peuvent pas faire eux-mêmes.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Demandes {
    /// Passer du mode sombre au mode clair, ou l'inverse.
    pub changer_mode: bool,
    /// Ouvrir un autre projet.
    pub ouvrir: Option<Ouverture>,
    /// Une action choisie dans un message.
    pub action: Option<Proposition>,
    /// Revenir à l'accueil (clic sur le nom de l'application).
    pub accueil: bool,
}

/// Ce qu'on est en train d'écrire dans un champ.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
enum Champ {
    /// Le nom du projet.
    Projet,
    /// Le nom d'une brique.
    Modele(String),
    /// La description d'une brique.
    Description(String),
    /// Ce qu'on cherche parmi « Mes briques ».
    Filtre,
    /// Le nom d'un élément du plan.
    Nom(Id),
    /// Le nombre de bits d'un interrupteur ou d'une lampe.
    Largeur(Id),
    /// Les morceaux d'un découpeur ou les entrées d'une jonction.
    Morceaux(Id),
    /// Une durée : demi-période d'horloge, tenue d'une touche, impulsion de démarrage.
    Duree(Id),
}

#[derive(Clone, Debug)]
struct Edition {
    champ: Champ,
    texte: String,
    /// Vrai à la première image : le champ doit prendre le clavier.
    neuf: bool,
}

/// Ce qu'on sait d'une brique de la palette, calculé une fois par changement du projet.
#[derive(Clone, Debug)]
struct Resume {
    portes: u64,
    entrees: usize,
    sorties: usize,
    posable: bool,
}

/// Ce qu'on sait du plan affiché, calculé une fois par modification : sur un grand plan,
/// le refaire à chaque image coûterait plus cher que tout le reste.
#[derive(Clone, Debug)]
struct Circuit {
    portes: usize,
    signaux: usize,
    profondeur: Profondeur,
    niveaux: usize,
    /// Les premiers signaux et leur nom, pour les sondes et le chronogramme.
    sondes: Vec<(Source, String)>,
}

/// La place gardée à un texte qui change sans cesse (le temps qui défile) : elle ne fait que
/// grandir, pour que ce qui suit ne saute pas d'une image à l'autre.
#[derive(Clone, Copy, Debug, Default)]
struct Reserve {
    /// La largeur du texte témoin : si elle change (taille du texte), on repart d'elle.
    temoin: f32,
    largeur: f32,
}

impl Reserve {
    /// La place à donner au texte : au moins celle du témoin, et jamais moins qu'avant.
    fn garder(&mut self, temoin: f32, largeur: f32) -> f32 {
        if (self.temoin - temoin).abs() > 0.01 {
            *self = Reserve {
                temoin,
                largeur: temoin,
            };
        }
        self.largeur = self.largeur.max(largeur);
        self.largeur
    }
}

/// Ce qui fait changer l'écart du plan affiché : version du projet, lieu, version du parcours.
type CleDuLieu = (u64, Vec<String>, u64);

/// L'état propre aux panneaux : tailles mesurées à la dernière image, défilements, saisie.
#[derive(Clone, Debug, Default)]
pub struct Panneaux {
    /// Tailles qui dépendent du contenu, mesurées en dessinant.
    pub contenus: Contenus,
    /// La place du temps « t = … » dans la barre du haut.
    place_temps: Reserve,
    /// La place du texte de l'état (« Stable », « Passé, t = … ») dans la barre du haut.
    place_etat: Reserve,
    /// Défilement du panneau des infos, en points.
    pub defilement_infos: f32,
    /// Défilement du panneau des briques, en points.
    pub defilement_briques: f32,
    /// Défilement du panneau de la quête, en points.
    pub defilement_quete: f32,
    /// Vrai quand la fiche « Comprendre » de la quête est dépliée (elle est repliée au
    /// départ, pour que la table et Vérifier restent visibles).
    pub comprendre_ouvert: bool,
    /// Les indices de la quête affichée, calculés une fois, et combien de ses indices ont
    /// été révélés, quête par quête (pour la session).
    indices: Option<(&'static str, Vec<Indice>)>,
    indices_vus: std::collections::BTreeMap<&'static str, usize>,
    /// Ce qu'on cherche parmi « Mes briques » (nom ou description, sans accents).
    pub filtre_briques: String,
    /// Vrai quand « Exporter en image » vient d'être cliqué ; l'application s'en charge.
    pub photo_demandee: bool,
    /// Noms des projets enregistrés, fournis par l'application.
    pub projets: Vec<String>,
    edition: Option<Edition>,
    resumes: Option<(u64, Option<String>, Vec<Resume>)>,
    circuit: Option<(u64, u64, Circuit)>,
    reperes: Vec<(&'static str, usize, Rect)>,
    /// L'écart à la construction minimale du plan affiché hors quête, calculé une fois par
    /// version du projet, lieu et version du parcours.
    ecart_du_lieu: Option<(CleDuLieu, Option<Ecart>)>,
    /// Les étoiles des quêtes réussies, calculées une fois par version du parcours.
    etoiles: Option<(u64, Vec<Option<u8>>)>,
}

/// Une zone cliquable, à l'image où elle est dessinée.
pub(crate) struct Touche {
    /// Vrai si elle vient d'être cliquée.
    pub(crate) clic: bool,
    /// Survol lissé, de 0 à 1.
    pub(crate) chaud: f32,
    /// Son rectangle, un peu rétréci pendant l'appui.
    pub(crate) rect: Rect,
    /// La réponse d'egui.
    pub(crate) reponse: egui::Response,
}

/// De quoi dessiner et réagir pendant une image : partagé par les panneaux et l'accueil.
pub(crate) struct Cadre<'a, 'u> {
    pub(crate) ui: &'u mut egui::Ui,
    pub(crate) t: Toile<'a>,
    pub(crate) anims: &'u mut Animations,
    pub(crate) a: &'a Apparence,
    pub(crate) dt: f32,
    /// Rectangles qui recouvrent le plan : il ne doit pas réagir au pointeur dessous.
    pub(crate) obstacles: Vec<Rect>,
    /// Dans une zone qui défile : ce qui dépasse ne réagit pas au pointeur.
    pub(crate) decoupe: Option<Rect>,
    /// Où se trouvent les commandes dessinées à cette image, par nom et par rang.
    pub(crate) reperes: Vec<(&'static str, usize, Rect)>,
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub(crate) enum Style {
    /// Bouton sur fond secondaire, avec bordure (gardé pour les thèmes).
    #[allow(dead_code)]
    Carre,
    /// Bouton principal de la barre : fond de la couleur du 1.
    Plein,
    /// Bouton sans fond tant qu'il n'est pas survolé.
    Nu,
}

/// Un élément de la palette : son nom, son icône, son info-bulle, de quoi le fabriquer.
type Outil = (&'static str, Icone, &'static str, fn() -> Element);

/// Une ligne de la fiche de la sélection.
enum Rang {
    /// Un nom et un texte.
    Texte(String, String),
    /// Un nom, le nom d'un signal et ce qu'on y lit.
    Signal(String, String, Lecture),
    /// Un nom et un champ modifiable.
    Saisie(String, Champ, String, &'static str),
}

/// Une action proposée dans la barre de la sélection. Son rang sert de repère.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Action {
    /// Ouvrir la brique sélectionnée.
    Ouvrir,
    /// Regrouper la sélection en brique.
    Regrouper,
    /// Dupliquer la sélection.
    Dupliquer,
    /// Supprimer la sélection, ou débrancher le fil.
    Supprimer,
    /// Poser ou retirer le point d'arrêt sur le fil sélectionné.
    Arret,
    /// Retourner la sélection de gauche à droite.
    Retourner,
}

impl Cadre<'_, '_> {
    /// Note où se trouve une commande, si elle est visible (pas cachée par le défilement).
    pub(crate) fn reperer(&mut self, nom: &'static str, rang: usize, rect: Rect) {
        if self.decoupe.is_none_or(|z| z.contains(rect.center())) {
            self.reperes.push((nom, rang, rect));
        }
    }

    /// Déclare qu'un rectangle recouvre le plan : les clics et la molette s'y arrêtent.
    pub(crate) fn couvrir(&mut self, rect: Rect, cle: egui::Id) {
        self.ui.interact(rect, cle, Sense::click_and_drag());
        self.obstacles.push(rect);
    }

    /// Une zone cliquable : survol lissé, léger enfoncement à ressort, info-bulle.
    pub(crate) fn toucher(
        &mut self,
        rect: Rect,
        cle: egui::Id,
        bulle: &str,
        sens: Sense,
    ) -> Touche {
        // Dans une zone qui défile, seule la partie visible réagit.
        let visible = self.decoupe.map_or(rect, |d| rect.intersect(d));
        let (zone, sens) = if visible.is_positive() {
            (visible, sens)
        } else {
            (Rect::from_min_size(rect.min, Vec2::ZERO), Sense::hover())
        };
        let reponse = self.ui.interact(zone, cle, sens);
        let m = &self.a.mouvement;
        let chaud = self.anims.vers(
            cle.with("chaud"),
            if reponse.hovered() { 1.0 } else { 0.0 },
            self.dt,
            m.lissage_ui,
        );
        let appui = if reponse.is_pointer_button_down_on() {
            0.96
        } else {
            1.0
        };
        let echelle = self.anims.ressort(
            cle.with("appui"),
            appui,
            self.dt,
            m.raideur,
            m.amortissement,
        );
        if reponse.hovered() && sens != Sense::hover() {
            self.ui.ctx().set_cursor_icon(CursorIcon::PointingHand);
        }
        let clic = reponse.clicked();
        // Un clic laisse une onde autour de la zone : on voit que le geste a été pris.
        let cle_eclair = cle.with("eclair");
        if clic && m.duree(m.eclair_clic) > 0.0 {
            self.anims.poser(cle_eclair, 1.0);
        }
        let eclair = self
            .anims
            .vers_si_connu(cle_eclair, 0.0, self.dt, m.eclair_clic / 3.0);
        if eclair > 0.01 {
            let rayon = self.a.formes.rayon_bouton.min(rect.height() * 0.4);
            let marge = 2.0 + 8.0 * (1.0 - eclair);
            self.t.p.rect_stroke(
                rect.expand(marge),
                rayon + marge,
                Stroke::new(1.5, self.t.c.accent_clair.alpha(0.75 * eclair)),
                StrokeKind::Outside,
            );
        }
        let reponse = if bulle.is_empty() {
            reponse
        } else {
            reponse.on_hover_text(bulle)
        };
        Touche {
            clic,
            chaud,
            rect: Rect::from_center_size(rect.center(), rect.size() * echelle),
            reponse,
        }
    }

    pub(crate) fn bouton_icone(
        &mut self,
        rect: Rect,
        cle: &str,
        icone: Icone,
        style: Style,
        bulle: &str,
        actif: bool,
    ) -> bool {
        let touche = self.toucher(
            rect,
            egui::Id::unique(cle),
            bulle,
            if actif {
                Sense::click()
            } else {
                Sense::hover()
            },
        );
        let c = self.t.c;
        let rayon = self.a.formes.rayon_bouton.min(rect.height() * 0.32);
        let chaud = if actif { touche.chaud } else { 0.0 };
        let couleur = match style {
            Style::Plein => {
                self.t
                    .p
                    .rect_filled(touche.rect, rayon, c.un.vers(c.perle, chaud * 0.2).c());
                c.sur_un.c()
            }
            Style::Carre => {
                self.t.element(touche.rect, rayon, chaud, false);
                c.doux.vers(c.texte, 0.5 + 0.5 * chaud).c()
            }
            Style::Nu => {
                self.t
                    .p
                    .rect_filled(touche.rect, rayon, c.survol.alpha(chaud));
                c.doux.vers(c.texte, chaud).c()
            }
        };
        let couleur = if actif { couleur } else { c.discret.alpha(0.5) };
        // L'icône garde la même taille optique dans tous les boutons : 16 dans un bouton de
        // 28, 18 dans un bouton de 32.
        let taille = (rect.height() * 0.56).clamp(13.0, 20.0);
        self.t.icone(
            icone,
            touche.rect.center(),
            taille * touche.rect.width() / rect.width(),
            couleur,
        );
        touche.clic && actif
    }

    /// Titre de panneau avec son bouton de fermeture. Renvoie `true` si on ferme.
    /// En-tête d'un panneau : titre demi-gras, bouton pour cacher, trait de séparation.
    fn entete(&mut self, rect: Rect, titre: &str, cle: &str) -> bool {
        let y = rect.top() + ENTETE / 2.0;
        self.t.texte(
            Pos2::new(rect.left() + self.a.formes.rembourrage + 2.0, y),
            Align2::LEFT_CENTER,
            titre,
            Police::CorpsDemi,
            self.a.typo.fort,
            self.t.c.texte.c(),
        );
        self.fermer(rect, cle)
    }

    /// Le bouton qui cache un panneau, et le trait qui sépare l'en-tête du contenu.
    fn fermer(&mut self, rect: Rect, cle: &str) -> bool {
        self.t
            .separateur(rect.left() + 1.0, rect.right() - 1.0, rect.top() + ENTETE);
        let r = Rect::from_center_size(
            Pos2::new(rect.right() - ENTETE / 2.0, rect.top() + ENTETE / 2.0),
            Vec2::splat(26.0),
        );
        self.bouton_icone(
            r,
            cle,
            Icone::Fermer,
            Style::Nu,
            tr!("Cacher ce panneau"),
            true,
        )
    }

    /// Petit titre en capitales, avec un texte optionnel à droite. Renvoie la hauteur occupée.
    pub(crate) fn rubrique(&self, x: f32, y: f32, largeur: f32, titre: &str, droite: &str) -> f32 {
        let c = self.t.c;
        let l_droite = if droite.is_empty() {
            0.0
        } else {
            self.t.largeur(droite, Police::Mono, self.a.typo.legende) + 12.0
        };
        self.t.legende(
            Pos2::new(x, y + 8.0),
            Align2::LEFT_CENTER,
            &self
                .t
                .abreger_legende(titre, (largeur - l_droite).max(20.0)),
            c.discret.c(),
        );
        if !droite.is_empty() {
            self.t.texte(
                Pos2::new(x + largeur, y + 8.0),
                Align2::RIGHT_CENTER,
                droite,
                Police::Mono,
                self.a.typo.legende,
                c.discret.c(),
            );
        }
        24.0
    }

    /// Une puce « nom + valeur » ou « texte ». Avance le curseur, passe à la ligne si besoin.
    fn puce(
        &self,
        curseur: &mut Pos2,
        gauche: f32,
        droite: f32,
        gras: &str,
        texte: &str,
        valeur: Option<char>,
    ) {
        let c = self.t.c;
        let ty = self.t.ty().clone();
        let l_gras = if gras.is_empty() {
            0.0
        } else {
            self.t.largeur(gras, Police::MonoGras, ty.corps) + 5.0
        };
        let l_texte = if texte.is_empty() {
            0.0
        } else if valeur.is_some() {
            self.t.largeur(texte, Police::Mono, ty.petit) + 6.0
        } else {
            self.t.largeur(texte, Police::Corps, ty.petit)
        };
        let l_valeur = if valeur.is_some() { 26.0 } else { 0.0 };
        let largeur =
            10.0 + l_gras + l_texte + l_valeur + 10.0 - if valeur.is_some() { 6.0 } else { 0.0 };
        if curseur.x + largeur > droite && curseur.x > gauche {
            *curseur = Pos2::new(gauche, curseur.y + 32.0);
        }
        let r = Rect::from_min_size(*curseur, Vec2::new(largeur, 26.0));
        self.t.p.rect(
            r,
            self.a.formes.rayon_petit + 2.0,
            c.carte_2.c(),
            Stroke::new(1.0, c.trait_.c()),
            StrokeKind::Inside,
        );
        let mut x = r.left() + 10.0;
        if !gras.is_empty() {
            self.t.texte(
                Pos2::new(x, r.center().y),
                Align2::LEFT_CENTER,
                gras,
                Police::MonoGras,
                ty.corps,
                c.texte.c(),
            );
            x += l_gras;
        }
        if !texte.is_empty() {
            let police = if valeur.is_some() {
                Police::Mono
            } else {
                Police::Corps
            };
            self.t.texte(
                Pos2::new(x, r.center().y),
                Align2::LEFT_CENTER,
                texte,
                police,
                ty.petit,
                if valeur.is_some() {
                    c.texte.c()
                } else {
                    c.doux.c()
                },
            );
            x += l_texte;
        }
        if let Some(v) = valeur {
            self.t.pastille(Pos2::new(x + 13.0, r.center().y), v, 18.0);
        }
        curseur.x += largeur + 6.0;
    }

    /// Un bouton large, de la couleur d'accent, avec son texte au milieu.
    pub(crate) fn bouton_texte(&mut self, rect: Rect, cle: &str, texte: &str, actif: bool) -> bool {
        let touche = self.toucher(
            rect,
            egui::Id::unique(cle),
            "",
            if actif {
                Sense::click()
            } else {
                Sense::hover()
            },
        );
        let c = self.t.c;
        let chaud = if actif { touche.chaud } else { 0.0 };
        let rayon = self.a.formes.rayon_bouton;
        if actif {
            // Action principale : la seule surface pleine d'accent de la vue.
            self.t.p.rect_filled(
                touche.rect,
                rayon,
                c.accent.vers(c.accent_clair, chaud * 0.18).c(),
            );
            self.t.reflet(touche.rect, rayon);
        } else {
            self.t.element(touche.rect, rayon, 0.0, false);
        }
        self.t.texte(
            touche.rect.center(),
            Align2::CENTER_CENTER,
            texte,
            Police::CorpsDemi,
            self.a.typo.corps,
            if actif {
                c.sur_accent.c()
            } else {
                c.discret.c()
            },
        );
        touche.clic && actif
    }

    /// Un bouton secondaire : surface d'élément, icône et texte, sans couleur d'accent.
    pub(crate) fn bouton_second(
        &mut self,
        rect: Rect,
        cle: &str,
        icone: Icone,
        texte: &str,
        actif: bool,
    ) -> bool {
        let touche = self.toucher(
            rect,
            egui::Id::unique(cle),
            "",
            if actif {
                Sense::click()
            } else {
                Sense::hover()
            },
        );
        let c = self.t.c;
        let chaud = if actif { touche.chaud } else { 0.0 };
        self.t
            .element(touche.rect, self.a.formes.rayon_bouton, chaud, false);
        let encre = if actif {
            c.texte.c()
        } else {
            c.discret.alpha(0.8)
        };
        let ty = &self.a.typo;
        let texte = &self.t.abreger(
            texte,
            Police::CorpsDemi,
            ty.corps,
            touche.rect.width() - 24.0 - 24.0,
        );
        let l = 16.0 + 8.0 + self.t.largeur(texte, Police::CorpsDemi, ty.corps);
        let x = touche.rect.center().x - l / 2.0;
        self.t.icone(
            icone,
            Pos2::new(x + 8.0, touche.rect.center().y),
            15.0,
            if actif { c.doux.c() } else { encre },
        );
        self.t.texte(
            Pos2::new(x + 24.0, touche.rect.center().y),
            Align2::LEFT_CENTER,
            texte,
            Police::CorpsDemi,
            ty.corps,
            encre,
        );
        touche.clic && actif
    }

    /// Le fond d'une ligne de liste : rien au repos, une surface au survol ; la ligne choisie
    /// est enfoncée, marquée d'un trait d'accent à gauche.
    pub(crate) fn ligne_de_liste(&self, rect: Rect, chaud: f32, choisie: bool) {
        let c = self.t.c;
        let rayon = self.a.formes.rayon_bouton;
        if choisie {
            self.t.p.rect_filled(rect, rayon, c.enfonce.c());
            self.t.p.rect_filled(
                Rect::from_min_size(
                    Pos2::new(rect.left(), rect.top() + 9.0),
                    Vec2::new(2.5, rect.height() - 18.0),
                ),
                1.25,
                c.accent_clair.c(),
            );
        } else {
            self.t.p.rect_filled(rect, rayon, c.survol.alpha(chaud));
        }
    }

    /// Fait défiler à la molette le contenu d'un panneau, haut de `hauteur` dans `zone`, et
    /// dessine le repère de défilement. Renvoie le nouveau défilement.
    pub(crate) fn defiler(
        &mut self,
        panneau: Rect,
        zone: Rect,
        hauteur: f32,
        defilement: f32,
        ouverture: f32,
    ) -> f32 {
        let depasse = (hauteur - zone.height()).max(0.0);
        let dessus = self
            .ui
            .input(|i| i.pointer.latest_pos())
            .is_some_and(|p| panneau.contains(p));
        let mut d = defilement;
        if dessus && ouverture > 0.5 {
            d -= self.ui.input(|i| i.smooth_scroll_delta.y);
        }
        let d = d.clamp(0.0, depasse);
        if depasse > 0.5 {
            // Repère discret sur le bord droit.
            let part = zone.height() / hauteur;
            let piste = Rect::from_min_max(
                Pos2::new(panneau.right() - 9.0, zone.top() + 4.0),
                Pos2::new(panneau.right() - 5.0, zone.bottom() - 4.0),
            );
            let haut = piste.top() + (d / hauteur) * piste.height();
            self.t.p.rect_filled(
                Rect::from_min_max(
                    Pos2::new(piste.left(), haut),
                    Pos2::new(piste.right(), haut + part * piste.height()),
                ),
                2.0,
                self.t.c.doux.alpha(0.45),
            );
        }
        d
    }
}

impl Panneaux {
    /// Dessine tous les panneaux et traite leurs clics.
    #[allow(clippy::too_many_arguments)]
    pub fn montrer(
        &mut self,
        ui: &mut egui::Ui,
        a: &Apparence,
        at: &mut Atelier,
        plan: &mut Plan,
        anims: &mut Animations,
        d: &Disposition,
        o: Ouvertures,
        dt: f32,
    ) -> Demandes {
        let mut demandes = Demandes::default();
        let base = Toile::nouvelle(ui.painter().with_clip_rect(d.ecran), a);
        let mut k = Cadre {
            ui,
            t: base,
            anims,
            a,
            dt,
            obstacles: Vec::new(),
            decoupe: None,
            reperes: Vec::new(),
        };

        let sous_le_fil = self.fil_d_ariane(&mut k, at, d);
        Self::messages(
            &mut k,
            at,
            d.libre,
            sous_le_fil,
            false,
            &mut demandes.action,
        );
        self.barre_de_selection(&mut k, at, d);
        self.outils(&mut k, at, plan, d);
        self.temps(&mut k, at, d, o.temps);
        self.briques(&mut k, at, plan, d, o.briques, &mut demandes);
        self.quete(&mut k, at, d, o.quete, &mut demandes);
        self.infos(&mut k, at, d, o.infos);
        self.marque(&mut k, d, &mut demandes);
        self.transport(&mut k, at, d);
        self.volets(&mut k, at, d, &mut demandes);
        plan.obstacles = k.obstacles;
        self.reperes = k.reperes;
        demandes
    }

    /// Où se trouve, à l'écran, une commande dessinée à la dernière image : `"nand"`,
    /// `"outil"`, `"modele"`, `"ouvrir-modele"`, `"enregistrer-brique"`, `"projet"`,
    /// `"ariane"`, `"action"`, `"champ"`, chacune avec son rang. Sert aux essais automatiques
    /// et aux aides qui montrent où cliquer.
    pub fn repere(&self, nom: &str, rang: usize) -> Option<Rect> {
        self.reperes
            .iter()
            .find(|(n, r, _)| *n == nom && *r == rang)
            .map(|(.., rect)| *rect)
    }

    /// Vrai si un champ de texte est en cours de saisie : les raccourcis clavier attendent.
    pub fn en_saisie(&self) -> bool {
        self.edition.is_some()
    }

    /// Abandonne la saisie en cours, s'il y en a une (on quitte l'atelier).
    pub fn abandonner_saisie(&mut self) {
        self.edition = None;
    }

    /// Un autre projet est ouvert : la barre du haut retrouve ses largeurs de départ.
    pub fn oublier_les_places(&mut self) {
        self.place_temps = Reserve::default();
        self.place_etat = Reserve::default();
    }

    /// Un champ de texte : il affiche `valeur`, un clic permet de la changer. Renvoie le texte
    /// validé par Entrée ou par un clic ailleurs ; Échap abandonne.
    #[allow(clippy::too_many_arguments)]
    fn champ(
        &mut self,
        k: &mut Cadre,
        rect: Rect,
        champ: Champ,
        valeur: &str,
        police: Police,
        taille: f32,
        bulle: &str,
    ) -> Option<String> {
        let c = k.t.c;
        let cle = egui::Id::unique(("champ", &champ));
        let rang = k.reperes.iter().filter(|(n, ..)| *n == "champ").count();
        k.reperes.push(("champ", rang, rect));
        if let Some(edition) = self.edition.as_mut().filter(|e| e.champ == champ) {
            if k.decoupe.is_some_and(|d| !d.contains_rect(rect)) {
                // Le champ a défilé hors de vue : la saisie est abandonnée.
                self.edition = None;
                return None;
            }
            // En saisie : fond creusé et anneau de focus d'accent.
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
                    .font(police.taille(taille * k.a.formes.echelle_texte))
                    .text_color(c.texte.c())
                    .frame(egui::Frame::NONE),
            );
            if edition.neuf {
                // Le champ prend le clavier, tout son texte sélectionné : taper le remplace.
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
                return None;
            }
            if reponse.lost_focus() {
                let abandon = k.ui.input(|i| i.key_pressed(Key::Escape));
                let texte = self.edition.take().map(|e| e.texte);
                return texte.filter(|t| !abandon && t.trim() != valeur);
            }
            return None;
        }
        let touche = k.toucher(rect, cle.with("lire"), bulle, Sense::click());
        if touche.reponse.hovered() {
            k.ui.ctx().set_cursor_icon(CursorIcon::Text);
        }
        k.t.p.rect(
            rect,
            k.a.formes.rayon_bouton,
            c.fond.alpha(0.6).lerp_to_gamma(c.survol.c(), touche.chaud),
            Stroke::new(1.0, c.bord.vers(c.bord_fort, touche.chaud).c()),
            StrokeKind::Inside,
        );
        k.t.texte(
            Pos2::new(rect.left() + 9.0, rect.center().y),
            Align2::LEFT_CENTER,
            &k.t.abreger(valeur, police, taille, rect.width() - 18.0),
            police,
            taille,
            c.texte.c(),
        );
        if touche.clic {
            self.edition = Some(Edition {
                champ,
                texte: valeur.to_owned(),
                neuf: true,
            });
        }
        None
    }

    /// Applique ce qui vient d'être écrit dans un champ.
    fn appliquer(at: &mut Atelier, a: &Apparence, champ: Champ, texte: &str) {
        let texte = texte.trim();
        match champ {
            Champ::Projet => {
                at.renommer_projet(texte);
            }
            Champ::Modele(ancien) => {
                // Le nom montré (traduit) qu'on n'a pas changé ne renomme rien.
                if crate::langue::nom_affiche(&ancien) != texte {
                    at.renommer_modele(&ancien, texte);
                }
            }
            Champ::Description(modele) => at.decrire_modele(&modele, texte),
            // Le filtre est rangé par celui qui dessine le champ.
            Champ::Filtre => {}
            Champ::Nom(id) => {
                let inchange = at
                    .doc
                    .element(id)
                    .is_some_and(|e| crate::langue::nom_affiche(&e.nom) == texte);
                if !inchange {
                    at.renommer_element(id, texte, a);
                }
            }
            Champ::Largeur(id) => match texte.parse() {
                Ok(n) => {
                    at.changer_largeur(id, n, a);
                }
                Err(_) => at.dire(tr!("Écris un nombre de bits, de 1 à 64.")),
            },
            Champ::Morceaux(id) => match at.doc.element(id).map(|e| e.genre) {
                Some(Genre::Decoupe) => {
                    at.regler_decoupe(id, texte, a);
                }
                Some(Genre::Jonction) => {
                    at.regler_jonction(id, texte, a);
                }
                _ => {}
            },
            Champ::Duree(id) => match texte.parse() {
                Ok(n) => {
                    at.changer_duree(id, n, a);
                }
                Err(_) => at.dire(tr!("Écris une durée en pas de simulation, par exemple 8.")),
            },
        }
    }

    /// Ce qu'il faut savoir de chaque brique pour la palette, recalculé seulement quand le
    /// projet ou le plan affiché change.
    fn resumer(&mut self, at: &Atelier) {
        let lieu = at.lieu().map(str::to_owned);
        if self.resumes.as_ref().is_some_and(|(r, l, v)| {
            *r == at.projet.revision && *l == lieu && v.len() == at.projet.modeles.len()
        }) {
            return;
        }
        let resumes = at
            .projet
            .modeles
            .iter()
            .map(|m| {
                let (e, s) = m.plan.interface();
                Resume {
                    portes: at.projet.nb_portes(&m.plan),
                    entrees: e.len(),
                    sorties: s.len(),
                    posable: at.projet.peut_poser(lieu.as_deref(), &m.nom),
                }
            })
            .collect();
        self.resumes = Some((at.projet.revision, lieu, resumes));
    }

    /// Ce qu'on sait du plan affiché, refait seulement quand il a changé.
    fn connaitre(&mut self, at: &Atelier) -> &Circuit {
        let a_jour = self
            .circuit
            .as_ref()
            .is_some_and(|(v, d, _)| *v == at.version && *d == at.deplacements);
        if !a_jour {
            let profondeur = at.doc.chemin_critique();
            let circuit = Circuit {
                portes: at.doc.portes().count(),
                signaux: at.doc.prises().count(),
                profondeur,
                niveaux: match profondeur {
                    Profondeur::Briques => at.projet.profondeur(&at.doc) as usize,
                    _ => 0,
                },
                sondes: at
                    .doc
                    .prises()
                    .take(SONDES + 1)
                    .map(|s| (s, at.doc.nom_signal(s)))
                    .collect(),
            };
            self.circuit = Some((at.version, at.deplacements, circuit));
        }
        &self.circuit.as_ref().expect("il vient d'être rempli").2
    }

    /// Le chemin des briques ouvertes, en haut à gauche du plan : un clic remonte. Renvoie
    /// la hauteur qu'il occupe.
    fn fil_d_ariane(&mut self, k: &mut Cadre, at: &mut Atelier, d: &Disposition) -> f32 {
        let presence = k.anims.vers(
            egui::Id::unique("fil-d-ariane"),
            if at.chemin.is_empty() { 0.0 } else { 1.0 },
            k.dt,
            k.a.mouvement.lissage_ui,
        );
        if at.chemin.is_empty() {
            return 0.0;
        }
        let c = k.t.c;
        let hauteur = 40.0;
        let noms: Vec<String> = std::iter::once(at.projet.nom.clone())
            .chain(at.etapes_du_chemin())
            .map(|n| {
                k.t.abreger(
                    &crate::langue::nom_affiche(&n),
                    Police::CorpsDemi,
                    k.a.typo.corps,
                    190.0,
                )
            })
            .collect();
        let largeurs: Vec<f32> = noms
            .iter()
            .map(|n| k.t.largeur(n, Police::CorpsDemi, k.a.typo.corps) + 20.0)
            .collect();
        let total = 6.0 + largeurs.iter().sum::<f32>() + 16.0 * (noms.len() - 1) as f32 + 6.0;
        let r = Rect::from_min_size(
            Pos2::new(d.libre.left(), d.libre.top() - (1.0 - presence) * 10.0),
            Vec2::new(total.min(d.libre.width()), hauteur),
        );
        k.couvrir(r, egui::Id::unique("ariane"));
        let voile = k.t.sur(presence, r.expand(40.0));
        let fond = std::mem::replace(&mut k.t, voile);
        k.t.carte(r, hauteur / 2.0);
        let dedans = k.t.sur(1.0, r.shrink(2.0));
        let dessus = std::mem::replace(&mut k.t, dedans);
        k.decoupe = Some(r);
        let mut x = r.left() + 6.0;
        let mut ou = None;
        let dernier = noms.len() - 1;
        for (i, (nom, l)) in noms.iter().zip(&largeurs).enumerate() {
            let rect = Rect::from_min_size(Pos2::new(x, r.center().y - 15.0), Vec2::new(*l, 30.0));
            let ici = i == dernier;
            k.reperes.push(("ariane", i, rect));
            let touche = k.toucher(
                rect,
                egui::Id::unique(("ariane", i)),
                if ici { "" } else { tr!("Revenir à ce plan") },
                if ici { Sense::hover() } else { Sense::click() },
            );
            let chaud = if ici { 0.0 } else { touche.chaud };
            k.t.p.rect_filled(touche.rect, 10.0, c.carte_2.alpha(chaud));
            k.t.texte(
                touche.rect.center(),
                Align2::CENTER_CENTER,
                nom,
                Police::CorpsDemi,
                k.a.typo.corps,
                if ici {
                    c.texte.c()
                } else {
                    c.doux.vers(c.accent_clair, chaud).c()
                },
            );
            if touche.clic {
                ou = Some(i);
            }
            x += l;
            if !ici {
                k.t.icone(
                    Icone::Chevron,
                    Pos2::new(x + 8.0, r.center().y),
                    12.0,
                    c.doux.alpha(0.7),
                );
                x += 16.0;
            }
        }
        k.decoupe = None;
        k.t = dessus;
        k.t = fond;
        if let Some(niveau) = ou {
            at.remonter(niveau);
        }
        (hauteur + 8.0) * presence
    }

    /// Prépare un panneau : bloque le plan dessous, dessine la carte, renvoie la toile découpée.
    fn panneau<'a>(
        k: &mut Cadre<'a, '_>,
        rect: Rect,
        cle: &str,
        ouverture: f32,
    ) -> Option<Toile<'a>> {
        if ouverture < 0.01 {
            return None;
        }
        if ouverture > 0.5 {
            k.couvrir(rect, egui::Id::unique(("panneau", cle)));
        }
        let voile = k.t.sur(ouverture, k.t.p.clip_rect());
        voile.carte(rect, k.a.formes.rayon_carte);
        Some(k.t.sur(ouverture, rect.shrink(1.0)))
    }

    fn marque(&mut self, k: &mut Cadre, d: &Disposition, demandes: &mut Demandes) {
        let r = d.marque;
        k.couvrir(r, egui::Id::unique("marque"));
        k.reperer("marque", 0, r);
        let touche = k.toucher(
            r,
            egui::Id::unique("marque-clic"),
            tr!("Accueil : projets, parcours des quêtes, paramètres"),
            Sense::click(),
        );
        if touche.clic {
            demandes.accueil = true;
        }
        k.t.carte(r, k.a.formes.rayon_carte);
        let c = k.t.c;
        if touche.chaud > 0.01 {
            k.t.p.rect_filled(
                r.shrink(4.0),
                k.a.formes.rayon_carte - 3.0,
                c.survol.alpha(touche.chaud),
            );
        }
        let logo = Rect::from_min_size(
            Pos2::new(r.left() + 12.0, r.center().y - 14.0),
            Vec2::splat(28.0),
        );
        k.t.p.rect_filled(logo, 8.0, c.un.c());
        k.t.reflet(logo, 8.0);
        k.t.icone(
            Icone::Nand,
            logo.center() + Vec2::new(0.5, 0.0),
            17.0,
            c.sur_un.c(),
        );
        let x = logo.right() + 10.0;
        let titre = tr!("Computer Engineer");
        // Fenêtre très étroite : le logo seul, suivi du chemin vers l'accueil.
        let fin = if d.etroit {
            x - 17.0
        } else {
            k.t.texte(
                Pos2::new(x, r.center().y - 7.5),
                Align2::LEFT_CENTER,
                titre,
                Police::CorpsDemi,
                k.a.typo.corps,
                c.texte.c(),
            );
            k.t.texte(
                Pos2::new(x, r.center().y + 8.5),
                Align2::LEFT_CENTER,
                concat!("version ", env!("CARGO_PKG_VERSION")),
                Police::Mono,
                k.a.typo.legende,
                c.discret.c(),
            );
            x + k.t.largeur(titre, Police::CorpsDemi, k.a.typo.corps)
        };
        // Le chemin vers l'accueil, à droite du nom.
        k.t.icone(
            Icone::Maison,
            Pos2::new(fin + 24.0, r.center().y),
            16.0,
            c.discret.vers(c.texte, touche.chaud).c(),
        );
        self.contenus.marque =
            x + k.t.largeur(titre, Police::CorpsDemi, k.a.typo.corps) - r.left() + 50.0;
        self.contenus.marque_logo = x - 17.0 - r.left() + 50.0;
    }

    fn transport(&mut self, k: &mut Cadre, at: &mut Atelier, d: &Disposition) {
        let r = d.transport;
        k.couvrir(r, egui::Id::unique("transport"));
        k.t.carte(r, k.a.formes.rayon_carte);
        let c = k.t.c;
        let b = k.a.formes.bouton;
        let mut x = r.left() + 8.0;
        let carre = |x: &mut f32| {
            let rect = Rect::from_min_size(Pos2::new(*x, r.center().y - b / 2.0), Vec2::splat(b));
            *x += b + 4.0;
            rect
        };
        let rect = carre(&mut x);
        k.reperer("zero", 0, rect);
        if k.bouton_icone(
            rect,
            "zero",
            Icone::Zero,
            Style::Nu,
            tr!("Revenir à t = 0"),
            true,
        ) {
            at.zero();
        }
        let rect = carre(&mut x);
        k.reperer("pas", 0, rect);
        if k.bouton_icone(
            rect,
            "pas",
            Icone::Pas,
            Style::Nu,
            tr!("Avancer d'un pas"),
            true,
        ) {
            at.pas();
        }
        let en_lecture = at.lecture && at.vue_t.is_none();
        let rect = carre(&mut x);
        k.reperer("jouer", 0, rect);
        if k.bouton_icone(
            rect,
            "jouer",
            if en_lecture {
                Icone::Pause
            } else {
                Icone::Lecture
            },
            Style::Plein,
            tr!("Lecture ou pause (Espace)"),
            true,
        ) {
            at.lecture_ou_pause();
        }
        x += 10.0;
        // Le temps en chiffres à chasse fixe : ils ne dansent pas quand il défile.
        let ty = k.a.typo.clone();
        let temps = format!("t = {}", at.t_affiche());
        k.t.texte(
            Pos2::new(x, r.center().y),
            Align2::LEFT_CENTER,
            &temps,
            Police::MonoGras,
            ty.fort,
            c.texte.c(),
        );
        // Six chiffres d'avance, et une place qui ne rétrécit pas : ni le temps qui défile ni
        // le retour à t = 0 ne déplacent les boutons.
        x += self.place_temps.garder(
            k.t.largeur("t = 000000", Police::MonoGras, ty.fort),
            k.t.largeur(&temps, Police::MonoGras, ty.fort),
        ) + 10.0;
        // Un trait vertical sépare la lecture des réglages.
        k.t.p.line_segment(
            [
                Pos2::new(x - 0.5, r.center().y - 10.0),
                Pos2::new(x - 0.5, r.center().y + 10.0),
            ],
            Stroke::new(1.0, c.bord.c()),
        );
        x += 8.0;

        // Vitesse. Fenêtre très étroite : l'icône seule, la vitesse dans la bulle.
        let libelle = at.libelle_vitesse(k.a);
        let l_libelle = 8.0
            + 16.0
            + 8.0
            + k.t
                .largeur(
                    &tr!("{n} pas/s", n = crate::langue::decimal("1.7".to_owned())),
                    Police::CorpsMoyen,
                    ty.corps,
                )
                .max(k.t.largeur(&libelle, Police::CorpsMoyen, ty.corps))
            + 10.0;
        let l = if d.etroit { b } else { l_libelle };
        let rect = Rect::from_min_size(Pos2::new(x, r.center().y - b / 2.0), Vec2::new(l, b));
        let bulle = if d.etroit {
            tr!("Changer la vitesse ({libelle})", libelle = libelle)
        } else {
            tr!("Changer la vitesse").to_owned()
        };
        let touche = k.toucher(rect, egui::Id::unique("vitesse"), &bulle, Sense::click());
        k.t.p.rect_filled(
            touche.rect,
            k.a.formes.rayon_bouton,
            c.survol.alpha(touche.chaud),
        );
        let encre = c.doux.vers(c.texte, touche.chaud).c();
        if d.etroit {
            k.t.icone(Icone::Vitesse, touche.rect.center(), 17.0, encre);
        } else {
            k.t.icone(
                Icone::Vitesse,
                Pos2::new(touche.rect.left() + 16.0, rect.center().y),
                16.0,
                encre,
            );
            k.t.texte(
                Pos2::new(touche.rect.left() + 32.0, rect.center().y),
                Align2::LEFT_CENTER,
                &libelle,
                Police::CorpsMoyen,
                ty.corps,
                encre,
            );
        }
        if touche.clic {
            at.changer_vitesse(k.a);
        }
        x += l + 8.0;

        // État.
        let (texte, couleur, bulle) = match at.etat() {
            Etat::Stable => (tr!("Stable").to_owned(), c.un, tr!("Plus rien ne bouge")),
            Etat::EnCours if at.rapide(k.a).is_some() && at.debit > 0.0 => (
                debit_lisible(at.debit),
                c.attente,
                tr!("Vitesse réellement atteinte par la simulation"),
            ),
            Etat::EnCours => (
                tr!("En cours").to_owned(),
                c.attente,
                tr!("Le courant se propage"),
            ),
            Etat::EnPause => (
                tr!("En pause").to_owned(),
                c.doux,
                tr!("Des portes attendent : lance la lecture ou avance d'un pas"),
            ),
            Etat::Passe(t) => (
                tr!("Passé, t = {t}", t = t),
                c.accent_clair,
                tr!("Tu regardes le passé : clique pour revenir au présent"),
            ),
        };
        let voulu = 12.0
            + 8.0
            + 8.0
            + self.place_etat.garder(
                k.t.largeur(&tr!("{n} k pas/s", n = "000"), Police::CorpsDemi, ty.corps),
                k.t.largeur(&texte, Police::CorpsDemi, ty.corps),
            )
            + 14.0;
        // Si la carte n'a pas pu avoir toute sa largeur (très petite fenêtre, très grand texte,
        // temps à huit chiffres), l'état s'abrège plutôt que de passer sous les cartes voisines.
        let l = voulu.min(r.right() - 8.0 - x).max(44.0);
        let texte = k.t.abreger(&texte, Police::CorpsDemi, ty.corps, l - 40.0);
        let rect = Rect::from_min_size(Pos2::new(x, r.center().y - 14.0), Vec2::new(l, 28.0));
        let touche = k.toucher(rect, egui::Id::unique("etat"), bulle, Sense::click());
        // Une pastille d'état teintée, comme un voyant : la couleur dit l'état, le texte aussi.
        k.t.p.rect(
            touche.rect,
            touche.rect.height() / 2.0,
            couleur.alpha(0.12 + 0.06 * touche.chaud),
            Stroke::new(1.0, couleur.alpha(0.45 + 0.3 * touche.chaud)),
            StrokeKind::Inside,
        );
        let pouls = if at.etat() == Etat::EnCours && !k.a.mouvement.sobre {
            0.5 + 0.5 * k.a.mouvement.pouls(at.horloge, 0.65)
        } else {
            1.0
        };
        let voyant = Pos2::new(touche.rect.left() + 15.0, rect.center().y);
        k.t.p
            .circle_filled(voyant, 6.5 * pouls, couleur.alpha(0.25));
        k.t.p.circle_filled(voyant, 3.5, couleur.c());
        k.t.texte(
            Pos2::new(touche.rect.left() + 27.0, rect.center().y),
            Align2::LEFT_CENTER,
            &texte,
            Police::CorpsDemi,
            ty.corps,
            c.texte.c(),
        );
        if touche.clic && at.vue_t.is_some() {
            at.vue_t = None;
        }
        x += voulu;
        // Les deux largeurs, quelle que soit celle affichée : la mise en page choisit.
        let fin = x - r.left() + 8.0;
        let l_vitesse = if d.etroit { b } else { l_libelle };
        self.contenus.transport = fin - l_vitesse + l_libelle;
        self.contenus.transport_reduit = fin - l_vitesse + b;
    }

    fn volets(
        &mut self,
        k: &mut Cadre,
        at: &mut Atelier,
        d: &Disposition,
        demandes: &mut Demandes,
    ) {
        let r = d.volets;
        k.couvrir(r, egui::Id::unique("volets"));
        k.t.carte(r, k.a.formes.rayon_carte);
        let c = k.t.c;
        let b = k.a.formes.bouton;
        let mut x = r.left() + 8.0;
        let boutons: [(&str, Icone, &str); 5] = [
            (
                tr!("Briques"),
                Icone::Briques,
                tr!("Montrer ou cacher les briques"),
            ),
            (
                tr!("Quête"),
                Icone::Quete,
                tr!("Montrer ou cacher la quête"),
            ),
            (
                tr!("Infos"),
                Icone::Infos,
                tr!("Montrer ou cacher les infos"),
            ),
            (
                tr!("Temps"),
                Icone::Temps,
                tr!("Montrer ou cacher le chronogramme"),
            ),
            (
                tr!("Étiquettes"),
                Icone::Etiquettes,
                tr!("Montrer ou cacher les étiquettes des signaux"),
            ),
        ];
        let libelle = |k: &Cadre, nom: &str| {
            8.0 + 16.0 + 7.0 + k.t.largeur(nom, Police::CorpsMoyen, k.a.typo.corps) + 10.0
        };
        // Les deux largeurs de la carte, quelle que soit celle affichée : la mise en page choisit.
        let libelles: f32 = boutons.iter().map(|(nom, ..)| libelle(k, nom) + 2.0).sum();
        self.contenus.volets = 8.0 + libelles + 9.0 + b + 8.0;
        self.contenus.volets_icones = 8.0 + boutons.len() as f32 * (b + 2.0) + 9.0 + b + 8.0;
        for (i, (nom, icone, bulle)) in boutons.into_iter().enumerate() {
            let ouvert = match i {
                0 => &mut at.volets.briques,
                1 => &mut at.volets.quete,
                2 => &mut at.volets.infos,
                3 => &mut at.volets.temps,
                _ => &mut at.volets.etiquettes,
            };
            let l = if d.compact { b } else { libelle(k, nom) };
            let rect = Rect::from_min_size(Pos2::new(x, r.center().y - b / 2.0), Vec2::new(l, b));
            let touche = k.toucher(rect, egui::Id::unique(("volet", i)), bulle, Sense::click());
            let allume = k.anims.vers(
                egui::Id::unique(("volet-allume", i)),
                if *ouvert { 1.0 } else { 0.0 },
                k.dt,
                k.a.mouvement.lissage_ui,
            );
            // Contrôle segmenté : un volet ouvert est « enfoncé » (surface plus claire, icône
            // d'accent, petit trait dessous) ; un volet fermé n'a pas de fond.
            let fond = c
                .survol
                .alpha(touche.chaud * (1.0 - allume))
                .lerp_to_gamma(c.enfonce.c(), allume);
            k.t.p
                .rect_filled(touche.rect, k.a.formes.rayon_bouton, fond);
            if allume > 0.01 {
                let largeur = 14.0 * allume;
                k.t.p.rect_filled(
                    Rect::from_center_size(
                        Pos2::new(touche.rect.center().x, touche.rect.bottom() - 2.5),
                        Vec2::new(largeur, 2.0),
                    ),
                    1.0,
                    c.accent_clair.alpha(allume),
                );
            }
            let encre = c.doux.vers(c.texte, allume.max(touche.chaud)).c();
            let encre_icone = c
                .doux
                .vers(c.texte, touche.chaud)
                .vers(c.accent_clair, allume)
                .c();
            if d.compact {
                k.t.icone(icone, touche.rect.center(), 17.0, encre_icone);
            } else {
                k.t.icone(
                    icone,
                    Pos2::new(touche.rect.left() + 16.0, rect.center().y),
                    16.0,
                    encre_icone,
                );
                k.t.texte(
                    Pos2::new(touche.rect.left() + 31.0, rect.center().y),
                    Align2::LEFT_CENTER,
                    nom,
                    Police::CorpsMoyen,
                    k.a.typo.corps,
                    encre,
                );
            }
            if touche.clic {
                *ouvert = !*ouvert;
            }
            x += l + 2.0;
        }
        // Le thème, séparé des volets par un trait.
        k.t.p.line_segment(
            [
                Pos2::new(x + 3.5, r.center().y - 10.0),
                Pos2::new(x + 3.5, r.center().y + 10.0),
            ],
            Stroke::new(1.0, c.bord.c()),
        );
        x += 9.0;
        let rect = Rect::from_min_size(Pos2::new(x, r.center().y - b / 2.0), Vec2::splat(b));
        let (icone, bulle) = match k.a.mode {
            Mode::Sombre => (Icone::Soleil, tr!("Passer en clair")),
            Mode::Clair => (Icone::Lune, tr!("Passer en sombre")),
        };
        if k.bouton_icone(rect, "mode", icone, Style::Nu, bulle, true) {
            demandes.changer_mode = true;
        }
    }

    fn briques(
        &mut self,
        k: &mut Cadre,
        at: &mut Atelier,
        plan: &mut Plan,
        d: &Disposition,
        ouverture: f32,
        demandes: &mut Demandes,
    ) {
        let r = d.briques;
        let Some(t) = Self::panneau(k, r, "briques", ouverture) else {
            return;
        };
        let fond = std::mem::replace(&mut k.t, t);
        let c = k.t.c;
        if k.entete(r, tr!("Briques"), "fermer-briques") {
            at.volets.briques = false;
        }
        let ty = k.a.typo.clone();
        let rb = k.a.formes.rayon_bouton;
        let pad = k.a.formes.rembourrage;
        let (x, l) = (r.left() + pad, r.width() - 2.0 * pad);
        // Le contenu défile sous le titre quand il dépasse la hauteur du panneau.
        let zone = Rect::from_min_max(
            Pos2::new(r.left(), r.top() + ENTETE + 1.0),
            Pos2::new(r.right(), r.bottom() - 8.0),
        );
        k.t = k.t.sur(1.0, zone);
        k.decoupe = Some(zone);
        let mut y = r.top() + ENTETE + 12.0 - self.defilement_briques;
        y += k.t.paragraphe(
            Pos2::new(x + 2.0, y),
            l - 4.0,
            tr!("Glisse un élément sur le plan, ou clique dessus pour l'ajouter."),
            Police::Corps,
            ty.petit,
            c.doux.c(),
        ) + 12.0;

        // La NAND : la seule porte de base. On la tire sur le plan, ou on clique.
        let rect = Rect::from_min_size(Pos2::new(x, y), Vec2::new(l, 52.0));
        k.reperes.push(("nand", 0, rect));
        let touche = k.toucher(
            rect,
            egui::Id::unique("brique-nand"),
            "",
            Sense::click_and_drag(),
        );
        k.t.element(touche.rect, rb + 2.0, touche.chaud, false);
        let pastille = Rect::from_min_size(
            Pos2::new(touche.rect.left() + 8.0, touche.rect.center().y - 18.0),
            Vec2::splat(36.0),
        );
        // La pièce maîtresse porte la couleur du 1 : c'est la seule porte de base.
        k.t.p.rect(
            pastille,
            rb,
            c.un.alpha(0.12),
            Stroke::new(1.0, c.un.alpha(0.45)),
            StrokeKind::Inside,
        );
        k.t.icone(Icone::Nand, pastille.center(), 20.0, c.un.c());
        k.t.texte(
            Pos2::new(pastille.right() + 12.0, rect.center().y - 8.0),
            Align2::LEFT_CENTER,
            "NAND",
            Police::MonoGras,
            ty.corps,
            c.texte.c(),
        );
        k.t.texte(
            Pos2::new(pastille.right() + 12.0, rect.center().y + 9.0),
            Align2::LEFT_CENTER,
            &k.t.abreger(
                tr!("porte de base · glisse-la"),
                Police::Corps,
                ty.petit,
                rect.right() - pastille.right() - 24.0,
            ),
            Police::Corps,
            ty.petit,
            c.doux.c(),
        );
        if touche.reponse.drag_started() {
            if let Some(p) = touche.reponse.interact_pointer_pos() {
                plan.commencer_pose(at, Element::nand(""), p);
            }
        } else if touche.clic {
            plan.poser_au_centre(k.a, at, Element::nand(""));
        }
        y += 52.0 + 8.0;

        // Ce qui entoure les portes : de quoi entrer, sortir, cadencer, découper les bus.
        let outils: [Outil; 10] = [
            (
                tr!("Interrupteur"),
                Icone::Interrupteur,
                tr!("Une entrée du circuit : clique dessus pour la changer"),
                || Element::entree("", 1),
            ),
            (tr!("Lampe"), Icone::Lampe, tr!("Une sortie du circuit"), || {
                Element::sortie("", "", 1)
            }),
            (
                tr!("Horloge"),
                Icone::Horloge,
                tr!("Bascule toute seule, régulièrement"),
                || Element::horloge("", DEMI_PERIODE_DEFAUT),
            ),
            (
                tr!("Démarrage"),
                Icone::Demarrage,
                tr!("Vaut 1 à la mise sous tension puis retombe à 0 : pour remettre un circuit à zéro"),
                || Element::impulsion("", 20),
            ),
            (
                tr!("Poussoir"),
                Icone::Poussoir,
                tr!("Un bouton : vaut 1 tant qu'on appuie dessus, 0 dès qu'on le lâche"),
                || Element::poussoir(""),
            ),
            (
                tr!("Afficheur"),
                Icone::Segments,
                tr!("Afficheur 7 segments : une lampe de 7 bits, a (en haut) à g (au milieu)"),
                || Element::afficheur("", ""),
            ),
            (
                tr!("Écran"),
                Icone::Ecran,
                tr!("Affiche le caractère présenté sur d quand ecrit passe à 1"),
                || Element::ecran(""),
            ),
            (
                tr!("Clavier"),
                Icone::Clavier,
                tr!("Présente le code de la touche tapée, sur 8 bits"),
                || Element::clavier(""),
            ),
            (
                tr!("Découpeur"),
                Icone::Decoupe,
                tr!("Sépare un bus en morceaux"),
                || Element::decoupe("", 4, &[(0, 0), (1, 1), (2, 2), (3, 3)]),
            ),
            (
                tr!("Jonction"),
                Icone::Jonction,
                tr!("Réunit des fils en un bus"),
                || Element::jonction("", &[1, 1, 1, 1]),
            ),
        ];
        let demi = (l - 6.0) / 2.0;
        for (i, (nom, icone, bulle, fabrique)) in outils.into_iter().enumerate() {
            let rect = Rect::from_min_size(
                Pos2::new(x + (i % 2) as f32 * (demi + 6.0), y + (i / 2) as f32 * 38.0),
                Vec2::new(demi, 32.0),
            );
            k.reperes.push(("outil", i, rect));
            let touche = k.toucher(
                rect,
                egui::Id::unique(("outil", i)),
                bulle,
                Sense::click_and_drag(),
            );
            k.t.element(touche.rect, rb, touche.chaud, false);
            k.t.icone(
                icone,
                Pos2::new(touche.rect.left() + 15.0, rect.center().y),
                15.0,
                c.doux.vers(c.texte, touche.chaud).c(),
            );
            k.t.texte(
                Pos2::new(touche.rect.left() + 29.0, rect.center().y),
                Align2::LEFT_CENTER,
                &k.t.abreger(nom, Police::CorpsMoyen, ty.petit, demi - 33.0),
                Police::CorpsMoyen,
                ty.petit,
                c.texte.c(),
            );
            if touche.reponse.drag_started() {
                if let Some(p) = touche.reponse.interact_pointer_pos() {
                    plan.commencer_pose(at, fabrique(), p);
                }
            } else if touche.clic {
                plan.poser_au_centre(k.a, at, fabrique());
            }
        }
        y += 5.0 * 38.0 + 10.0;

        // Les briques du projet : tout ce qui a été regroupé ou enregistré.
        let nb = at.projet.modeles.len();
        y += k.rubrique(
            x + 2.0,
            y,
            l - 4.0,
            tr!("Mes briques"),
            &if nb == 0 {
                String::new()
            } else {
                nb.to_string()
            },
        );
        if nb == 0 {
            y += k.t.paragraphe(
                Pos2::new(x + 2.0, y),
                l - 4.0,
                tr!("Aucune pour l'instant. Sélectionne des portes (Maj + glisser) puis « Regrouper », ou enregistre tout le plan."),
                Police::Corps,
                ty.petit,
                c.discret.c(),
            ) + 10.0;
        }
        // Une recherche, dès qu'il y a de quoi chercher.
        if nb > 4 {
            let rect = Rect::from_min_size(Pos2::new(x, y), Vec2::new(l, 30.0));
            k.reperes.push(("filtre-briques", 0, rect));
            let valeur = self.filtre_briques.clone();
            if let Some(texte) = self.champ(
                k,
                rect,
                Champ::Filtre,
                &valeur,
                Police::Corps,
                ty.petit,
                tr!("Cherche une brique par son nom ou sa description"),
            ) {
                self.filtre_briques = texte.trim().to_owned();
            }
            let en_saisie = self
                .edition
                .as_ref()
                .is_some_and(|e| e.champ == Champ::Filtre);
            if !en_saisie && self.filtre_briques.is_empty() {
                k.t.texte(
                    Pos2::new(rect.left() + 9.0, rect.center().y),
                    Align2::LEFT_CENTER,
                    tr!("Chercher une brique…"),
                    Police::Corps,
                    ty.petit,
                    c.discret.c(),
                );
            }
            y += 30.0 + 10.0;
        }
        // Le filtre suit la frappe, sans attendre Entrée.
        let filtre = crate::fenetres::plier(
            self.edition
                .as_ref()
                .filter(|e| e.champ == Champ::Filtre)
                .map_or(self.filtre_briques.as_str(), |e| e.texte.as_str())
                .trim(),
        );
        self.resumer(at);
        let resumes = self
            .resumes
            .as_ref()
            .map(|r| r.2.clone())
            .unwrap_or_default();
        let mut a_ouvrir = None;
        let mut trouvees = 0;
        for (i, resume) in resumes.iter().enumerate() {
            let nom = at.projet.modeles[i].nom.clone();
            let affiche = crate::langue::nom_affiche(&nom).into_owned();
            if !filtre.is_empty()
                && !crate::fenetres::plier(&nom).contains(&filtre)
                && !crate::fenetres::plier(&affiche).contains(&filtre)
                && !crate::fenetres::plier(&at.projet.modeles[i].description).contains(&filtre)
            {
                continue;
            }
            trouvees += 1;
            let rect = Rect::from_min_size(Pos2::new(x, y), Vec2::new(l, 42.0));
            y += 42.0 + 4.0;
            if !zone.intersects(rect) {
                continue;
            }
            let posable = resume.posable;
            let ici = at.lieu() == Some(nom.as_str());
            let portes = resume.portes;
            let sous = if ici {
                tr!("ouverte en ce moment").to_owned()
            } else if !posable {
                tr!("contient la brique ouverte").to_owned()
            } else {
                tr!(
                    "{portes} porte{s} · {entrees} → {sorties}",
                    portes = portes,
                    s = crate::langue::s(portes),
                    entrees = resume.entrees,
                    sorties = resume.sorties
                )
            };
            let ouvrir = Rect::from_min_size(
                Pos2::new(rect.right() - 33.0, rect.center().y - 13.0),
                Vec2::splat(26.0),
            );
            let corps = Rect::from_min_max(rect.min, Pos2::new(ouvrir.left() - 2.0, rect.bottom()));
            k.reperes.push(("modele", i, corps));
            k.reperes.push(("ouvrir-modele", i, ouvrir));
            let description = at.projet.modeles[i].description.trim();
            let bulle = if !posable {
                tr!("Une brique ne peut pas se contenir elle-même").to_owned()
            } else if description.is_empty() {
                tr!("Glisse sur le plan, ou clique pour poser").to_owned()
            } else {
                format!(
                    "{}\n{}",
                    crate::langue::traduire_texte(description),
                    tr!("Glisse sur le plan, ou clique pour poser")
                )
            };
            let touche = k.toucher(
                corps,
                egui::Id::unique(("modele", i)),
                &bulle,
                if posable {
                    Sense::click_and_drag()
                } else {
                    Sense::hover()
                },
            );
            let chaud = if posable { touche.chaud } else { 0.0 };
            k.t.element(rect, rb, chaud, ici);
            let force = if posable { 1.0 } else { 0.5 };
            k.t.icone(
                Icone::Brique,
                Pos2::new(rect.left() + 17.0, rect.center().y),
                16.0,
                c.doux.vers(c.texte, chaud).alpha(force),
            );
            let place = ouvrir.left() - rect.left() - 38.0;
            k.t.texte(
                Pos2::new(rect.left() + 33.0, rect.center().y - 8.0),
                Align2::LEFT_CENTER,
                &k.t.abreger(&affiche, Police::MonoGras, ty.petit, place),
                Police::MonoGras,
                ty.petit,
                c.texte.alpha(force),
            );
            k.t.texte(
                Pos2::new(rect.left() + 33.0, rect.center().y + 8.5),
                Align2::LEFT_CENTER,
                &sous,
                Police::Corps,
                ty.legende,
                c.discret.alpha(force),
            );
            if k.bouton_icone(
                ouvrir,
                &format!("ouvrir-modele-{i}"),
                Icone::Ouvrir,
                Style::Nu,
                tr!("Ouvrir cette brique pour voir et modifier son plan"),
                !ici,
            ) {
                a_ouvrir = Some(nom.clone());
            }
            if posable && touche.reponse.drag_started() {
                if let (Some(p), Some(element)) = (
                    touche.reponse.interact_pointer_pos(),
                    at.projet.modele(&nom).map(|m| m.instance()),
                ) {
                    plan.commencer_pose(at, element, p);
                }
            } else if posable && touche.clic {
                if let Some(element) = at.projet.modele(&nom).map(|m| m.instance()) {
                    plan.poser_au_centre(k.a, at, element);
                }
            }
        }
        if trouvees == 0 && !filtre.is_empty() {
            y += k.t.paragraphe(
                Pos2::new(x + 2.0, y),
                l - 4.0,
                tr!("Aucune brique ne correspond à cette recherche."),
                Police::Corps,
                ty.petit,
                c.discret.c(),
            ) + 10.0;
        }
        if let Some(nom) = a_ouvrir {
            // Depuis la palette, on ouvre la brique à partir du plan principal.
            at.remonter(0);
            at.entrer_dans(&nom);
        }

        y += 6.0;
        let nb_groupables = at
            .elements_selectionnes()
            .iter()
            .filter(|id| at.doc.element(**id).is_some_and(|e| groupable(e.genre)))
            .count();
        let (texte, actif) = if nb_groupables > 0 {
            (tr!("Regrouper la sélection"), true)
        } else if at.chemin.is_empty() {
            (tr!("Enregistrer comme brique"), true)
        } else {
            (tr!("Regrouper la sélection"), false)
        };
        let rect = Rect::from_min_size(Pos2::new(x, y), Vec2::new(l, 32.0));
        k.reperes.push(("enregistrer-brique", 0, rect));
        // Action secondaire : l'action principale de l'atelier reste celle de la quête.
        if k.bouton_second(rect, "enregistrer-brique", Icone::Grouper, texte, actif) {
            if nb_groupables > 0 {
                at.regrouper_selection(k.a);
            } else {
                at.plan_en_brique();
            }
        }
        y += 32.0 + 18.0;

        // Les projets : celui qui est ouvert, ceux qui sont enregistrés, les exemples.
        y += k.rubrique(x + 2.0, y, l - 4.0, tr!("Projets"), "");
        let mut lignes: Vec<(String, Option<Ouverture>, &str)> = vec![(
            at.projet.nom.clone(),
            None,
            tr!("ouvert, enregistré tout seul"),
        )];
        for nom in self.projets.iter().filter(|n| **n != at.projet.nom) {
            lignes.push((
                nom.clone(),
                Some(Ouverture::Projet(nom.clone())),
                tr!("enregistré"),
            ));
        }
        for (rang, exemple) in EXEMPLES.iter().enumerate() {
            let nom = crate::langue::traduire(exemple.nom);
            if at.projet.nom != nom && !self.projets.iter().any(|n| n == nom) {
                lignes.push((
                    nom.to_owned(),
                    Some(Ouverture::Exemple(rang)),
                    crate::langue::traduire(exemple.resume),
                ));
            }
        }
        lignes.push((
            tr!("Nouveau projet").to_owned(),
            Some(Ouverture::Nouveau),
            tr!("un plan vide"),
        ));
        for (i, (nom, ouverture, sous)) in lignes.into_iter().enumerate() {
            let rect = Rect::from_min_size(Pos2::new(x, y), Vec2::new(l, 40.0));
            y += 40.0 + 2.0;
            if !zone.intersects(rect) {
                continue;
            }
            let ouvert = ouverture.is_none();
            k.reperes.push(("projet", i, rect));
            let touche = k.toucher(
                rect,
                egui::Id::unique(("projet", i)),
                "",
                if ouvert {
                    Sense::hover()
                } else {
                    Sense::click()
                },
            );
            let chaud = if ouvert { 0.0 } else { touche.chaud };
            k.ligne_de_liste(touche.rect, chaud, ouvert);
            k.t.texte(
                Pos2::new(rect.left() + 12.0, rect.center().y - 8.0),
                Align2::LEFT_CENTER,
                &k.t.abreger(&nom, Police::CorpsMoyen, ty.corps, l - 24.0),
                Police::CorpsMoyen,
                ty.corps,
                c.texte.c(),
            );
            k.t.texte(
                Pos2::new(rect.left() + 12.0, rect.center().y + 9.0),
                Align2::LEFT_CENTER,
                &k.t.abreger(sous, Police::Corps, ty.legende, l - 24.0),
                Police::Corps,
                ty.legende,
                c.discret.c(),
            );
            if touche.clic {
                demandes.ouvrir = ouverture;
            }
        }

        let hauteur = y + self.defilement_briques - zone.top() + 6.0;
        self.contenus.briques = hauteur + ENTETE + 10.0;
        self.defilement_briques = k.defiler(r, zone, hauteur, self.defilement_briques, ouverture);
        k.decoupe = None;
        k.t = fond;
    }

    fn quete(
        &mut self,
        k: &mut Cadre,
        at: &mut Atelier,
        d: &Disposition,
        ouverture: f32,
        demandes: &mut Demandes,
    ) {
        let r = d.quete;
        let Some(t) = Self::panneau(k, r, "quete", ouverture) else {
            return;
        };
        let fond = std::mem::replace(&mut k.t, t);
        let y = match at.quete.clone().filter(|_| at.chemin.is_empty()) {
            Some(quete) => self.table_de_la_quete(k, at, &quete, r, demandes),
            None => self.fiche_du_lieu(k, at, r, demandes),
        };
        // Le contenu défile sous le titre quand il dépasse la hauteur du panneau.
        let zone = Self::zone_de_la_quete(r);
        let hauteur = y + self.defilement_quete - zone.top() + 12.0;
        self.contenus.quete = hauteur + (zone.top() - r.top()) + 10.0;
        self.defilement_quete = k.defiler(r, zone, hauteur, self.defilement_quete, ouverture);
        k.decoupe = None;
        k.t = fond;
    }

    fn zone_de_la_quete(r: Rect) -> Rect {
        Rect::from_min_max(
            Pos2::new(r.left(), r.top() + ENTETE + 1.0),
            Pos2::new(r.right(), r.bottom() - 8.0),
        )
    }

    /// Commence le contenu qui défile, sous le titre du panneau : renvoie le haut du contenu.
    fn defilement_de_la_quete(&self, k: &mut Cadre, r: Rect) -> f32 {
        let zone = Self::zone_de_la_quete(r);
        k.t = k.t.sur(1.0, zone);
        k.decoupe = Some(zone);
        r.top() + ENTETE + 14.0 - self.defilement_quete
    }

    /// La liste des quêtes du parcours : réussies, en cours, à venir. Renvoie le bas.
    fn liste_du_parcours(
        &mut self,
        k: &mut Cadre,
        at: &Atelier,
        x: f32,
        l: f32,
        mut y: f32,
        demandes: &mut Demandes,
    ) -> f32 {
        let c = k.t.c;
        k.t.legende(
            Pos2::new(x, y + 8.0),
            Align2::LEFT_CENTER,
            tr!("PARCOURS"),
            c.discret.c(),
        );
        let faites = PARCOURS.iter().filter(|f| at.parcours.faite(f.cle)).count();
        k.t.texte(
            Pos2::new(x + l, y + 8.0),
            Align2::RIGHT_CENTER,
            &tr!(
                "{faites} sur {total}",
                faites = faites,
                total = PARCOURS.len()
            ),
            Police::Mono,
            k.a.typo.petit,
            c.doux.c(),
        );
        y += 24.0;
        if self.etoiles.as_ref().map(|(v, _)| *v) != Some(at.parcours.version) {
            self.etoiles = Some((
                at.parcours.version,
                economie::etoiles_du_parcours(&at.parcours),
            ));
        }
        let etoiles = self
            .etoiles
            .as_ref()
            .map(|(_, e)| e.clone())
            .unwrap_or_default();
        let en_cours = at.quete.as_ref().map(|q| q.cle);
        let prochaine = at.parcours.prochaine().map(|f| f.cle);
        let mut chapitre = "";
        let mut branche = savoir::Branche::Tronc;
        for (i, f) in PARCOURS.iter().enumerate() {
            if f.chapitre != chapitre {
                chapitre = f.chapitre;
                let ici = savoir::chapitre(chapitre).map_or(branche, |c| c.branche);
                if ici != branche {
                    branche = ici;
                    y += 8.0;
                    k.t.legende(
                        Pos2::new(x, y + 10.0),
                        Align2::LEFT_CENTER,
                        &k.t.abreger_legende(&ici.nom().to_uppercase(), l),
                        c.accent_clair.alpha(0.8),
                    );
                    y += 22.0;
                }
                let fait = PARCOURS
                    .iter()
                    .filter(|g| g.chapitre == chapitre)
                    .all(|g| at.parcours.faite(g.cle));
                k.t.texte(
                    Pos2::new(x, y + 14.0),
                    Align2::LEFT_CENTER,
                    crate::langue::traduire(chapitre),
                    Police::CorpsDemi,
                    k.a.typo.petit,
                    if fait { c.un.c() } else { c.accent_clair.c() },
                );
                y += 26.0;
            }
            let rect = Rect::from_min_size(Pos2::new(x - 6.0, y), Vec2::new(l + 12.0, 30.0));
            y += 31.0;
            let ici = en_cours == Some(f.cle);
            k.reperer("quete-du-parcours", i, rect);
            let touche = k.toucher(
                rect,
                egui::Id::unique(("parcours", i)),
                if ici {
                    ""
                } else {
                    tr!("Commencer cette quête")
                },
                if ici { Sense::hover() } else { Sense::click() },
            );
            let chaud = if ici { 0.0 } else { touche.chaud };
            let faite = at.parcours.faite(f.cle);
            let suivante = prochaine == Some(f.cle);
            k.ligne_de_liste(rect, chaud, ici);
            let centre = Pos2::new(rect.left() + 20.0, rect.center().y);
            if faite {
                k.t.icone(Icone::Coche, centre, 16.0, c.un.c());
            } else {
                k.t.texte(
                    centre,
                    Align2::CENTER_CENTER,
                    &(i + 1).to_string(),
                    Police::Mono,
                    k.a.typo.petit,
                    if suivante {
                        c.accent_clair.c()
                    } else {
                        c.doux.c()
                    },
                );
            }
            // Les étoiles de la brique gagnée, à droite de la ligne.
            let place_etoiles = match etoiles.get(i).copied().flatten() {
                Some(n) => {
                    Self::etoiles(k, n, Pos2::new(rect.right() - 10.0, rect.center().y), false);
                    3.0 * 2.5 * k.a.formes.etoile_petite + 6.0
                }
                None => 0.0,
            };
            k.t.texte(
                Pos2::new(rect.left() + 36.0, rect.center().y),
                Align2::LEFT_CENTER,
                &k.t.abreger(
                    crate::langue::traduire(f.titre),
                    Police::CorpsMoyen,
                    k.a.typo.corps,
                    l - 46.0 - place_etoiles,
                ),
                Police::CorpsMoyen,
                k.a.typo.corps,
                if ici || suivante {
                    c.texte.c()
                } else {
                    c.doux.vers(c.texte, 0.4 + 0.6 * chaud).c()
                },
            );
            if touche.clic {
                demandes.ouvrir = Some(Ouverture::Quete(f.cle.to_owned()));
            }
        }
        y
    }

    /// Sans quête, ou dans une brique : ce qu'est le plan affiché.
    fn fiche_du_lieu(
        &mut self,
        k: &mut Cadre,
        at: &mut Atelier,
        r: Rect,
        demandes: &mut Demandes,
    ) -> f32 {
        let c = k.t.c;
        let pad = k.a.formes.rembourrage + 2.0;
        let (x, l) = (r.left() + pad, r.width() - 2.0 * pad);
        let dans_brique = at.lieu().map(str::to_owned);
        k.t.texte(
            Pos2::new(x, r.top() + ENTETE / 2.0),
            Align2::LEFT_CENTER,
            if dans_brique.is_some() {
                tr!("BRIQUE")
            } else {
                tr!("PROJET")
            },
            Police::CorpsDemi,
            k.a.typo.legende,
            c.doux.c(),
        );
        if k.fermer(r, "fermer-quete") {
            at.volets.quete = false;
        }
        let mut y = self.defilement_de_la_quete(k, r);
        let titre = dans_brique
            .as_deref()
            .map(|n| crate::langue::nom_affiche(n).into_owned())
            .unwrap_or_else(|| at.projet.nom.clone());
        let champ = match &dans_brique {
            Some(nom) => Champ::Modele(nom.clone()),
            None => Champ::Projet,
        };
        let rect = Rect::from_min_size(Pos2::new(x - 6.0, y), Vec2::new(l + 12.0, 32.0));
        if let Some(nom) = self.champ(
            k,
            rect,
            champ.clone(),
            &titre,
            Police::Titre,
            k.a.typo.titre,
            tr!("Clique pour changer le nom"),
        ) {
            Self::appliquer(at, k.a, champ, &nom);
        }
        y += 32.0 + 10.0;
        // Les portes de ce plan, briques comprises (pas celles de tout ce qui tourne autour).
        let portes = at.projet.nb_portes(&at.doc);
        let niveaux = at.projet.profondeur(&at.doc);
        let pluriel = |n: u64| crate::langue::s(n);
        let mut texte = tr!(
            "{portes} porte{v} NAND",
            portes = portes,
            v = pluriel(portes)
        );
        if niveaux > 0 {
            texte.push_str(&tr!(
                " en tout, sur {niveaux} niveau{v} de briques emboîtées.",
                niveaux = niveaux,
                v = match crate::langue::s(niveaux) {
                    "" => "",
                    _ if crate::langue::en_francais() => "x",
                    _ => "s",
                }
            ));
        } else {
            texte.push('.');
        }
        match &dans_brique {
            Some(nom) => {
                let (e, s) = at.doc.interface();
                let noms = |b: &[Broche]| {
                    if b.is_empty() {
                        tr!("aucune").to_owned()
                    } else {
                        b.iter()
                            .map(|b| {
                                if b.largeur > 1 {
                                    format!("{}[{}]", crate::langue::nom_affiche(&b.nom), b.largeur)
                                } else {
                                    crate::langue::nom_affiche(&b.nom).into_owned()
                                }
                            })
                            .collect::<Vec<_>>()
                            .join(", ")
                    }
                };
                let n = at.projet.nb_utilisations(nom);
                texte.push_str(&tr!(" Entrées : {v}. Sorties : {v2}. Posée {n} fois. Ses interrupteurs et ses lampes sont ses broches : ce que tu changes ici change partout.", n = n, v = noms(&e), v2 = noms(&s)));
                texte.push_str(if at.session.exemplaire() {
                    tr!(" Tu vois cet exemplaire tourner en direct dans le plan du dessus : ses entrées viennent de là-haut.")
                } else {
                    tr!(" Elle est simulée seule : ses interrupteurs se changent à la main.")
                });
            }
            None => texte.push_str(
                tr!(" Double-clique sur une brique pour l'ouvrir. Maj + glisser sélectionne plusieurs éléments."),
            ),
        }
        y += k.t.paragraphe(
            Pos2::new(x, y),
            l,
            &texte,
            Police::Corps,
            k.a.typo.corps,
            c.doux.c(),
        ) + 14.0;
        // La construction minimale de ce plan, s'il en a une connue.
        let cle = (at.version, at.chemin.clone(), at.parcours.version);
        if self.ecart_du_lieu.as_ref().map(|(c, _)| c) != Some(&cle) {
            let ecart =
                economie::du_plan(&at.projet, &at.doc, dans_brique.as_deref(), &at.parcours);
            self.ecart_du_lieu = Some((cle, ecart));
        }
        let ecart = self.ecart_du_lieu.as_ref().and_then(|(_, e)| e.clone());
        if portes > 0 {
            y = Self::construction_minimale(k, ecart.as_ref(), true, x, l, y);
        }
        if dans_brique.is_some() {
            let rect = Rect::from_min_size(Pos2::new(x, y), Vec2::new(l, 40.0));
            if k.bouton_texte(rect, "sortir-brique", tr!("Refermer la brique"), true) {
                at.sortir();
            }
            y += 40.0;
        } else {
            y += k.t.paragraphe(
                Pos2::new(x, y),
                l,
                tr!("Envie d'un défi ? Le parcours mène de la porte NON à l'ordinateur complet. Chaque quête réussie devient une brique pour les suivantes."),
                Police::Corps,
                k.a.typo.corps,
                c.doux.c(),
            ) + 14.0;
            y = self.liste_du_parcours(k, at, x, l, y, demandes);
        }
        y
    }

    fn table_de_la_quete(
        &mut self,
        k: &mut Cadre,
        at: &mut Atelier,
        quete: &Quete,
        r: Rect,
        demandes: &mut Demandes,
    ) -> f32 {
        let c = k.t.c;
        let pad = k.a.formes.rembourrage + 2.0;
        let (x, l) = (r.left() + pad, r.width() - 2.0 * pad);
        let cas = quete.cas();
        let total = cas.len();
        let rang = PARCOURS
            .iter()
            .position(|f| f.cle == quete.cle)
            .unwrap_or(0);
        // Où l'on est : le chapitre, et l'étape dans ce chapitre.
        let chapitre = PARCOURS.get(rang).map_or("", |f| f.chapitre);
        let etapes = savoir::quetes_du_chapitre(chapitre);
        let etape = etapes.iter().position(|&i| i == rang).unwrap_or(0);
        let entete = tr!(
            "{to_uppercase} · ÉTAPE {v} SUR {v2}",
            to_uppercase = crate::langue::traduire(chapitre).to_uppercase(),
            v = etape + 1,
            v2 = etapes.len().max(1)
        );
        let compteur = tr!("{vus} sur {total}", vus = quete.vus.len(), total = total);
        let l_compteur = if total > 0 {
            k.t.largeur(&compteur, Police::Mono, k.a.typo.petit) + 12.0
        } else {
            0.0
        };
        k.t.legende(
            Pos2::new(x, r.top() + ENTETE / 2.0),
            Align2::LEFT_CENTER,
            &k.t.abreger_legende(&entete, (r.right() - 48.0 - l_compteur - x).max(20.0)),
            c.discret.c(),
        );
        if total > 0 {
            k.t.texte(
                Pos2::new(r.right() - 48.0, r.top() + ENTETE / 2.0),
                Align2::RIGHT_CENTER,
                &compteur,
                Police::Mono,
                k.a.typo.petit,
                c.doux.c(),
            );
        }
        if k.fermer(r, "fermer-quete") {
            at.volets.quete = false;
        }
        let mut y = self.defilement_de_la_quete(k, r);
        let reussie = quete.reussie();
        y += k.t.paragraphe(
            Pos2::new(x, y),
            l,
            crate::langue::traduire(quete.titre),
            Police::Titre,
            k.a.typo.titre,
            c.texte.c(),
        ) + 8.0;
        y = Self::etapes(k, at, &etapes, rang, reussie, x, l, y);
        let consigne = if reussie {
            quete.felicitations.as_str()
        } else {
            crate::langue::traduire(quete.consigne)
        };
        y += k.t.paragraphe(
            Pos2::new(x, y),
            l,
            consigne,
            Police::Corps,
            k.a.typo.corps,
            if reussie { c.texte.c() } else { c.doux.c() },
        ) + 14.0;
        // Quête réussie : la brique gagnée, puis la suite, tout de suite, sans faire défiler.
        if reussie {
            y = Self::badge(k, at, quete, x, l, y);
            y = self.quete_suivante(k, at, quete, x, l, y, demandes);
        }
        // À quel point la construction est économe : l'écart, sous la brique gagnée une fois
        // la quête réussie ; avant, l'objectif, sous Vérifier (qui doit rester visible).
        let ecart = economie::de_la_quete(quete, &at.projet, &at.doc);
        if reussie {
            y = Self::construction_minimale(k, ecart.as_ref(), true, x, l, y);
        }
        y = self.comprendre(k, quete.cle, x, l, y);

        if total > 0 {
            y = self.cas_de_la_quete(k, at, quete, &cas, x, l, y);
        }
        y = self.verification(k, at, quete, x, l, y, demandes);
        if !reussie {
            y += 8.0;
            y = Self::construction_minimale(k, ecart.as_ref(), false, x, l, y);
        }
        y = Self::voir_la_solution(k, quete, x, l, y + 10.0, demandes);
        y += 22.0;
        self.liste_du_parcours(k, at, x, l, y, demandes)
    }

    /// Le bouton qui ouvre la solution de la quête, la plus petite construction connue, dans un
    /// projet à part. Renvoie le bas.
    fn voir_la_solution(
        k: &mut Cadre,
        quete: &Quete,
        x: f32,
        l: f32,
        y: f32,
        demandes: &mut Demandes,
    ) -> f32 {
        let rect = Rect::from_min_size(Pos2::new(x, y), Vec2::new(l, 32.0));
        k.reperer("voir-la-solution", 0, rect);
        if k.bouton_second(
            rect,
            "voir-la-solution",
            Icone::Lampe,
            tr!("Voir la solution"),
            true,
        ) {
            demandes.ouvrir = Some(Ouverture::Solution(quete.cle.to_owned()));
        }
        y + 32.0
    }

    /// Les étapes du chapitre, une pastille par quête : réussies, en cours, à venir. La
    /// pastille de la quête en cours se remplit en rebondissant quand elle est réussie.
    /// Renvoie le bas.
    #[allow(clippy::too_many_arguments)]
    fn etapes(
        k: &mut Cadre,
        at: &Atelier,
        etapes: &[usize],
        rang: usize,
        reussie: bool,
        x: f32,
        l: f32,
        y: f32,
    ) -> f32 {
        if etapes.len() < 2 {
            return y;
        }
        let c = k.t.c;
        let m = &k.a.mouvement;
        let pas = (l / etapes.len() as f32).min(26.0);
        let rayon = (pas * 0.32).clamp(3.5, 7.0);
        let cy = y + 10.0;
        // Le trait qui relie les étapes, rempli jusqu'à la dernière réussie.
        let premier = Pos2::new(x + rayon, cy);
        let dernier = Pos2::new(x + rayon + pas * (etapes.len() - 1) as f32, cy);
        k.t.p
            .line_segment([premier, dernier], Stroke::new(2.0, c.carte_2.c()));
        let faites = etapes
            .iter()
            .take_while(|&&i| at.parcours.faite(PARCOURS[i].cle) || (i == rang && reussie))
            .count();
        if faites > 1 {
            let bout = Pos2::new(x + rayon + pas * (faites - 1) as f32, cy);
            k.t.p
                .line_segment([premier, bout], Stroke::new(2.0, c.un.alpha(0.6)));
        }
        let fete = at
            .fete
            .map(|t0| (at.horloge - t0) as f32)
            .filter(|age| !m.sobre && *age < m.badge * 2.0);
        for (j, &i) in etapes.iter().enumerate() {
            let centre = Pos2::new(x + rayon + pas * j as f32, cy);
            let ici = i == rang;
            let faite = at.parcours.faite(PARCOURS[i].cle) || (ici && reussie);
            k.reperer(
                "etape",
                j,
                Rect::from_center_size(centre, Vec2::splat(2.0 * rayon)),
            );
            if ici {
                if let Some(age) = fete.filter(|_| faite) {
                    // L'onde qui part de l'étape réussie.
                    let q = (age / (m.badge * 2.0)).clamp(0.0, 1.0);
                    k.t.p.circle_stroke(
                        centre,
                        rayon + 18.0 * Courbe::Sortie.en(q),
                        Stroke::new(2.0 * (1.0 - q), c.un.alpha(1.0 - q)),
                    );
                    k.anims.en_mouvement = true;
                }
                let grossit = fete.filter(|_| faite).map_or(1.0, |age| {
                    0.4 + 0.6 * Courbe::Rebond.en(age / m.badge.max(0.01))
                });
                k.t.p
                    .circle_stroke(centre, rayon + 3.0, Stroke::new(1.5, c.accent_clair.c()));
                if faite {
                    k.t.p.circle_filled(centre, rayon * grossit, c.un.c());
                } else {
                    k.t.p.circle_filled(centre, rayon, c.accent.alpha(0.6));
                }
            } else if faite {
                k.t.p.circle_filled(centre, rayon, c.un.c());
            } else {
                k.t.p.circle_filled(centre, rayon, c.carte_2.c());
                k.t.p
                    .circle_stroke(centre, rayon, Stroke::new(1.0, c.bord.c()));
            }
        }
        y + 20.0 + 12.0
    }

    /// Trois étoiles dont `n` pleines, alignées à droite sur `droite` ; grandes ou petites.
    fn etoiles(k: &mut Cadre, n: u8, droite: Pos2, grandes: bool) {
        let c = k.t.c;
        let rayon = if grandes {
            k.a.formes.etoile
        } else {
            k.a.formes.etoile_petite
        };
        let pas = rayon * 2.5;
        for i in 0..3u8 {
            let centre = Pos2::new(droite.x - rayon - pas * (2 - i) as f32, droite.y);
            let pleine = i < n;
            let couleur = if pleine { c.un } else { c.bord_fort };
            k.t.etoile(centre, rayon, pleine, couleur.c());
        }
    }

    /// La carte « Construction minimale » : le nombre de portes de la construction, la plus
    /// petite construction connue et ce qu'on en sait, puis, une fois la construction juste,
    /// les étoiles, une jauge de proximité et une phrase. Sans cible connue, une ligne le dit.
    /// Renvoie le bas.
    fn construction_minimale(
        k: &mut Cadre,
        ecart: Option<&Ecart>,
        juste: bool,
        x: f32,
        l: f32,
        mut y: f32,
    ) -> f32 {
        let c = k.t.c;
        let rect_legende = k.t.legende(
            Pos2::new(x, y + 8.0),
            Align2::LEFT_CENTER,
            tr!("CONSTRUCTION MINIMALE"),
            c.discret.c(),
        );
        k.reperer("construction-minimale", 0, rect_legende);
        let Some(e) = ecart else {
            y += 22.0;
            y += k.t.paragraphe(
                Pos2::new(x, y),
                l,
                tr!("Pas de minimum connu pour ce plan : il a une mémoire, ou plus de 3 bits d'entrée."),
                Police::Corps,
                k.a.typo.petit,
                c.doux.c(),
            );
            return y + 14.0;
        };
        if juste {
            Self::etoiles(k, e.etoiles(), Pos2::new(x + l, y + 8.0), true);
        }
        y += 22.0;
        let pluriel = crate::langue::s(e.tiennes);
        let tiennes = match e.origine {
            Origine::Ajouts => tr!(
                "{tiennes} porte{pluriel} ajoutée{pluriel}",
                pluriel = pluriel,
                tiennes = e.tiennes
            ),
            _ => tr!(
                "{tiennes} porte{pluriel}",
                tiennes = e.tiennes,
                pluriel = pluriel
            ),
        };
        let ligne = if juste {
            format!("{tiennes} · {}", e.titre())
        } else {
            tr!(
                "{tiennes} pour l'instant · Objectif : {v}",
                tiennes = tiennes,
                v = e.titre().to_lowercase()
            )
        };
        y += k.t.paragraphe(
            Pos2::new(x, y),
            l,
            &ligne,
            Police::CorpsDemi,
            k.a.typo.corps,
            c.texte.c(),
        ) + 8.0;
        if juste {
            let piste = Rect::from_min_size(Pos2::new(x, y), Vec2::new(l, 4.0));
            k.reperer("jauge-minimale", 0, piste);
            k.t.p.rect_filled(piste, 2.0, c.enfonce.c());
            let mut plein = piste;
            plein.set_width((l * e.proximite()).max(4.0));
            let couleur = if e.en_trop() == 0 {
                c.un
            } else {
                c.accent_clair
            };
            k.t.p.rect_filled(plein, 2.0, couleur.c());
            y += 4.0 + 8.0;
        }
        let phrase = if juste {
            e.phrase()
        } else {
            tr!("Vérifie ta construction pour savoir à quel point elle est économe.").to_owned()
        };
        y += k.t.paragraphe(
            Pos2::new(x, y),
            l,
            &phrase,
            Police::Corps,
            k.a.typo.petit,
            c.doux.c(),
        );
        y + 14.0
    }

    /// Le badge de la brique gagnée : il arrive en rebondissant, un reflet le traverse.
    /// Renvoie le bas.
    fn badge(k: &mut Cadre, at: &Atelier, quete: &Quete, x: f32, l: f32, y: f32) -> f32 {
        let c = k.t.c;
        let m = &k.a.mouvement;
        let age = at.fete.map(|t0| (at.horloge - t0) as f32);
        let arrivee = age.map_or(1.0, |a| {
            if m.sobre {
                1.0
            } else {
                Courbe::Rebond.en(a / m.badge.max(0.01))
            }
        });
        let rect = Rect::from_min_size(Pos2::new(x, y), Vec2::new(l, 46.0));
        k.reperer("badge", 0, rect);
        let dessine = Rect::from_center_size(rect.center(), rect.size() * (0.7 + 0.3 * arrivee));
        k.t.p.rect(
            dessine,
            k.a.formes.rayon_bouton + 2.0,
            c.un.alpha(0.10),
            Stroke::new(1.0, c.un.alpha(0.6)),
            StrokeKind::Inside,
        );
        // Le reflet qui traverse le badge, une fois.
        if let Some(a) = age.filter(|a| !m.sobre && *a < m.badge * 1.6) {
            let q = (a / (m.badge * 1.6)).clamp(0.0, 1.0);
            let xr = dessine.left() + dessine.width() * Courbe::Douce.en(q);
            let reflet = Rect::from_min_max(
                Pos2::new((xr - 18.0).max(dessine.left()), dessine.top() + 2.0),
                Pos2::new((xr + 18.0).min(dessine.right()), dessine.bottom() - 2.0),
            );
            if reflet.is_positive() {
                k.t.p
                    .rect_filled(reflet, 8.0, c.perle.alpha(0.25 * (1.0 - q)));
            }
            k.anims.en_mouvement = true;
        }
        if arrivee > 0.5 {
            let icone = if quete.epreuve.ordinateur() {
                Icone::Ecran
            } else {
                Icone::Brique
            };
            k.t.icone(
                icone,
                Pos2::new(dessine.left() + 24.0, dessine.center().y),
                20.0,
                c.un.c(),
            );
            let texte = if matches!(quete.epreuve, crate::quete::Epreuve::Systeme { .. }) {
                tr!("Grand projet construit").to_owned()
            } else if quete.epreuve.ordinateur() {
                tr!("Ordinateur construit").to_owned()
            } else {
                tr!("Brique gagnée : {brique}", brique = quete.brique)
            };
            k.t.texte(
                Pos2::new(dessine.left() + 46.0, dessine.center().y),
                Align2::LEFT_CENTER,
                &k.t.abreger(
                    &texte,
                    Police::CorpsDemi,
                    k.a.typo.corps,
                    dessine.width() - 60.0,
                ),
                Police::CorpsDemi,
                k.a.typo.corps,
                c.texte.c(),
            );
        }
        y + 46.0 + 10.0
    }

    /// La fiche « Comprendre » de la quête : à quoi sert ce qu'on construit, comment ça
    /// marche, sa place dans le tout. Elle se replie d'un clic. Renvoie le bas.
    /// La fiche « Comprendre » de la quête, puis ses indices.
    fn comprendre(&mut self, k: &mut Cadre, cle: &'static str, x: f32, l: f32, y: f32) -> f32 {
        let y = self.comprendre_le_composant(k, cle, x, l, y);
        self.indices(k, cle, x, l, y)
    }

    /// Les indices de la quête, révélés un par un : d'abord la méthode, puis les briques du
    /// corrigé, puis l'expression attendue. Renvoie le bas.
    fn indices(&mut self, k: &mut Cadre, cle: &'static str, x: f32, l: f32, mut y: f32) -> f32 {
        if self.indices.as_ref().is_none_or(|(c, _)| *c != cle) {
            self.indices = Some((cle, logique::indices(cle)));
        }
        let Some((_, tous)) = &self.indices else {
            return y;
        };
        let total = tous.len();
        if total == 0 {
            return y;
        }
        let vus = self.indices_vus.get(cle).copied().unwrap_or(0).min(total);
        let c = k.t.c;
        let debut = y;
        for (n, indice) in tous.iter().take(vus).enumerate() {
            k.t.legende(
                Pos2::new(x, y + 7.0),
                Align2::LEFT_CENTER,
                &tr!("Indice {n} · {titre}", n = n + 1, titre = indice.titre),
                c.accent_clair.c(),
            );
            y += 18.0;
            y += k.t.paragraphe(
                Pos2::new(x, y),
                l,
                &indice.texte,
                Police::Corps,
                k.a.typo.corps,
                c.texte.alpha(0.88),
            ) + 10.0;
        }
        if vus > 0 {
            k.t.p.line_segment(
                [Pos2::new(x - 10.0, debut), Pos2::new(x - 10.0, y - 10.0)],
                Stroke::new(2.0, c.accent_clair.alpha(0.35)),
            );
        }
        if vus < total {
            let tete = Rect::from_min_size(Pos2::new(x - 6.0, y), Vec2::new(l + 12.0, 32.0));
            k.reperer("indice", 0, tete);
            let touche = k.toucher(
                tete,
                egui::Id::unique("indice-suivant"),
                tr!("Bloqué ? Chaque indice en dit un peu plus. Essaie d'abord avec le précédent."),
                Sense::click(),
            );
            if touche.clic {
                self.indices_vus.insert(cle, vus + 1);
            }
            k.t.element(touche.rect, k.a.formes.rayon_bouton, touche.chaud, false);
            k.t.icone(
                Icone::Lampe,
                Pos2::new(tete.left() + 18.0, tete.center().y),
                15.0,
                c.accent_clair.c(),
            );
            let texte = if vus == 0 {
                tr!("Un indice · {total} en réserve", total = total)
            } else {
                tr!(
                    "Indice suivant · {v} sur {total}",
                    total = total,
                    v = vus + 1
                )
            };
            k.t.texte(
                Pos2::new(tete.left() + 34.0, tete.center().y),
                Align2::LEFT_CENTER,
                &texte,
                Police::CorpsDemi,
                k.a.typo.corps,
                c.texte.c(),
            );
            y += 32.0 + 12.0;
        }
        y
    }

    fn comprendre_le_composant(
        &mut self,
        k: &mut Cadre,
        cle: &str,
        x: f32,
        l: f32,
        mut y: f32,
    ) -> f32 {
        let Some(s) = savoir::savoir(cle) else {
            return y;
        };
        let c = k.t.c;
        let tete = Rect::from_min_size(Pos2::new(x - 6.0, y), Vec2::new(l + 12.0, 32.0));
        k.reperer("comprendre", 0, tete);
        let touche = k.toucher(
            tete,
            egui::Id::unique("comprendre"),
            if self.comprendre_ouvert {
                tr!("Replier")
            } else {
                tr!("Déplier : à quoi ça sert, comment ça marche, le savais-tu ?")
            },
            Sense::click(),
        );
        if touche.clic {
            self.comprendre_ouvert = !self.comprendre_ouvert;
        }
        let ouvert = k.anims.vers(
            egui::Id::unique("comprendre-ouvert"),
            if self.comprendre_ouvert { 1.0 } else { 0.0 },
            k.dt,
            k.a.mouvement.lissage_ui,
        );
        k.t.element(touche.rect, k.a.formes.rayon_bouton, touche.chaud, false);
        k.t.icone(
            Icone::Infos,
            Pos2::new(tete.left() + 18.0, tete.center().y),
            15.0,
            c.accent_clair.c(),
        );
        k.t.texte(
            Pos2::new(tete.left() + 34.0, tete.center().y),
            Align2::LEFT_CENTER,
            tr!("Comprendre ce composant"),
            Police::CorpsDemi,
            k.a.typo.corps,
            c.texte.c(),
        );
        // Le chevron tourne d'un quart de tour à l'ouverture.
        let centre = Pos2::new(tete.right() - 18.0, tete.center().y);
        let angle = std::f32::consts::FRAC_PI_2 * ouvert;
        let (ca, sa) = (angle.cos(), angle.sin());
        let pt = |a: f32, b: f32| Pos2::new(centre.x + a * ca - b * sa, centre.y + a * sa + b * ca);
        k.t.p
            .line_segment([pt(-3.0, -5.0), pt(3.0, 0.0)], Stroke::new(1.8, c.doux.c()));
        k.t.p
            .line_segment([pt(3.0, 0.0), pt(-3.0, 5.0)], Stroke::new(1.8, c.doux.c()));
        y += 32.0 + 8.0;
        if ouvert < 0.02 {
            return y + 4.0;
        }
        let debut = y;
        for (titre, texte) in s.rubriques() {
            k.t.legende(
                Pos2::new(x, y + 7.0),
                Align2::LEFT_CENTER,
                &titre.to_uppercase(),
                c.accent_clair.alpha(ouvert),
            );
            y += 18.0;
            y += k.t.paragraphe(
                Pos2::new(x, y),
                l,
                texte,
                Police::Corps,
                k.a.typo.corps,
                c.texte.alpha(0.88 * ouvert),
            ) + 10.0;
        }
        // Le filet à gauche qui tient les paragraphes ensemble.
        k.t.p.line_segment(
            [Pos2::new(x - 10.0, debut), Pos2::new(x - 10.0, y - 10.0)],
            Stroke::new(2.0, c.accent_clair.alpha(0.35 * ouvert)),
        );
        debut + (y - debut) * ouvert + 6.0
    }

    /// Pour une petite quête : la table de vérité, cochée cas par cas. Renvoie le bas.
    #[allow(clippy::too_many_arguments)]
    fn cas_de_la_quete(
        &mut self,
        k: &mut Cadre,
        at: &mut Atelier,
        quete: &Quete,
        cas: &[Vec<bool>],
        x: f32,
        l: f32,
        mut y: f32,
    ) -> f32 {
        let c = k.t.c;
        let total = cas.len();
        // Barre d'avancement.
        let part = k.anims.vers(
            egui::Id::unique("quete-avance"),
            quete.vus.len() as f32 / total.max(1) as f32,
            k.dt,
            k.a.mouvement.lissage_barre,
        );
        let barre = Rect::from_min_size(Pos2::new(x, y), Vec2::new(l, 4.0));
        k.t.p.rect_filled(barre, 2.0, c.carte_2.c());
        if part > 0.005 {
            k.t.p.rect_filled(
                Rect::from_min_size(barre.min, Vec2::new(l * part, 4.0)),
                2.0,
                c.un.c(),
            );
        }
        y += 4.0 + 14.0;

        // Table de vérité : une ligne par cas, cochée quand le circuit l'a montré juste.
        let colonnes: Vec<&str> = quete
            .entrees
            .iter()
            .chain(quete.sorties.iter())
            .map(|b| b.nom.as_str())
            .collect();
        let pas = (l - 44.0) / colonnes.len() as f32;
        let centre = |i: usize| x + 6.0 + pas * (i as f32 + 0.5);
        let n_entrees = quete.entrees.len();
        for (i, nom) in colonnes.iter().enumerate() {
            k.t.texte(
                Pos2::new(centre(i), y + 8.0),
                Align2::CENTER_CENTER,
                &crate::langue::nom_affiche(nom),
                Police::MonoGras,
                k.a.typo.petit,
                if i < n_entrees {
                    c.discret.c()
                } else {
                    c.accent_clair.c()
                },
            );
        }
        y += 20.0;
        k.t.separateur(x - 6.0, x + l + 6.0, y - 2.0);
        // Un trait vertical sépare les entrées (ce que tu règles) des sorties (ce que tu
        // construis).
        if n_entrees > 0 && n_entrees < colonnes.len() {
            let xs = x + 6.0 + pas * n_entrees as f32;
            k.t.p.line_segment(
                [
                    Pos2::new(xs, y - 22.0),
                    Pos2::new(xs, y + 32.0 * cas.len() as f32),
                ],
                Stroke::new(1.0, c.trait_.c()),
            );
        }
        y += 4.0;
        let courant = quete.cas_courant(&at.doc, &at.session);
        for (i, ligne) in cas.iter().enumerate() {
            let rect = Rect::from_min_size(Pos2::new(x - 6.0, y), Vec2::new(l + 12.0, 30.0));
            k.reperer("cas", i, rect);
            let vu = quete.vus.contains(ligne);
            let ici = courant.as_ref() == Some(ligne);
            let touche = k.toucher(
                rect,
                egui::Id::unique(("cas", ligne.clone())),
                tr!("Régler les interrupteurs sur ce cas"),
                Sense::click(),
            );
            let neuf =
                at.cas_neuf
                    .as_ref()
                    .filter(|(cas, _)| cas == ligne)
                    .map_or(0.0, |(_, t0)| {
                        (1.0 - (at.horloge - t0) as f32 / k.a.mouvement.cas_valide.max(0.01))
                            .clamp(0.0, 1.0)
                    });
            if ici || touche.chaud > 0.02 || neuf > 0.0 {
                let teinte = if vu { c.un } else { c.accent_clair };
                // Le cas réglé en ce moment : ligne enfoncée, bordure de sa couleur.
                let fond = if ici {
                    c.enfonce.c()
                } else {
                    c.survol.alpha(touche.chaud)
                };
                k.t.p.rect(
                    rect,
                    k.a.formes.rayon_bouton,
                    fond.lerp_to_gamma(teinte.c(), 0.25 * neuf),
                    Stroke::new(1.0, teinte.alpha(if ici { 0.8 } else { 0.0 })),
                    StrokeKind::Inside,
                );
            }
            let attendu = quete.attendu(ligne).unwrap_or_default();
            for (i, v) in ligne.iter().enumerate() {
                k.t.texte(
                    Pos2::new(centre(i), rect.center().y),
                    Align2::CENTER_CENTER,
                    if *v { "1" } else { "0" },
                    Police::MonoGras,
                    k.a.typo.corps,
                    c.texte.c(),
                );
            }
            for (j, v) in attendu.iter().enumerate() {
                let (texte, couleur) = if vu {
                    (if *v { "1" } else { "0" }, c.texte.c())
                } else {
                    ("?", c.doux.c())
                };
                k.t.texte(
                    Pos2::new(centre(ligne.len() + j), rect.center().y),
                    Align2::CENTER_CENTER,
                    texte,
                    Police::MonoGras,
                    k.a.typo.corps,
                    couleur,
                );
            }
            let (icone, couleur) = if vu {
                (Icone::Coche, c.un.c())
            } else {
                (Icone::Rond, c.discret.alpha(0.8))
            };
            k.t.icone(
                icone,
                Pos2::new(rect.right() - 20.0, rect.center().y),
                15.0 * (1.0 + 0.35 * neuf),
                couleur,
            );
            if touche.clic {
                // Règle les interrupteurs sur ce cas.
                for (b, voulu) in quete.entrees.iter().zip(ligne) {
                    let entree = at
                        .doc
                        .elements
                        .iter()
                        .find(|e| e.genre == Genre::Entree && e.nom == b.nom)
                        .map(|e| e.id);
                    let voulu = u64::from(*voulu);
                    if let Some(id) = entree.filter(|id| at.session.entrees.get(id) != Some(&voulu))
                    {
                        at.basculer(id);
                    }
                }
            }
            y += 32.0;
        }
        y + 8.0
    }

    /// Le bouton Vérifier, son verdict, et la quête suivante une fois réussie. Renvoie le bas.
    #[allow(clippy::too_many_arguments)]
    fn verification(
        &mut self,
        k: &mut Cadre,
        at: &mut Atelier,
        quete: &Quete,
        x: f32,
        l: f32,
        mut y: f32,
        demandes: &mut Demandes,
    ) -> f32 {
        let c = k.t.c;
        let rect = Rect::from_min_size(Pos2::new(x, y), Vec2::new(l, 36.0));
        k.reperer("verifier", 0, rect);
        let libelle = if quete.verdict.is_some() {
            tr!("Vérifier de nouveau")
        } else {
            tr!("Vérifier")
        };
        if k.bouton_texte(rect, "verifier-quete", libelle, true) {
            at.verifier_la_quete();
        }
        y += 36.0 + 10.0;
        // Le verdict affiché est celui de la quête de l'atelier, à jour même juste après le clic.
        let verdict = at.quete.as_ref().and_then(|q| q.verdict.clone());
        if let Some(v) = verdict {
            let couleur = if v.reussi { c.un } else { c.danger };
            let marque = Pos2::new(x + 9.0, y + 10.0);
            if v.reussi {
                k.t.icone(Icone::Coche, marque, 16.0, couleur.c());
            } else {
                k.t.p.circle_filled(marque, 4.5, couleur.c());
            }
            y += k.t.paragraphe(
                Pos2::new(x + 26.0, y),
                l - 26.0,
                &v.texte,
                Police::Corps,
                k.a.typo.corps,
                c.texte.c(),
            ) + 12.0;
        }
        // Réussie à l'instant, par ce clic : le bouton de la suite n'a pas encore été dessiné
        // plus haut à cette image, il l'est ici ; dès la suivante, il est sous les félicitations.
        let reussie = at.quete.as_ref().is_some_and(Quete::reussie);
        if reussie && !quete.reussie() {
            y = self.quete_suivante(k, at, quete, x, l, y, demandes);
        }
        y
    }

    /// Le bouton « Quête suivante » d'une quête réussie : il arrive avec la fête, puis bat
    /// doucement pour inviter à continuer. Renvoie le bas.
    #[allow(clippy::too_many_arguments)]
    fn quete_suivante(
        &mut self,
        k: &mut Cadre,
        at: &Atelier,
        quete: &Quete,
        x: f32,
        l: f32,
        y: f32,
        demandes: &mut Demandes,
    ) -> f32 {
        let Some(suivante) = Parcours::suivante(quete.cle) else {
            return y;
        };
        let m = &k.a.mouvement;
        let arrivee = at.fete.map_or(1.0, |t0| {
            m.avancement(at.horloge - t0, m.apparition, m.courbe_apparition)
        });
        let rect = Rect::from_min_size(Pos2::new(x, y), Vec2::new(l, 36.0));
        k.reperer("quete-suivante", 0, rect);
        let texte = k.t.abreger(
            &tr!(
                "Quête suivante : {titre}",
                titre = crate::langue::traduire(suivante.titre)
            ),
            Police::CorpsDemi,
            k.a.typo.corps,
            l - 24.0,
        );
        if !m.sobre {
            let c = k.t.c;
            let pouls = m.pouls(at.horloge, 1.0);
            k.t.p.rect_stroke(
                rect.expand(2.0 + 5.0 * pouls),
                k.a.formes.rayon_bouton + 2.0 + 5.0 * pouls,
                Stroke::new(1.5, c.accent_clair.alpha(0.55 * (1.0 - pouls))),
                StrokeKind::Outside,
            );
            k.anims.en_mouvement = true;
        }
        let dessine = Rect::from_center_size(rect.center(), rect.size() * (0.9 + 0.1 * arrivee));
        if k.bouton_texte(dessine, "quete-suivante", &texte, true) {
            demandes.ouvrir = Some(Ouverture::Quete(suivante.cle.to_owned()));
        }
        y + 36.0 + 10.0
    }

    /// Les lignes de la fiche de ce qui est sélectionné.
    fn fiche(at: &Atelier) -> Vec<Rang> {
        let mut f = Vec::new();
        let texte = |nom: &str, valeur: String| Rang::Texte(nom.to_owned(), valeur);
        match &at.selection {
            Some(Selection::Elements(ids)) if ids.len() == 1 => {
                let id = ids[0];
                let Some(e) = at.doc.element(id) else {
                    return f;
                };
                let lue = |nom: String, i: usize| {
                    let fil = Fil {
                        puits: id,
                        entree: i,
                    };
                    let signal = at.doc.source_de(fil).map_or(tr!("libre").to_owned(), |s| {
                        crate::langue::nom_affiche(&at.doc.nom_signal(s)).into_owned()
                    });
                    Rang::Signal(nom, signal, at.lecture_lue(fil))
                };
                let sortie = |nom: String, j: usize| {
                    let s = Source::nouvelle(id, j);
                    Rang::Signal(
                        nom,
                        crate::langue::nom_affiche(&at.doc.nom_signal(s)).into_owned(),
                        at.lecture(s),
                    )
                };
                f.push(Rang::Saisie(
                    tr!("Nom").to_owned(),
                    Champ::Nom(id),
                    crate::langue::nom_affiche(&e.nom).into_owned(),
                    tr!("Clique pour renommer"),
                ));
                match e.genre {
                    Genre::Nand => {
                        f.push(texte(tr!("Délai"), tr!("1 pas").to_owned()));
                        for (i, b) in e.broches_e.iter().enumerate() {
                            f.push(lue(tr!("Entrée {nom}", nom = b.nom), i));
                        }
                        f.push(sortie(tr!("Sortie").to_owned(), 0));
                        let attend = at.vue_t.is_none()
                            && at.session.cible(&at.doc, id) != at.session.valeur(Source::de(id));
                        f.push(texte(
                            tr!("État"),
                            if attend {
                                tr!("va basculer")
                            } else {
                                tr!("au repos")
                            }
                            .to_owned(),
                        ));
                    }
                    Genre::Entree if e.param > 0 => {
                        f.push(Rang::Saisie(
                            tr!("Durée").to_owned(),
                            Champ::Duree(id),
                            e.param.to_string(),
                            tr!("Nombre de pas pendant lesquels il vaut 1 au démarrage"),
                        ));
                        f.push(sortie(tr!("Valeur").to_owned(), 0));
                        f.push(texte(
                            tr!("Pour le rejouer"),
                            tr!("reviens à t = 0").to_owned(),
                        ));
                    }
                    Genre::Entree if e.est_poussoir() => {
                        f.push(sortie(tr!("Valeur").to_owned(), 0));
                        f.push(texte(
                            tr!("Pour appuyer"),
                            tr!("garde le bouton de la souris enfoncé").to_owned(),
                        ));
                    }
                    Genre::Entree => {
                        f.push(Rang::Saisie(
                            tr!("Bits").to_owned(),
                            Champ::Largeur(id),
                            e.broches_s[0].largeur.to_string(),
                            tr!("De 1 à 64 : au-delà de 1, c'est un bus"),
                        ));
                        f.push(sortie(tr!("Valeur").to_owned(), 0));
                        f.push(texte(tr!("Pour changer"), tr!("clique dessus").to_owned()));
                    }
                    Genre::Sortie => {
                        f.push(Rang::Saisie(
                            tr!("Bits").to_owned(),
                            Champ::Largeur(id),
                            e.broches_e[0].largeur.to_string(),
                            tr!("De 1 à 64 : au-delà de 1, c'est un bus"),
                        ));
                        f.push(lue(tr!("Branchée sur").to_owned(), 0));
                        if e.est_afficheur() {
                            f.push(texte(
                                tr!("Segments"),
                                tr!("bit 0 = a en haut, … bit 6 = g au milieu").to_owned(),
                            ));
                        }
                    }
                    Genre::Brique => {
                        f.push(Rang::Saisie(
                            tr!("Brique").to_owned(),
                            Champ::Modele(e.modele.clone()),
                            crate::langue::nom_affiche(&e.modele).into_owned(),
                            tr!("Renomme la brique, partout où elle est posée"),
                        ));
                        let description = at
                            .projet
                            .modele(&e.modele)
                            .map(|m| crate::langue::traduire_texte(&m.description).into_owned())
                            .unwrap_or_default();
                        f.push(Rang::Saisie(
                            tr!("Description").to_owned(),
                            Champ::Description(e.modele.clone()),
                            if description.is_empty() {
                                "—".to_owned()
                            } else {
                                description
                            },
                            tr!("Ce que fait la brique, en une phrase"),
                        ));
                        f.push(texte(tr!("Utilisée dans"), at.usages_du_modele(&e.modele)));
                        f.push(texte(tr!("Pour l'ouvrir"), "double-clic".to_owned()));
                        f.push(texte(tr!("Sa documentation"), tr!("F2").to_owned()));
                        for (i, b) in e.broches_e.iter().enumerate().take(16) {
                            f.push(lue(tr!("Entrée {nom}", nom = b.nom), i));
                        }
                        for (j, b) in e.broches_s.iter().enumerate().take(16) {
                            let s = Source::nouvelle(id, j);
                            f.push(Rang::Signal(
                                tr!("Sortie {nom}", nom = b.nom),
                                String::new(),
                                at.lecture(s),
                            ));
                        }
                    }
                    Genre::Horloge => {
                        f.push(Rang::Saisie(
                            tr!("Demi-période").to_owned(),
                            Champ::Duree(id),
                            e.param.to_string(),
                            tr!("Nombre de pas entre deux bascules"),
                        ));
                        f.push(sortie(tr!("Sortie").to_owned(), 0));
                    }
                    Genre::Ecran => {
                        f.push(texte(
                            tr!("Caractères écrits"),
                            at.session.ecran(id).chars().count().to_string(),
                        ));
                        for (i, b) in e.broches_e.iter().enumerate() {
                            f.push(lue(tr!("Entrée {nom}", nom = b.nom), i));
                        }
                    }
                    Genre::Clavier => {
                        f.push(Rang::Saisie(
                            tr!("Tenue d'une touche").to_owned(),
                            Champ::Duree(id),
                            e.param.to_string(),
                            tr!("Nombre de pas pendant lesquels le code de la touche est présenté"),
                        ));
                        f.push(texte(
                            tr!("Touches en attente"),
                            at.session.touches_en_attente(id).to_string(),
                        ));
                        f.push(sortie(tr!("Sortie").to_owned(), 0));
                        f.push(texte(
                            tr!("Pour taper"),
                            tr!("sélectionne-le, puis tape").to_owned(),
                        ));
                    }
                    Genre::Decoupe => {
                        let morceaux: Vec<String> = e
                            .plages
                            .iter()
                            .map(|(a, b)| {
                                if a == b {
                                    a.to_string()
                                } else {
                                    format!("{a}:{b}")
                                }
                            })
                            .collect();
                        f.push(Rang::Saisie(
                            tr!("Morceaux").to_owned(),
                            Champ::Morceaux(id),
                            morceaux.join(" "),
                            tr!("Par exemple 0:3 4:7, ou 0 1 2 3 pour des bits seuls"),
                        ));
                        f.push(lue(tr!("Bus d'entrée").to_owned(), 0));
                    }
                    Genre::Jonction => {
                        let largeurs: Vec<String> =
                            e.broches_e.iter().map(|b| b.largeur.to_string()).collect();
                        f.push(Rang::Saisie(
                            tr!("Entrées").to_owned(),
                            Champ::Morceaux(id),
                            largeurs.join(" "),
                            tr!("La largeur de chaque entrée, par exemple 4 4"),
                        ));
                        f.push(sortie(tr!("Bus de sortie").to_owned(), 0));
                    }
                }
            }
            Some(Selection::Elements(ids)) => {
                let compte = |genre: Genre| {
                    ids.iter()
                        .filter(|id| at.doc.element(**id).is_some_and(|e| e.genre == genre))
                        .count()
                };
                f.push(texte(tr!("Éléments"), ids.len().to_string()));
                f.push(texte(tr!("Portes NAND"), compte(Genre::Nand).to_string()));
                if compte(Genre::Brique) > 0 {
                    f.push(texte(tr!("Briques"), compte(Genre::Brique).to_string()));
                }
                f.push(texte(tr!("Pour en faire une brique"), "Ctrl+G".to_owned()));
            }
            Some(Selection::Fil(fil)) => {
                if let (Some(source), Some(puits)) =
                    (at.doc.source_de(*fil), at.doc.element(fil.puits))
                {
                    f.push(Rang::Signal(
                        tr!("Signal").to_owned(),
                        crate::langue::nom_affiche(&at.doc.nom_signal(source)).into_owned(),
                        at.lecture(source),
                    ));
                    if let Some(e) = at.doc.element(source.id) {
                        let depart = match e.broches_s.get(source.sortie) {
                            Some(b) if e.broches_s.len() > 1 => format!(
                                "{} · {}",
                                crate::langue::nom_affiche(&e.nom),
                                crate::langue::nom_affiche(&b.nom)
                            ),
                            _ => crate::langue::nom_affiche(&e.nom).into_owned(),
                        };
                        f.push(texte(tr!("Part de"), depart));
                    }
                    let arrivee = match puits.broches_e.get(fil.entree) {
                        Some(b) if puits.broches_e.len() > 1 => {
                            format!(
                                "{} · {}",
                                crate::langue::nom_affiche(&puits.nom),
                                crate::langue::nom_affiche(&b.nom)
                            )
                        }
                        _ => crate::langue::nom_affiche(&puits.nom).into_owned(),
                    };
                    f.push(texte(tr!("Arrive sur"), arrivee));
                    let largeur = at.doc.largeur_source(source).unwrap_or(1);
                    if largeur > 1 {
                        f.push(texte(tr!("Largeur"), format!("{largeur} bits")));
                    }
                }
            }
            None => {}
        }
        f
    }

    fn infos(&mut self, k: &mut Cadre, at: &mut Atelier, d: &Disposition, ouverture: f32) {
        let r = d.infos;
        let Some(t) = Self::panneau(k, r, "infos", ouverture) else {
            return;
        };
        let fond = std::mem::replace(&mut k.t, t);
        let c = k.t.c;
        if k.entete(r, tr!("Infos"), "fermer-infos") {
            at.volets.infos = false;
        }
        let pad = k.a.formes.rembourrage + 2.0;
        let (x, l) = (r.left() + pad, r.width() - 2.0 * pad);
        // Le contenu défile sous le titre quand il dépasse la hauteur du panneau.
        let zone = Rect::from_min_max(
            Pos2::new(r.left(), r.top() + ENTETE + 1.0),
            Pos2::new(r.right(), r.bottom() - 8.0),
        );
        k.t = k.t.sur(1.0, zone);
        k.decoupe = Some(zone);
        let mut y = r.top() + ENTETE + 12.0 - self.defilement_infos;

        // Sélection.
        y += k.rubrique(
            x,
            y,
            l,
            tr!("Sélection"),
            &k.t.abreger(
                &at.nom_selection().unwrap_or_default(),
                Police::Mono,
                k.a.typo.legende,
                l - 90.0,
            ),
        );
        let fiche = Self::fiche(at);
        if fiche.is_empty() {
            y += k.t.paragraphe(
                Pos2::new(x, y + 2.0),
                l,
                tr!("Clique sur une porte, un fil, une brique, un interrupteur ou une lampe pour voir ses détails."),
                Police::Corps,
                k.a.typo.petit,
                c.discret.c(),
            ) + 12.0;
        }
        let mut saisies = Vec::new();
        for rang in fiche {
            let milieu = y + 13.0;
            let nom = match &rang {
                Rang::Texte(nom, _) | Rang::Signal(nom, ..) | Rang::Saisie(nom, ..) => nom,
            };
            let fin_nom =
                k.t.texte(
                    Pos2::new(x, milieu),
                    Align2::LEFT_CENTER,
                    &k.t.abreger(nom, Police::Corps, k.a.typo.corps, l * 0.62),
                    Police::Corps,
                    k.a.typo.corps,
                    c.doux.c(),
                )
                .right();
            match rang {
                Rang::Texte(_, valeur) => {
                    k.t.texte(
                        Pos2::new(x + l, milieu),
                        Align2::RIGHT_CENTER,
                        &k.t.abreger(
                            &valeur,
                            Police::Mono,
                            k.a.typo.corps,
                            x + l - fin_nom - 12.0,
                        ),
                        Police::Mono,
                        k.a.typo.corps,
                        c.texte.c(),
                    );
                }
                Rang::Signal(_, signal, lecture) => {
                    let mut droite = x + l;
                    if lecture.largeur <= 1 {
                        k.t.pastille(
                            Pos2::new(droite - 9.0, milieu),
                            symbole(lecture.valeur()),
                            18.0,
                        );
                        droite -= 26.0;
                    } else {
                        let couleur = if lecture.flous != 0 {
                            c.x
                        } else if lecture.uns != 0 {
                            c.un
                        } else {
                            c.z
                        };
                        droite =
                            k.t.texte(
                                Pos2::new(droite, milieu),
                                Align2::RIGHT_CENTER,
                                &lecture.texte(),
                                Police::MonoGras,
                                k.a.typo.corps,
                                couleur.c(),
                            )
                            .left()
                                - 10.0;
                    }
                    k.t.texte(
                        Pos2::new(droite, milieu),
                        Align2::RIGHT_CENTER,
                        &k.t.abreger(
                            &signal,
                            Police::Mono,
                            k.a.typo.corps,
                            droite - fin_nom - 12.0,
                        ),
                        Police::Mono,
                        k.a.typo.corps,
                        c.texte.c(),
                    );
                }
                Rang::Saisie(_, champ, valeur, bulle) => {
                    let largeur = (x + l - fin_nom - 12.0).min(l * 0.56).max(60.0);
                    let rect = Rect::from_min_size(
                        Pos2::new(x + l - largeur, milieu - 12.0),
                        Vec2::new(largeur, 24.0),
                    );
                    if let Some(texte) = self.champ(
                        k,
                        rect,
                        champ.clone(),
                        &valeur,
                        Police::Mono,
                        k.a.typo.corps,
                        bulle,
                    ) {
                        saisies.push((champ, texte));
                    }
                }
            }
            y += 28.0;
        }
        for (champ, texte) in saisies {
            Self::appliquer(at, k.a, champ, &texte);
        }
        y += 12.0;

        // Circuit.
        y += k.rubrique(x, y, l, tr!("Circuit"), "");
        let mut curseur = Pos2::new(x, y);
        let circuit = self.connaitre(at).clone();
        let portes = circuit.portes;
        let total = at.session.nb_portes();
        let pluriel = |n: usize, un: &'static str, plusieurs: &'static str| {
            if crate::langue::s(n).is_empty() {
                un
            } else {
                plusieurs
            }
        };
        if total == portes {
            k.puce(
                &mut curseur,
                x,
                x + l,
                &portes.to_string(),
                pluriel(portes, tr!("porte NAND"), tr!("portes NAND")),
                None,
            );
        } else {
            k.puce(
                &mut curseur,
                x,
                x + l,
                &total.to_string(),
                pluriel(total, tr!("porte NAND en tout"), tr!("portes NAND en tout")),
                None,
            );
            k.puce(
                &mut curseur,
                x,
                x + l,
                &portes.to_string(),
                tr!("sur ce plan"),
                None,
            );
        }
        k.puce(
            &mut curseur,
            x,
            x + l,
            &circuit.signaux.to_string(),
            if crate::langue::s(circuit.signaux).is_empty() {
                tr!("signal")
            } else {
                tr!("signaux")
            },
            None,
        );
        match circuit.profondeur {
            Profondeur::Pas(n) => k.puce(
                &mut curseur,
                x,
                x + l,
                &n.to_string(),
                tr!("pas au plus long"),
                None,
            ),
            Profondeur::Boucle => {
                k.puce(&mut curseur, x, x + l, "", tr!("contient une boucle"), None)
            }
            Profondeur::Briques => k.puce(
                &mut curseur,
                x,
                x + l,
                &circuit.niveaux.to_string(),
                pluriel(
                    circuit.niveaux,
                    tr!("niveau de briques"),
                    tr!("niveaux de briques"),
                ),
                None,
            ),
        }
        y = curseur.y + 26.0 + 16.0;
        if let Some(erreur) = at.session.erreur() {
            y += k.t.paragraphe(
                Pos2::new(x, y - 4.0),
                l,
                &tr!(
                    "Ce circuit ne peut pas être simulé : {erreur}",
                    erreur = erreur
                ),
                Police::Corps,
                k.a.typo.petit,
                c.danger.c(),
            ) + 8.0;
        }

        // Sondes : la valeur de chaque signal à l'instant affiché.
        y += k.rubrique(x, y, l, tr!("Sondes"), &format!("t = {}", at.t_affiche()));
        let mut curseur = Pos2::new(x, y);
        let prises = &circuit.sondes;
        for (source, nom) in prises.iter().take(SONDES) {
            let lecture = at.lecture(*source);
            let nom = k.t.abreger(
                &crate::langue::nom_affiche(nom),
                Police::Mono,
                k.a.typo.corps,
                l - 90.0,
            );
            if lecture.largeur <= 1 {
                k.puce(
                    &mut curseur,
                    x,
                    x + l,
                    "",
                    &nom,
                    Some(symbole(lecture.valeur())),
                );
            } else {
                k.puce(
                    &mut curseur,
                    x,
                    x + l,
                    "",
                    &format!("{nom} = {}", lecture.texte()),
                    None,
                );
            }
        }
        y = curseur.y + 26.0 + 14.0;
        let mut resume = if at.vue_t.is_some() {
            tr!("Tu regardes un instant passé. Les valeurs sont celles de cet instant.").to_owned()
        } else if at.session.stable() {
            tr!("Tout est au repos.").to_owned()
        } else if at.session.a_une_horloge() {
            tr!("Une horloge bat : ce circuit ne s'arrête jamais de lui-même.").to_owned()
        } else {
            match at.session.en_attente(&at.doc).len() {
                0 => tr!("Le courant se propage dans les briques.").to_owned(),
                1 => tr!("Une porte n'a pas encore pris sa valeur.").to_owned(),
                n => tr!("{n} portes n'ont pas encore pris leur valeur.", n = n),
            }
        };
        if prises.len() > SONDES {
            resume.push_str(&tr!(" Seuls les {SONDES} premiers signaux sont montrés : sélectionne un élément pour lire les siens.", SONDES = SONDES));
        }
        y += k.t.paragraphe(
            Pos2::new(x, y),
            l,
            &resume,
            Police::Corps,
            k.a.typo.petit,
            c.discret.c(),
        );
        let hauteur = y + self.defilement_infos - zone.top() + 6.0;
        self.defilement_infos = k.defiler(r, zone, hauteur, self.defilement_infos, ouverture);
        k.decoupe = None;
        k.t = fond;
    }

    fn temps(&mut self, k: &mut Cadre, at: &mut Atelier, d: &Disposition, ouverture: f32) {
        let r = d.temps;
        let circuit = self.connaitre(at);
        let toutes = circuit.signaux;
        let sources: Vec<(Source, String)> = circuit
            .sondes
            .iter()
            .take(LIGNES_CHRONO)
            .map(|(s, n)| (*s, crate::langue::nom_affiche(n).into_owned()))
            .collect();
        let ligne = k.a.formes.ligne_chrono;
        self.contenus.temps = 56.0 + 20.0 + sources.len().max(1) as f32 * ligne + 18.0;
        let Some(t) = Self::panneau(k, r, "temps", ouverture) else {
            return;
        };
        let fond = std::mem::replace(&mut k.t, t);
        let c = k.t.c;
        if k.entete(r, tr!("Chronogramme"), "fermer-temps") {
            at.volets.temps = false;
        }
        let rapide = at.rapide(k.a).is_some();
        if r.width() > 620.0 {
            let conseil = if rapide {
                tr!("En vitesse rapide, le chronogramme repart à chaque image : ralentis pour suivre les signaux").to_owned()
            } else if toutes > sources.len() {
                tr!("Les {len} premiers signaux sur {toutes} · clique sur un instant pour remonter le temps", toutes = toutes, len = sources.len())
            } else {
                tr!("Clique sur un instant pour remonter le temps").to_owned()
            };
            k.t.texte(
                Pos2::new(r.right() - 48.0, r.top() + ENTETE / 2.0),
                Align2::RIGHT_CENTER,
                &k.t.abreger(&conseil, Police::Corps, k.a.typo.petit, r.width() - 260.0),
                Police::Corps,
                k.a.typo.petit,
                c.doux.c(),
            );
        }
        let largeur_journal = if r.width() > 760.0 { 250.0 } else { 0.0 };
        let x = r.left() + k.a.formes.rembourrage + 2.0;
        let y_axe = r.top() + ENTETE + 10.0;
        let y0 = y_axe + 20.0;
        let ondes = Rect::from_min_max(
            Pos2::new(x + 96.0, y0),
            Pos2::new(
                r.right() - 16.0 - largeur_journal - if largeur_journal > 0.0 { 24.0 } else { 0.0 },
                y0 + sources.len().max(1) as f32 * ligne,
            ),
        );

        // Fenêtre de temps : elle glisse pour garder l'instant courant visible.
        let n = k.a.formes.instants_chrono as f32;
        let cellule = ondes.width() / n;
        let t_courant = at.session.t;
        let premier = at.session.historique.front().map_or(0, |i| i.t);
        let debut_vise = (t_courant as f32 - (n - 5.0)).max(premier as f32);
        let cle_debut = egui::Id::unique("chrono-debut");
        let mut debut = k
            .anims
            .vers(cle_debut, debut_vise, k.dt, k.a.mouvement.lissage_chrono);
        if (debut - debut_vise).abs() > 4.0 * n {
            // Un saut dans le temps (vitesse rapide, remise à zéro) : on n'y glisse pas.
            k.anims.placer(cle_debut, debut_vise);
            debut = debut_vise;
        }
        let x_de = |t: f32| ondes.left() + (t - debut) * cellule;
        let dessin = k.t.sur(1.0, ondes.expand2(Vec2::new(0.0, 22.0)));

        k.t.p.rect(
            ondes,
            k.a.formes.rayon_petit,
            c.fond.alpha(0.6),
            Stroke::new(1.0, c.trait_.c()),
            StrokeKind::Inside,
        );
        let t_min = debut.floor().max(0.0) as u64;
        let t_max = (debut + n).ceil() as u64;
        for t in t_min..=t_max {
            let xg = x_de(t as f32);
            dessin.p.line_segment(
                [Pos2::new(xg, ondes.top()), Pos2::new(xg, ondes.bottom())],
                Stroke::new(1.0, c.trait_.c()),
            );
            if t % 2 == 0 {
                let ici = t == at.t_affiche();
                dessin.texte(
                    Pos2::new(xg + cellule / 2.0, y_axe + 8.0),
                    Align2::CENTER_CENTER,
                    &t.to_string(),
                    Police::Mono,
                    k.a.typo.legende,
                    if ici {
                        c.accent_clair.c()
                    } else {
                        c.discret.c()
                    },
                );
            }
        }
        for (rang, (source, nom)) in sources.iter().enumerate() {
            let haut = ondes.top() + rang as f32 * ligne;
            let milieu = haut + ligne / 2.0;
            let lecture = at.lecture(*source);
            let bus = lecture.largeur > 1;
            k.t.texte(
                Pos2::new(x + 6.0, milieu),
                Align2::LEFT_CENTER,
                &k.t.abreger(
                    nom,
                    Police::Mono,
                    k.a.typo.petit,
                    if bus { 46.0 } else { 54.0 },
                ),
                Police::Mono,
                k.a.typo.petit,
                c.doux.c(),
            );
            if bus {
                k.t.texte(
                    Pos2::new(x + 88.0, milieu),
                    Align2::RIGHT_CENTER,
                    &k.t.abreger(&lecture.texte(), Police::MonoGras, k.a.typo.petit, 34.0),
                    Police::MonoGras,
                    k.a.typo.petit,
                    c.accent_clair.c(),
                );
            } else {
                k.t.pastille(Pos2::new(x + 76.0, milieu), symbole(lecture.valeur()), 17.0);
            }
            let (y_haut, y_bas) = (haut + 4.0, haut + ligne - 4.0);
            for t in t_min..=t_max.min(t_courant) {
                let Some(l) = at.session.instant_a(t).and_then(|i| i.lecture(*source)) else {
                    continue;
                };
                let avant = t
                    .checked_sub(1)
                    .and_then(|p| at.session.instant_a(p))
                    .and_then(|i| i.lecture(*source));
                // Le dernier instant connu se prolonge jusqu'au bord de sa case seulement.
                let (x0, x1) = (x_de(t as f32), x_de(t as f32 + 1.0));
                if bus {
                    // Un bus : deux rails, un trait à chaque changement, la valeur écrite.
                    let couleur = if l.flous != 0 { c.x } else { c.accent_clair };
                    dessin.p.rect_filled(
                        Rect::from_min_max(Pos2::new(x0, y_haut), Pos2::new(x1, y_bas)),
                        0.0,
                        couleur.alpha(0.12),
                    );
                    for yr in [y_haut, y_bas] {
                        dessin.p.line_segment(
                            [Pos2::new(x0, yr), Pos2::new(x1, yr)],
                            Stroke::new(1.5, couleur.c()),
                        );
                    }
                    if avant != Some(l) {
                        if avant.is_some() {
                            dessin.p.line_segment(
                                [Pos2::new(x0, y_haut), Pos2::new(x0, y_bas)],
                                Stroke::new(1.5, couleur.c()),
                            );
                        }
                        dessin.texte(
                            Pos2::new(x0 + 4.0, milieu),
                            Align2::LEFT_CENTER,
                            &l.texte(),
                            Police::Mono,
                            k.a.typo.legende,
                            c.texte.c(),
                        );
                    }
                    continue;
                }
                match symbole(l.valeur()) {
                    '1' => {
                        dessin.p.rect_filled(
                            Rect::from_min_max(Pos2::new(x0, y_haut), Pos2::new(x1, y_bas)),
                            0.0,
                            c.un.alpha(0.22),
                        );
                        dessin.p.line_segment(
                            [Pos2::new(x0, y_haut), Pos2::new(x1, y_haut)],
                            Stroke::new(2.0, c.un.c()),
                        );
                    }
                    '0' => {
                        dessin.p.line_segment(
                            [Pos2::new(x0, y_bas), Pos2::new(x1, y_bas)],
                            Stroke::new(2.0, c.z.c()),
                        );
                    }
                    _ if l.valeur() == ce_moteur::Valeur::Z => {
                        // Haute impédance : trait au milieu, en pointillés (convention des
                        // chronogrammes, lisible sans la couleur).
                        dessin.p.extend(egui::Shape::dashed_line(
                            &[Pos2::new(x0, milieu), Pos2::new(x1, milieu)],
                            Stroke::new(1.5, c.z.c()),
                            2.0,
                            3.0,
                        ));
                    }
                    _ => {
                        // Inconnu : zone hachurée entre les deux niveaux, pas seulement colorée.
                        let zone = Rect::from_min_max(Pos2::new(x0, y_haut), Pos2::new(x1, y_bas));
                        dessin.p.rect_filled(zone, 0.0, c.x.alpha(0.10));
                        let h = zone.height();
                        let pas = 7.0;
                        let mut xh = (x0 / pas).floor() * pas - h;
                        let hache = dessin
                            .p
                            .with_clip_rect(zone.intersect(dessin.p.clip_rect()));
                        while xh < x1 {
                            hache.line_segment(
                                [Pos2::new(xh, y_bas), Pos2::new(xh + h, y_haut)],
                                Stroke::new(1.0, c.x.alpha(0.45)),
                            );
                            xh += pas;
                        }
                        for yr in [y_haut, y_bas] {
                            dessin.p.line_segment(
                                [Pos2::new(x0, yr), Pos2::new(x1, yr)],
                                Stroke::new(1.5, c.x.c()),
                            );
                        }
                    }
                }
                // Front montant ou descendant entre deux instants.
                if let Some(avant) = avant {
                    let (a, b) = (symbole(avant.valeur()), symbole(l.valeur()));
                    if a != b {
                        let couleur = match (a, b) {
                            ('X', _) => c.x.c(),
                            (_, '1') => c.un.c(),
                            _ => c.z.c(),
                        };
                        dessin.p.line_segment(
                            [Pos2::new(x0, y_haut), Pos2::new(x0, y_bas)],
                            Stroke::new(if a == 'X' { 1.5 } else { 2.0 }, couleur),
                        );
                    }
                }
            }
        }
        if sources.is_empty() {
            k.t.texte(
                ondes.center(),
                Align2::CENTER_CENTER,
                tr!("Pose une porte ou un interrupteur : ses signaux s'afficheront ici."),
                Police::Corps,
                k.a.typo.corps,
                c.doux.c(),
            );
        }
        // Curseur de l'instant affiché.
        let cle_curseur = egui::Id::unique("chrono-curseur");
        let vise = at.t_affiche() as f32;
        let mut xc = k
            .anims
            .vers(cle_curseur, vise, k.dt, k.a.mouvement.lissage_ui);
        if (xc - vise).abs() > 4.0 * n {
            k.anims.placer(cle_curseur, vise);
            xc = vise;
        }
        let curseur = Rect::from_min_max(
            Pos2::new(x_de(xc), ondes.top() - 2.0),
            Pos2::new(x_de(xc + 1.0), ondes.bottom() + 2.0),
        );
        dessin.p.rect(
            curseur,
            k.a.formes.rayon_petit,
            c.accent.alpha(0.16),
            Stroke::new(1.5, c.accent_clair.alpha(0.9)),
            StrokeKind::Inside,
        );

        // Clic sur un instant : on remonte le temps.
        let reponse =
            k.ui.interact(ondes, egui::Id::unique("chrono"), Sense::click_and_drag());
        if reponse.hovered() {
            k.ui.ctx().set_cursor_icon(CursorIcon::PointingHand);
        }
        if let Some(p) = reponse
            .interact_pointer_pos()
            .filter(|_| reponse.clicked() || reponse.dragged())
        {
            let t = (debut + (p.x - ondes.left()) / cellule).floor().max(0.0) as u64;
            at.regarder(t.clamp(premier, t_courant));
        }

        // Journal : les derniers changements, le plus récent en haut.
        if largeur_journal > 0.0 {
            let xj = r.right() - 16.0 - largeur_journal;
            k.t.legende(
                Pos2::new(xj, y_axe + 8.0),
                Align2::LEFT_CENTER,
                tr!("JOURNAL"),
                c.discret.c(),
            );
            let lignes = ((r.bottom() - 14.0 - y0) / 22.0).floor().max(0.0) as usize;
            if at.session.journal.is_empty() {
                k.t.texte(
                    Pos2::new(xj, y0 + 12.0),
                    Align2::LEFT_CENTER,
                    tr!("Rien n'a encore changé."),
                    Police::Corps,
                    k.a.typo.petit,
                    c.discret.c(),
                );
            }
            for (i, l) in at.session.journal.iter().rev().take(lignes).enumerate() {
                let y = y0 + 11.0 + i as f32 * 22.0;
                let force = 1.0 - 0.55 * (i as f32 / lignes.max(1) as f32);
                k.t.texte(
                    Pos2::new(xj, y),
                    Align2::LEFT_CENTER,
                    &k.t.abreger(&format!("t = {}", l.t), Police::Mono, k.a.typo.petit, 70.0),
                    Police::Mono,
                    k.a.typo.petit,
                    c.discret.alpha(force),
                );
                k.t.texte(
                    Pos2::new(xj + 76.0, y),
                    Align2::LEFT_CENTER,
                    &k.t.abreger(
                        &crate::langue::nom_affiche(&l.nom),
                        Police::MonoGras,
                        k.a.typo.petit,
                        42.0,
                    ),
                    Police::MonoGras,
                    k.a.typo.petit,
                    if l.entree {
                        c.accent_clair.alpha(force)
                    } else {
                        c.texte.alpha(force)
                    },
                );
                k.t.texte(
                    Pos2::new(xj + 122.0, y),
                    Align2::LEFT_CENTER,
                    &format!("{} → {}", l.de.texte(), l.vers.texte()),
                    Police::Mono,
                    k.a.typo.petit,
                    c.texte.alpha(force),
                );
                if l.entree && l.vers.largeur <= 1 {
                    k.t.texte(
                        Pos2::new(xj + 186.0, y),
                        Align2::LEFT_CENTER,
                        "toi",
                        Police::Corps,
                        k.a.typo.petit,
                        c.accent_clair.alpha(force),
                    );
                }
            }
        }
        k.t = fond;
    }

    fn outils(&mut self, k: &mut Cadre, at: &mut Atelier, plan: &mut Plan, d: &Disposition) {
        let r = d.outils;
        k.couvrir(r, egui::Id::unique("outils"));
        k.t.carte(r, k.a.formes.rayon_carte);
        let c = k.t.c;
        let b = k.a.formes.bouton;
        let mut x = r.left() + 6.0;
        let carre = |x: &mut f32| {
            let rect = Rect::from_min_size(Pos2::new(*x, r.center().y - b / 2.0), Vec2::splat(b));
            *x += b + 2.0;
            rect
        };
        if k.bouton_icone(
            carre(&mut x),
            "annuler",
            Icone::Annuler,
            Style::Nu,
            tr!("Annuler (Ctrl+Z)"),
            at.historique.peut_annuler(),
        ) {
            at.annuler();
        }
        if k.bouton_icone(
            carre(&mut x),
            "retablir",
            Icone::Retablir,
            Style::Nu,
            tr!("Rétablir (Ctrl+Y)"),
            at.historique.peut_retablir(),
        ) {
            at.retablir();
        }
        x += 4.0;
        k.t.p.line_segment(
            [
                Pos2::new(x - 0.5, r.center().y - 10.0),
                Pos2::new(x - 0.5, r.center().y + 10.0),
            ],
            Stroke::new(1.0, c.bord.c()),
        );
        x += 6.0;
        if k.bouton_icone(
            carre(&mut x),
            "zoom-moins",
            Icone::Moins,
            Style::Nu,
            tr!("Dézoomer"),
            true,
        ) {
            plan.zoomer_cran(k.a, -1.0);
        }
        let zoom = format!("{} %", (plan.cam.tk * 100.0).round() as i32);
        let l = k.t.largeur("000 %", Police::Mono, k.a.typo.petit) + 12.0;
        let rect = Rect::from_min_size(Pos2::new(x, r.center().y - b / 2.0), Vec2::new(l, b));
        let touche = k.toucher(
            rect,
            egui::Id::unique("zoom-cent"),
            tr!("Revenir à 100 %"),
            Sense::click(),
        );
        k.t.p.rect_filled(
            touche.rect,
            k.a.formes.rayon_bouton,
            c.survol.alpha(touche.chaud),
        );
        k.t.texte(
            rect.center(),
            Align2::CENTER_CENTER,
            &zoom,
            Police::Mono,
            k.a.typo.petit,
            c.doux.vers(c.texte, touche.chaud).c(),
        );
        if touche.clic {
            let facteur = 1.0 / plan.cam.tk;
            plan.cam
                .zoomer(d.libre.center(), facteur, &k.a.manipulation, false);
        }
        x += l + 2.0;
        if k.bouton_icone(
            carre(&mut x),
            "zoom-plus",
            Icone::Plus,
            Style::Nu,
            tr!("Zoomer"),
            true,
        ) {
            plan.zoomer_cran(k.a, 1.0);
        }
        if k.bouton_icone(
            carre(&mut x),
            "cadrer",
            Icone::Cadrer,
            Style::Nu,
            tr!("Recentrer le circuit (F)"),
            true,
        ) {
            plan.cadrer(false);
        }
        if k.bouton_icone(
            carre(&mut x),
            "photo",
            Icone::Photo,
            Style::Nu,
            tr!("Exporter le plan en image (PNG)"),
            true,
        ) {
            self.photo_demandee = true;
        }
        self.contenus.outils = x - 2.0 - r.left() + 6.0;
    }

    fn barre_de_selection(&mut self, k: &mut Cadre, at: &mut Atelier, d: &Disposition) {
        let nom = at.nom_selection();
        let presence = k.anims.vers(
            egui::Id::unique("barre-selection"),
            if nom.is_some() { 1.0 } else { 0.0 },
            k.dt,
            k.a.mouvement.lissage_ui,
        );
        let Some(nom) = nom.filter(|_| presence > 0.02) else {
            return;
        };
        let c = k.t.c;
        let ids = at.elements_selectionnes().to_vec();
        let est = |id: &Id, f: &dyn Fn(&Element) -> bool| at.doc.element(*id).is_some_and(f);
        let mut actions: Vec<(Action, &str, Icone, &str)> = Vec::new();
        if ids.len() == 1 && est(&ids[0], &|e| e.genre == Genre::Brique) {
            actions.push((
                Action::Ouvrir,
                tr!("Ouvrir"),
                Icone::Ouvrir,
                tr!("Voir et modifier le plan de cette brique (double-clic)"),
            ));
        }
        if ids.len() >= 2 && ids.iter().any(|id| est(id, &|e| groupable(e.genre))) {
            actions.push((
                Action::Regrouper,
                tr!("Regrouper"),
                Icone::Grouper,
                tr!("En faire une brique réutilisable (Ctrl+G)"),
            ));
        }
        if !ids.is_empty() {
            actions.push((
                Action::Dupliquer,
                tr!("Dupliquer"),
                Icone::Dupliquer,
                tr!("Copier juste à côté (Ctrl+D)"),
            ));
            actions.push((
                Action::Retourner,
                tr!("Retourner"),
                Icone::Miroir,
                tr!("Entrées à droite, sorties à gauche (touche R)"),
            ));
        }
        let supprimable = match &at.selection {
            Some(Selection::Elements(ids)) => ids.iter().any(|id| at.supprimable(*id)),
            Some(Selection::Fil(_)) => true,
            None => false,
        };
        if let Some(source) = at.fil_selectionne().and_then(|f| at.doc.source_de(f)) {
            let pose = at.session.arret() == Some(source);
            actions.push((
                Action::Arret,
                if pose {
                    tr!("Retirer l'arrêt")
                } else {
                    tr!("Point d'arrêt")
                },
                Icone::Pause,
                tr!("La lecture s'arrête quand ce fil passe à 1"),
            ));
        }
        if supprimable {
            actions.push((
                Action::Supprimer,
                if at.fil_selectionne().is_some() {
                    tr!("Débrancher")
                } else {
                    tr!("Supprimer")
                },
                Icone::Poubelle,
                tr!("Touche Suppr"),
            ));
        }
        let nom = k.t.abreger(&nom, Police::Mono, k.a.typo.petit, 220.0);
        let l_nom = k.t.largeur(&nom, Police::Mono, k.a.typo.petit);
        let b = k.a.formes.bouton;
        // Avec les mots si la place le permet, sinon les icônes seules.
        let place = d.outils.left() - 8.0 - d.selection.x;
        let largeurs = |mots: bool| -> Vec<f32> {
            actions
                .iter()
                .map(|(_, texte, ..)| {
                    if mots {
                        8.0 + 16.0
                            + 7.0
                            + k.t.largeur(texte, Police::CorpsMoyen, k.a.typo.corps)
                            + 10.0
                    } else {
                        b
                    }
                })
                .collect()
        };
        let total = |l: &[f32]| {
            14.0 + l_nom
                + if l.is_empty() {
                    14.0
                } else {
                    20.0 + l.iter().sum::<f32>() + 2.0 * (l.len() - 1) as f32 + 6.0
                }
        };
        let (mots, l_boutons) = match (largeurs(true), largeurs(false)) {
            (l, _) if total(&l) <= place => (true, l),
            (_, l) if total(&l) <= place => (false, l),
            _ => return,
        };
        let r = Rect::from_min_size(
            d.selection + Vec2::new(0.0, (1.0 - presence) * 14.0),
            Vec2::new(total(&l_boutons), d.hauteur_pilule),
        );
        k.couvrir(r, egui::Id::unique("selection"));
        let voile = k.t.sur(presence, k.t.p.clip_rect());
        let fond = std::mem::replace(&mut k.t, voile);
        k.t.carte(r, k.a.formes.rayon_carte);
        k.t.texte(
            Pos2::new(r.left() + 14.0, r.center().y),
            Align2::LEFT_CENTER,
            &nom,
            Police::Mono,
            k.a.typo.petit,
            c.doux.c(),
        );
        let mut x = r.left() + 14.0 + l_nom + 10.0;
        k.t.p.line_segment(
            [
                Pos2::new(x - 0.5, r.center().y - 10.0),
                Pos2::new(x - 0.5, r.center().y + 10.0),
            ],
            Stroke::new(1.0, c.bord.c()),
        );
        x += 10.0;
        let mut choisie = None;
        for ((action, texte, icone, bulle), l) in actions.iter().zip(&l_boutons) {
            let rect = Rect::from_min_size(Pos2::new(x, r.center().y - b / 2.0), Vec2::new(*l, b));
            x += l + 2.0;
            let cle = match action {
                Action::Ouvrir => "ouvrir-selection",
                Action::Regrouper => "regrouper",
                Action::Dupliquer => "dupliquer",
                Action::Supprimer => "supprimer",
                Action::Arret => "point-d-arret",
                Action::Retourner => "retourner",
            };
            k.reperes.push(("action", *action as usize, rect));
            let touche = k.toucher(rect, egui::Id::unique(cle), bulle, Sense::click());
            let teinte = if *action == Action::Supprimer {
                c.danger
            } else {
                c.doux.vers(c.texte, touche.chaud)
            };
            k.t.p.rect_filled(
                touche.rect,
                k.a.formes.rayon_bouton,
                if *action == Action::Supprimer {
                    c.danger.alpha(0.12 * touche.chaud)
                } else {
                    c.survol.alpha(touche.chaud)
                },
            );
            if mots {
                k.t.icone(
                    *icone,
                    Pos2::new(touche.rect.left() + 16.0, rect.center().y),
                    16.0,
                    teinte.c(),
                );
                k.t.texte(
                    Pos2::new(touche.rect.left() + 31.0, rect.center().y),
                    Align2::LEFT_CENTER,
                    texte,
                    Police::CorpsMoyen,
                    k.a.typo.corps,
                    teinte.c(),
                );
            } else {
                k.t.icone(*icone, touche.rect.center(), 17.0, teinte.c());
            }
            if touche.clic {
                choisie = Some(*action);
            }
        }
        k.t = fond;
        match choisie {
            Some(Action::Ouvrir) => {
                at.entrer(ids[0]);
            }
            Some(Action::Regrouper) => {
                at.regrouper_selection(k.a);
            }
            Some(Action::Dupliquer) => {
                at.dupliquer_selection(k.a);
            }
            Some(Action::Supprimer) => {
                at.supprimer_selection(k.a);
            }
            Some(Action::Arret) => at.basculer_point_d_arret(),
            Some(Action::Retourner) => {
                at.retourner_selection(k.a);
            }
            None => {}
        }
    }

    /// Les messages passagers, centrés en haut de `libre`, `decalage` points sous son bord.
    pub(crate) fn messages(
        k: &mut Cadre,
        at: &mut Atelier,
        libre: Rect,
        decalage: f32,
        en_bas: bool,
        choisie: &mut Option<Proposition>,
    ) {
        let c = k.t.c;
        // Les messages s'empilent sous le haut de `libre`, ou au-dessus de son bas.
        let mut y = if en_bas {
            libre.bottom() - decalage
        } else {
            libre.top() + decalage
        };
        let mut a_fermer = None;
        let largeur_max = (libre.width() - 24.0).min(620.0);
        let mv = &k.a.mouvement;
        let (entree, sortie, glissement) = (
            mv.duree(mv.message_entree).max(0.01),
            mv.duree(mv.message_sortie).max(0.01),
            mv.glissement,
        );
        for m in &at.messages {
            let age = (at.horloge - m.ne) as f32;
            let reste = m.duree.map_or(f32::INFINITY, |duree| duree - age);
            // Le message arrive en glissant et repart en s'effaçant.
            let presence = Courbe::Sortie
                .en((age / entree).clamp(0.0, 1.0))
                .min((reste / sortie).clamp(0.0, 1.0));
            if presence <= 0.0 {
                // Pas encore là ou déjà parti : rien à montrer, et rien ne doit capter les clics.
                continue;
            }
            let ferme = m.duree.is_none();
            // Les liens du message, de gauche à droite : l'action proposée, puis de quoi fermer.
            let liens: Vec<(&str, Option<Proposition>)> = match m.action {
                Some(action) => vec![(action.libelle(), Some(action)), (tr!("Plus tard"), None)],
                None if ferme => vec![(tr!("Compris"), None)],
                None => Vec::new(),
            };
            let largeurs: Vec<f32> = liens
                .iter()
                .map(|(texte, _)| k.t.largeur(texte, Police::CorpsDemi, k.a.typo.corps) + 20.0)
                .collect();
            let l_lien: f32 = largeurs.iter().sum();
            let galley = k.t.p.layout(
                m.texte.clone(),
                Police::Corps.taille(k.a.typo.corps * k.a.formes.echelle_texte),
                c.texte.c(),
                (largeur_max - 50.0 - l_lien).max(80.0),
            );
            let taille = galley.size();
            let largeur = taille.x + 50.0 + l_lien;
            let hauteur = (taille.y + 20.0).max(40.0);
            let haut = if en_bas {
                y - hauteur + (1.0 - presence) * glissement
            } else {
                y - (1.0 - presence) * glissement
            };
            let r = Rect::from_min_size(
                Pos2::new(libre.center().x - largeur / 2.0, haut),
                Vec2::new(largeur, hauteur),
            );
            k.couvrir(r, egui::Id::unique(("message", m.id)));
            let voile = k.t.sur(presence, k.t.p.clip_rect());
            let fond = std::mem::replace(&mut k.t, voile);
            k.t.carte(r, k.a.formes.rayon_carte);
            // Une pastille d'information à gauche : le message se lit comme une notification.
            k.t.icone(
                Icone::Infos,
                Pos2::new(r.left() + 22.0, r.top() + 20.0),
                16.0,
                c.accent_clair.c(),
            );
            k.t.p.galley(
                Pos2::new(r.left() + 38.0, r.center().y - taille.y / 2.0),
                galley,
                c.texte.c(),
            );
            let mut gauche = r.right() - l_lien - 10.0;
            for (i, ((texte, action), largeur)) in liens.iter().zip(&largeurs).enumerate() {
                let rect = Rect::from_min_max(
                    Pos2::new(gauche, r.top() + 6.0),
                    Pos2::new(gauche + largeur - 2.0, r.bottom() - 6.0),
                );
                gauche += largeur;
                let repere = match action {
                    Some(Proposition::MettreAJour) => "mettre-a-jour",
                    Some(Proposition::Redemarrer) => "redemarrer",
                    Some(Proposition::Restaurer) => "restaurer",
                    Some(Proposition::VoirLeRapport) => "voir-le-rapport",
                    None => "fermer-le-message",
                };
                k.reperer(repere, 0, rect);
                let touche = k.toucher(
                    rect,
                    egui::Id::unique(("lien", m.id, i)),
                    "",
                    Sense::click(),
                );
                let couleur = if action.is_some() {
                    c.accent_clair.vers(c.texte, touche.chaud)
                } else {
                    c.texte.vers(c.accent_clair, touche.chaud)
                };
                let fin = k.t.texte(
                    rect.center(),
                    Align2::CENTER_CENTER,
                    texte,
                    Police::CorpsDemi,
                    k.a.typo.corps,
                    couleur.c(),
                );
                k.t.p.line_segment(
                    [
                        fin.left_bottom() + Vec2::new(0.0, 1.0),
                        fin.right_bottom() + Vec2::new(0.0, 1.0),
                    ],
                    Stroke::new(1.0, couleur.c()),
                );
                if touche.clic {
                    a_fermer = Some(m.id);
                    if action.is_some() {
                        *choisie = *action;
                    }
                }
            }
            k.t = fond;
            if en_bas {
                y -= (hauteur + 8.0) * presence;
            } else {
                y += (hauteur + 8.0) * presence;
            }
        }
        if let Some(id) = a_fermer {
            at.fermer_message(id);
        }
    }
}

/// Nombre de signaux montrés au plus dans les sondes.
const SONDES: usize = 40;
/// Hauteur de l'en-tête d'un panneau (titre et bouton pour le cacher), trait compris.
const ENTETE: f32 = 44.0;
/// Nombre de signaux montrés au plus dans le chronogramme.
const LIGNES_CHRONO: usize = 10;

/// Un débit de simulation écrit court : « 850 pas/s », « 33 k pas/s », « 1,2 M pas/s ».
fn debit_lisible(pas_par_seconde: f32) -> String {
    if pas_par_seconde < 1000.0 {
        tr!("{n} pas/s", n = pas_par_seconde.round() as i64)
    } else if pas_par_seconde < 1_000_000.0 {
        tr!("{n} k pas/s", n = (pas_par_seconde / 1000.0).round() as i64)
    } else {
        tr!(
            "{n} M pas/s",
            n = crate::langue::decimal(format!("{:.1}", pas_par_seconde / 1_000_000.0))
        )
    }
}

/// Les visuels des widgets standard d'egui (fenêtres d'aide et d'éditeur, info-bulles,
/// champs) habillés avec les jetons de la direction artistique : mêmes surfaces, bords,
/// rayons et accent que l'interface peinte à la main.
pub fn visuels(a: &Apparence) -> egui::Visuals {
    let c = a.palette();
    let f = &a.formes;
    let mut v = match a.mode {
        Mode::Sombre => egui::Visuals::dark(),
        Mode::Clair => egui::Visuals::light(),
    };
    v.window_fill = c.carte.c();
    v.panel_fill = c.fond.c();
    v.window_stroke = Stroke::new(1.0, c.bord.c());
    v.window_corner_radius = f.rayon_vitrine.into();
    v.window_shadow = egui::epaint::Shadow {
        offset: [0, 12],
        blur: 32,
        spread: 0,
        color: c.ombre.c(),
    };
    v.window_highlight_topmost = false;
    v.override_text_color = Some(c.texte.c());
    v.menu_corner_radius = f.rayon_carte.into();
    v.popup_shadow = egui::epaint::Shadow {
        offset: [0, 6],
        blur: 18,
        spread: 0,
        color: c.ombre.c(),
    };
    v.extreme_bg_color = c.fond.c();
    v.text_edit_bg_color = Some(c.fond.c());
    v.faint_bg_color = c.carte_2.c();
    v.code_bg_color = c.enfonce.c();
    v.hyperlink_color = c.accent_clair.c();
    v.warn_fg_color = c.attente.c();
    v.error_fg_color = c.danger.c();
    v.selection.bg_fill = c.accent.alpha(0.28);
    v.selection.stroke = Stroke::new(1.0, c.texte.c());
    v.text_cursor.stroke = Stroke::new(2.0, c.accent_clair.c());
    let rayon = egui::CornerRadius::from(f.rayon_bouton);
    let w = &mut v.widgets;
    w.noninteractive.bg_fill = c.carte.c();
    w.noninteractive.weak_bg_fill = c.carte.c();
    w.noninteractive.bg_stroke = Stroke::new(1.0, c.trait_.c());
    w.noninteractive.fg_stroke = Stroke::new(1.0, c.doux.c());
    w.noninteractive.corner_radius = rayon;
    for (etat, fond, bord, encre) in [
        (&mut w.inactive, c.carte_2, c.bord, c.texte),
        (&mut w.hovered, c.survol, c.bord_fort, c.texte),
        (&mut w.active, c.enfonce, c.bord_fort, c.texte),
        (&mut w.open, c.enfonce, c.bord_fort, c.texte),
    ] {
        etat.bg_fill = fond.c();
        etat.weak_bg_fill = fond.c();
        etat.bg_stroke = Stroke::new(1.0, bord.c());
        etat.fg_stroke = Stroke::new(1.5, encre.c());
        etat.corner_radius = rayon;
        etat.expansion = 0.0;
    }
    v
}

/// Le style complet des widgets d'egui : les visuels ci-dessus, plus les tailles de texte et
/// les espacements de la direction artistique.
pub fn style_egui(a: &Apparence, style: &mut egui::Style) {
    use egui::TextStyle;
    style.visuals = visuels(a);
    let (ty, e) = (&a.typo, a.formes.echelle_texte);
    style.text_styles = [
        (TextStyle::Small, Police::Corps.taille(ty.petit * e)),
        (TextStyle::Body, Police::Corps.taille(ty.corps * e)),
        (TextStyle::Button, Police::CorpsMoyen.taille(ty.corps * e)),
        (TextStyle::Heading, Police::CorpsDemi.taille(ty.fort * e)),
        (TextStyle::Monospace, Police::Mono.taille(ty.corps * e)),
    ]
    .into();
    let s = &mut style.spacing;
    s.item_spacing = Vec2::new(8.0, 6.0);
    s.button_padding = Vec2::new(10.0, 5.0);
    s.interact_size.y = a.formes.controle - 4.0;
    s.window_margin = egui::Margin::same(a.formes.rembourrage as i8);
    s.menu_margin = egui::Margin::same(8);
}
