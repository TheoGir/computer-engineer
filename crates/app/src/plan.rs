//! Le plan : le circuit dessiné, animé et manipulable.
//!
//! Le plan garde ce qui n'appartient qu'à l'écran : la caméra, la position affichée de chaque
//! élément (qui rejoint en douceur sa position réelle), le front du courant sur chaque fil et
//! le geste en cours. Il lit l'[`Atelier`] et lui demande les modifications.

use std::collections::{HashMap, HashSet};

use ce_moteur::Valeur;
use egui::epaint::{Mesh, Shadow};
use egui::{Align2, Color32, CursorIcon, Pos2, Rect, Sense, Shape, Stroke, StrokeKind, Vec2};

use crate::apparence::{Apparence, Courbe, Manipulation, Mouvement, Palette};
use crate::calques::{self, Fantome};
use crate::dessin::{coins, contour_arrondi, fermer, halo, Animations, Icone, Police, Toile};
use crate::disposition::Disposition;
use crate::document::{Document, Element, Fil, Genre, Id, Point, Refus, Source, PAS_BROCHE};
use crate::etat::{Atelier, Selection, Trajet};
use crate::routage;
use crate::simulation::symbole;

/// La caméra : `écran = (x, y) + monde × k`. Les champs `t*` sont la cible que la caméra
/// rejoint en douceur ; `v*` est la vitesse de glissade après un déplacement lâché.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Camera {
    /// Décalage horizontal affiché.
    pub x: f32,
    /// Décalage vertical affiché.
    pub y: f32,
    /// Zoom affiché.
    pub k: f32,
    /// Décalage horizontal visé.
    pub tx: f32,
    /// Décalage vertical visé.
    pub ty: f32,
    /// Zoom visé.
    pub tk: f32,
    /// Vitesse de glissade horizontale, en points par seconde.
    pub vx: f32,
    /// Vitesse de glissade verticale.
    pub vy: f32,
}

impl Default for Camera {
    fn default() -> Self {
        Camera {
            x: 0.0,
            y: 0.0,
            k: 1.0,
            tx: 0.0,
            ty: 0.0,
            tk: 1.0,
            vx: 0.0,
            vy: 0.0,
        }
    }
}

impl Camera {
    /// Position à l'écran d'un point du plan.
    pub fn ecran(&self, p: Point) -> Pos2 {
        Pos2::new(self.x + p.x * self.k, self.y + p.y * self.k)
    }

    /// Point du plan sous une position de l'écran.
    pub fn monde(&self, p: Pos2) -> Point {
        Point::nouveau((p.x - self.x) / self.k, (p.y - self.y) / self.k)
    }

    /// Déplace la caméra tout de suite, sans lissage (le plan suit le doigt).
    pub fn deplacer(&mut self, d: Vec2) {
        self.x += d.x;
        self.y += d.y;
        self.tx += d.x;
        self.ty += d.y;
    }

    /// Zoome autour d'un point de l'écran, qui reste fixe. `direct` : sans lissage (pincement).
    pub fn zoomer(&mut self, ancre: Pos2, facteur: f32, m: &Manipulation, direct: bool) {
        let k = (self.tk * facteur).clamp(m.zoom_min, m.zoom_max);
        let r = k / self.tk;
        self.tx = ancre.x - (ancre.x - self.tx) * r;
        self.ty = ancre.y - (ancre.y - self.ty) * r;
        self.tk = k;
        if direct {
            self.poser();
        }
    }

    /// Vise le cadrage qui montre `boite` (en unités du plan) en entier dans `libre`.
    pub fn cadrer(&mut self, boite: Rect, libre: Rect, m: &Manipulation, direct: bool) {
        let k = (libre.width() / boite.width().max(1.0))
            .min(libre.height() / boite.height().max(1.0))
            .clamp(m.zoom_min, m.zoom_cadrage);
        self.tk = k;
        self.tx = libre.center().x - boite.center().x * k;
        self.ty = libre.center().y - boite.center().y * k;
        self.vx = 0.0;
        self.vy = 0.0;
        if direct {
            self.poser();
        }
    }

    fn poser(&mut self) {
        self.x = self.tx;
        self.y = self.ty;
        self.k = self.tk;
    }

    /// Avance le lissage et la glissade. Renvoie `true` tant que la caméra bouge.
    pub fn animer(&mut self, dt: f32, m: &Mouvement) -> bool {
        let mut bouge = false;
        if self.vx.hypot(self.vy) > 4.0 && m.inertie > 0.0 {
            self.deplacer(Vec2::new(self.vx * dt, self.vy * dt));
            let frein = (-dt / m.inertie).exp();
            self.vx *= frein;
            self.vy *= frein;
            bouge = true;
        } else {
            self.vx = 0.0;
            self.vy = 0.0;
        }
        let proche = (self.x - self.tx).abs() < 0.05
            && (self.y - self.ty).abs() < 0.05
            && (self.k - self.tk).abs() < 0.0005;
        if proche || m.lissage_camera <= 0.0 {
            self.poser();
        } else {
            let f = 1.0 - (-dt / m.lissage_camera).exp();
            self.x += (self.tx - self.x) * f;
            self.y += (self.ty - self.y) * f;
            self.k += (self.tk - self.k) * f;
            bouge = true;
        }
        bouge
    }
}

/// Un point de connexion.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Port {
    /// Une sortie d'un élément.
    Sortie(Source),
    /// Une entrée d'un élément.
    Entree(Fil),
}

/// Ce qui se trouve sous le pointeur.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Survol {
    /// Le fond du plan.
    Rien,
    /// Un point de connexion.
    Port(Port),
    /// Un élément.
    Element(Id),
    /// Un fil.
    Fil(Fil),
}

/// Le geste en cours.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Geste {
    /// Aucun.
    Aucun,
    /// On déplace le plan (ou on clique sur le fond, ou sur un fil).
    Plan {
        /// Distance parcourue depuis l'appui, en points.
        parcouru: f32,
        /// Le fil sur lequel l'appui a eu lieu, s'il y en a un.
        fil: Option<Fil>,
    },
    /// On déplace un élément.
    Element {
        /// L'élément.
        id: Id,
        /// Écart entre le centre de l'élément et le pointeur, en unités du plan.
        decalage: Vec2,
        /// Position du pointeur à l'appui.
        appui: Pos2,
        /// Vrai dès que le pointeur a assez bougé pour que ce ne soit plus un clic.
        bouge: bool,
        /// Vrai si l'élément vient de la palette et n'est pas encore posé.
        nouveau: bool,
        /// Vrai si la hauteur est aimantée sur un fil voisin.
        aimante: bool,
    },
    /// On trace un cadre de sélection.
    Cadre {
        /// Le coin de départ, en unités du plan.
        depart: Point,
        /// Vrai si le cadre s'ajoute à la sélection au lieu de la remplacer.
        ajoute: bool,
    },
    /// L'appui a déjà fait son effet : on attend seulement que le bouton soit relâché.
    Inerte,
    /// On tire un fil.
    Fil {
        /// Le bout déjà fixé.
        depuis: Port,
        /// Le point de connexion sur lequel le fil s'accrocherait si on lâchait maintenant.
        aimant: Option<Port>,
    },
}

#[derive(Clone, Copy, Debug)]
struct Onde {
    de: Valeur,
    vers: Valeur,
    debut: f64,
}

/// Ce que le plan retient d'un élément affiché.
#[derive(Clone, Copy, Debug)]
struct Vu {
    /// Sa position affichée.
    pos: Point,
    /// Son corps, relatif à son centre : gauche, haut, droite, bas.
    corps: (f32, f32, f32, f32),
    /// Vrai pour une lampe, qui est ronde.
    rond: bool,
}

/// Le passage d'un plan à un autre : on plonge dans une brique, ou on en remonte.
struct Plongee {
    /// Le circuit qu'on quitte, tel qu'il était affiché.
    fantome: Option<Fantome>,
    /// L'instant du passage.
    ne: f64,
    /// Le point de l'écran autour duquel tout grossit ou rapetisse : la brique.
    foyer: Pos2,
    /// De combien grossit le circuit qu'on quitte ; celui qui arrive part de l'inverse.
    grossit: f32,
}

/// Au-delà de ce nombre d'éléments ou de fils changés d'un coup, on ne les anime pas un par un.
const ANIMES_MAX: usize = 200;

/// Un nombre entre 0 et 1 tiré de `graine`, toujours le même pour la même graine.
fn hasard(graine: u64) -> f32 {
    let mut x = graine.wrapping_add(0x9E37_79B9_7F4A_7C15);
    x = (x ^ (x >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
    x = (x ^ (x >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
    ((x ^ (x >> 31)) & 0xFFFF) as f32 / 65_535.0
}

/// Le plan interactif.
pub struct Plan {
    /// La caméra.
    pub cam: Camera,
    /// Le geste en cours.
    pub geste: Geste,
    /// Ce qui est sous le pointeur.
    pub survol: Survol,
    affiche: HashMap<Id, Vu>,
    ondes: HashMap<Source, Onde>,
    eclats: HashMap<Id, f64>,
    naissances: HashMap<Id, f64>,
    /// Les éléments qui viennent d'être retirés, avec l'instant de leur départ.
    disparus: Vec<(Vu, f64)>,
    /// Les fils qui viennent d'être retirés : leur tracé et l'instant de leur départ.
    fils_partis: Vec<(Vec<Point>, f64)>,
    /// Les fils qui viennent d'être branchés, avec l'instant de leur branchement.
    fils_nes: HashMap<(Source, Fil), f64>,
    /// Le câblage (sans les positions) au dernier tracé, pour repérer les fils changés.
    cle_cablage: u64,
    cablage_connu: bool,
    /// Les formes du circuit peintes à la dernière image, sans le fond du plan.
    plage: (usize, usize),
    /// La brique où l'on entre à la fin de l'image (double-clic), une fois le plan dessiné.
    entree_demandee: Option<Id>,
    plongee: Option<Plongee>,
    /// Le tracé de tous les fils, refait seulement quand le câblage ou une position change.
    faisceaux: Vec<routage::Faisceau>,
    /// Pour chaque fil de chaque faisceau, le rectangle qui le contient.
    boites: Vec<Vec<Rect>>,
    cle_routage: u64,
    cadrage_demande: Option<bool>,
    initialise: bool,
    t_vu: u64,
    dernier_clic: Option<(f64, Pos2)>,
    dernier_clic_element: Option<(f64, Id)>,
    /// Le bouton poussoir enfoncé en ce moment : il remonte quand on lâche.
    poussoir: Option<Id>,
    /// Le plan tel qu'il était avant de tirer un élément depuis la palette.
    avant_pose: Option<Document>,
    lieu_vu: u64,
    /// Nombre d'éléments et de fils réellement dessinés à la dernière image.
    pub dessines: (usize, usize),
    dernier_pointeur: Option<(Pos2, bool)>,
    libre: Rect,
    /// Les rectangles des panneaux posés au-dessus du plan, à l'image précédente : le plan
    /// ignore le pointeur quand il est dessus.
    pub obstacles: Vec<Rect>,
    /// Vrai si le plan a bougé pendant l'image : il faut redessiner.
    pub en_mouvement: bool,
}

impl Default for Plan {
    fn default() -> Self {
        Plan {
            cam: Camera::default(),
            geste: Geste::Aucun,
            survol: Survol::Rien,
            affiche: HashMap::new(),
            ondes: HashMap::new(),
            eclats: HashMap::new(),
            naissances: HashMap::new(),
            disparus: Vec::new(),
            fils_partis: Vec::new(),
            fils_nes: HashMap::new(),
            cle_cablage: 0,
            cablage_connu: false,
            plage: (0, 0),
            entree_demandee: None,
            plongee: None,
            faisceaux: Vec::new(),
            boites: Vec::new(),
            cle_routage: 0,
            cadrage_demande: Some(true),
            initialise: false,
            t_vu: 0,
            dernier_clic: None,
            dernier_clic_element: None,
            poussoir: None,
            avant_pose: None,
            lieu_vu: 0,
            dessines: (0, 0),
            dernier_pointeur: None,
            libre: Rect::NOTHING,
            obstacles: Vec::new(),
            en_mouvement: false,
        }
    }
}

fn couleur_valeur(c: &Palette, v: Valeur) -> Color32 {
    match symbole(v) {
        '1' => c.un.c(),
        '0' => c.zero.c(),
        _ => c.x.c(),
    }
}

fn arrondi(v: f32, pas: f32) -> f32 {
    (v / pas).round() * pas
}

impl Plan {
    /// Position affichée d'un élément (elle rejoint en douceur sa position réelle).
    pub fn position(&self, at: &Atelier, id: Id) -> Point {
        self.affiche
            .get(&id)
            .map(|vu| vu.pos)
            .or_else(|| at.doc.element(id).map(|e| e.pos))
            .unwrap_or_default()
    }

    /// Centre d'un élément à l'écran.
    pub fn centre_ecran(&self, at: &Atelier, id: Id) -> Pos2 {
        self.cam.ecran(self.position(at, id))
    }

    /// Position à l'écran d'un point de connexion.
    pub fn port_ecran(&self, at: &Atelier, port: Port) -> Pos2 {
        self.cam.ecran(self.port_monde(at, port))
    }

    fn port_monde(&self, at: &Atelier, port: Port) -> Point {
        match port {
            Port::Sortie(s) => at
                .doc
                .element(s.id)
                .map(|e| e.point_sortie(self.position(at, s.id), s.sortie))
                .unwrap_or_default(),
            Port::Entree(f) => at
                .doc
                .element(f.puits)
                .map(|e| e.entree_en(self.position(at, f.puits), f.entree))
                .unwrap_or_default(),
        }
    }

    /// Demande un recadrage du circuit à la prochaine image. `direct` : sans animation.
    pub fn cadrer(&mut self, direct: bool) {
        self.cadrage_demande = Some(direct);
    }

    /// Zoome d'un cran autour du centre de la zone libre.
    pub fn zoomer_cran(&mut self, a: &Apparence, sens: f32) {
        let facteur = a.manipulation.cran_zoom.powf(sens);
        self.cam
            .zoomer(self.libre.center(), facteur, &a.manipulation, false);
    }

    /// Commence à poser un élément tiré depuis la palette : il suit le pointeur.
    pub fn commencer_pose(&mut self, at: &mut Atelier, element: Element, pointeur: Pos2) {
        self.avant_pose = Some(at.doc.clone());
        let id = at.poser(element, self.cam.monde(pointeur));
        self.geste = Geste::Element {
            id,
            decalage: Vec2::ZERO,
            appui: pointeur,
            bouge: true,
            nouveau: true,
            aimante: false,
        };
    }

    /// Pose un élément au milieu de la zone libre (clic dans la palette), là où il y a de la
    /// place.
    pub fn poser_au_centre(&mut self, a: &Apparence, at: &mut Atelier, element: Element) -> Id {
        let g = a.manipulation.grille;
        let c = self.cam.monde(self.libre.center());
        let mut pos = Point::nouveau(arrondi(c.x, g), arrondi(c.y, g));
        let (l, h) = (element.largeur(), element.hauteur());
        let gene = |pos: Point| {
            at.doc.elements.iter().any(|e| {
                (e.pos.x - pos.x).abs() < (e.largeur() + l) / 2.0 + 8.0
                    && (e.pos.y - pos.y).abs() < (e.hauteur() + h) / 2.0 + 8.0
            })
        };
        let mut essais = 0;
        while gene(pos) && essais < 400 {
            pos = Point::nouveau(pos.x + 30.0, pos.y + 40.0);
            essais += 1;
        }
        let id = at.poser(element, pos);
        at.apres_modification(a);
        id
    }

    /// Ce qui se trouve sous un point du plan.
    pub fn viser(&self, a: &Apparence, at: &Atelier, p: Point) -> Survol {
        let rayon = a.manipulation.rayon_port.max(9.0 / self.cam.k);
        for e in at.doc.elements.iter().rev() {
            let pos = self.position(at, e.id);
            // Loin de l'élément, inutile de regarder ses points de connexion.
            if (p.x - pos.x).abs() > e.largeur() / 2.0 + rayon + 20.0
                || (p.y - pos.y).abs() > e.hauteur() / 2.0 + rayon + 40.0
            {
                continue;
            }
            for j in 0..e.broches_s.len() {
                let s = e.point_sortie(pos, j);
                if p.distance(Point::nouveau(s.x + 6.0, s.y)) <= rayon.min(PAS_BROCHE / 2.0) {
                    return Survol::Port(Port::Sortie(Source::nouvelle(e.id, j)));
                }
            }
            for i in 0..e.entrees.len() {
                let q = e.entree_en(pos, i);
                if p.distance(Point::nouveau(q.x - 6.0, q.y)) <= rayon.min(PAS_BROCHE / 2.0) {
                    return Survol::Port(Port::Entree(Fil {
                        puits: e.id,
                        entree: i,
                    }));
                }
            }
        }
        for e in at.doc.elements.iter().rev() {
            if e.contient(self.position(at, e.id), p) {
                return Survol::Element(e.id);
            }
        }
        let touche = a.manipulation.touche_fil.max(7.0 / self.cam.k);
        let mut meilleur = None;
        let ici = Pos2::new(p.x, p.y);
        for (faisceau, boites) in self.faisceaux.iter().zip(&self.boites) {
            for ((fil, ligne), boite) in faisceau.fils.iter().zip(boites) {
                if !boite.expand(touche).contains(ici) {
                    continue;
                }
                let d = routage::distance_a(ligne, p);
                if d <= touche && meilleur.is_none_or(|(m, _)| d < m) {
                    meilleur = Some((d, *fil));
                }
            }
        }
        meilleur.map_or(Survol::Rien, |(_, f)| Survol::Fil(f))
    }

    /// Le point de connexion compatible le plus proche du pointeur, à portée d'accroche.
    fn accrocher(&self, a: &Apparence, at: &Atelier, depuis: Port, p: Point) -> Option<Port> {
        let portee = a.manipulation.accroche.max(20.0 / self.cam.k);
        let mut meilleur: Option<(f32, Port)> = None;
        let mut proposer = |port: Port, q: Point| {
            let d = p.distance(q);
            if d <= portee && meilleur.is_none_or(|(m, _)| d < m) {
                meilleur = Some((d, port));
            }
        };
        for e in &at.doc.elements {
            let pos = self.position(at, e.id);
            if (p.x - pos.x).abs() > e.largeur() / 2.0 + portee + 20.0
                || (p.y - pos.y).abs() > e.hauteur() / 2.0 + portee + 40.0
            {
                continue;
            }
            match depuis {
                Port::Sortie(source) => {
                    for i in 0..e.entrees.len() {
                        let fil = Fil {
                            puits: e.id,
                            entree: i,
                        };
                        if at.doc.verifier(source, fil).is_ok() {
                            proposer(Port::Entree(fil), e.entree_en(pos, i));
                        }
                    }
                }
                Port::Entree(fil) => {
                    for j in 0..e.broches_s.len() {
                        let source = Source::nouvelle(e.id, j);
                        if at.doc.verifier(source, fil).is_ok() {
                            proposer(Port::Sortie(source), e.point_sortie(pos, j));
                        }
                    }
                }
            }
        }
        meilleur.map(|(_, port)| port)
    }

    /// Redresse la hauteur d'un élément déplacé pour qu'un de ses fils devienne droit.
    fn aimanter(a: &Apparence, at: &Atelier, id: Id, cible: Point) -> (Point, bool) {
        let Some(e) = at.doc.element(id) else {
            return (cible, false);
        };
        let mut candidats = Vec::new();
        for (i, s) in e.entrees.iter().enumerate() {
            let Some(s) = s else { continue };
            if let Some(src) = at.doc.element(s.id).filter(|src| src.id != id) {
                candidats.push(src.pos.y + src.decalage_sortie(s.sortie) - e.decalage_entree(i));
            }
        }
        // Ce qui lit l'élément : seulement s'il n'y en a pas trop (un grand plan).
        if at.doc.elements.len() <= 600 {
            for o in at.doc.elements.iter().filter(|o| o.id != id) {
                for (k, s) in o.entrees.iter().enumerate() {
                    if let Some(s) = s.filter(|s| s.id == id) {
                        candidats
                            .push(o.pos.y + o.decalage_entree(k) - e.decalage_sortie(s.sortie));
                    }
                }
            }
        }
        let proche = candidats
            .into_iter()
            .map(|y| ((y - cible.y).abs(), y))
            .filter(|(d, _)| *d <= a.manipulation.aimant)
            .min_by(|x, y| x.0.total_cmp(&y.0));
        match proche {
            Some((_, y)) => (Point::nouveau(cible.x, y), true),
            None => (cible, false),
        }
    }

    /// La place de tout le circuit à l'écran, avec sa marge, en points.
    pub fn zone_du_circuit(&self, at: &Atelier) -> Rect {
        let b = Self::boite(at);
        Rect::from_min_max(
            self.cam.ecran(Point::nouveau(b.min.x, b.min.y)),
            self.cam.ecran(Point::nouveau(b.max.x, b.max.y)),
        )
    }

    /// La boîte qui contient tout le circuit, en unités du plan.
    fn boite(at: &Atelier) -> Rect {
        let mut r = Rect::NOTHING;
        for e in &at.doc.elements {
            let (g, h, d, b) = e.corps(e.pos);
            r = r.union(
                Rect::from_min_max(Pos2::new(g, h), Pos2::new(d, b)).expand2(Vec2::new(24.0, 32.0)),
            );
        }
        if r.is_positive() {
            r.expand2(Vec2::new(16.0, 6.0))
        } else {
            Rect::from_min_size(Pos2::ZERO, Vec2::new(800.0, 500.0))
        }
    }

    /// Une image : gestes, animations, dessin.
    pub fn montrer(
        &mut self,
        ui: &mut egui::Ui,
        a: &Apparence,
        at: &mut Atelier,
        anims: &mut Animations,
        d: &Disposition,
        dt: f32,
    ) {
        ui.interact(d.ecran, egui::Id::unique("plan"), Sense::click_and_drag());
        self.en_mouvement = false;
        self.libre = d.libre;
        let maintenant = at.horloge;

        // On vient d'ouvrir une brique, d'en sortir ou de changer de projet : tout ce que le
        // plan retenait de l'ancien circuit est oublié, et la vue se recadre.
        let deplace = self.lieu_vu != at.deplacements;
        if deplace {
            self.lieu_vu = at.deplacements;
            self.affiche.clear();
            self.ondes.clear();
            self.eclats.clear();
            self.naissances.clear();
            self.disparus.clear();
            self.fils_partis.clear();
            self.fils_nes.clear();
            self.cablage_connu = false;
            self.entree_demandee = None;
            self.faisceaux.clear();
            self.boites.clear();
            self.cle_routage = 0;
            self.geste = Geste::Aucun;
            self.survol = Survol::Rien;
            self.initialise = false;
            self.cadrage_demande = Some(true);
        }

        // Cadrage demandé (au lancement, touche F, double-clic sur le fond).
        if let Some(direct) = self.cadrage_demande.take() {
            let zone = Rect::from_min_max(
                d.libre.min + Vec2::new(20.0, 56.0),
                d.libre.max - Vec2::new(20.0, d.hauteur_pilule + 16.0),
            );
            self.cam.cadrer(
                Self::boite(at),
                if zone.is_positive() { zone } else { d.libre },
                &a.manipulation,
                direct,
            );
        }
        // En remontant d'une brique, tout se resserre vers elle : on sait maintenant où elle est.
        if let (true, Some(plongee), Trajet::Sortie(Some(id))) =
            (deplace, &mut self.plongee, at.trajet)
        {
            if let Some(e) = at.doc.element(id) {
                plongee.foyer = self.cam.ecran(e.pos);
                if let Some(fantome) = &mut plongee.fantome {
                    fantome.centre = plongee.foyer;
                }
            }
        }

        self.gestes(ui, a, at, d.ecran, maintenant);

        // Positions affichées : chaque élément glisse vers sa position réelle.
        let f = if a.mouvement.lissage_element > 0.0 {
            1.0 - (-dt / a.mouvement.lissage_element).exp()
        } else {
            1.0
        };
        // Les éléments retirés (supprimés, annulés, rendus à la palette) s'effacent en rapetissant.
        let mut partis = Vec::new();
        self.affiche.retain(|id, vu| {
            let reste = at.doc.element(*id).is_some();
            if !reste {
                partis.push((*vu, maintenant));
            }
            reste
        });
        if !a.mouvement.sobre && partis.len() <= ANIMES_MAX {
            self.disparus.extend(partis);
        }
        for e in &at.doc.elements {
            let neuf = !self.affiche.contains_key(&e.id);
            let vu = self.affiche.entry(e.id).or_insert_with(|| Vu {
                pos: e.pos,
                corps: e.corps(Point::default()),
                rond: e.genre == Genre::Sortie,
            });
            let p = &mut vu.pos;
            if neuf && self.initialise {
                self.naissances.insert(e.id, maintenant);
            }
            if p.distance(e.pos) < 0.05 {
                *p = e.pos;
            } else {
                *p = Point::nouveau(p.x + (e.pos.x - p.x) * f, p.y + (e.pos.y - p.y) * f);
                self.en_mouvement = true;
            }
        }
        self.initialise = true;
        self.en_mouvement |= self.cam.animer(dt, &a.mouvement);

        // Portes qui viennent de basculer : un éclat.
        if at.session.t != self.t_vu {
            self.t_vu = at.session.t;
            for id in &at.session.tirs {
                self.eclats.insert(*id, maintenant);
            }
        }
        self.eclats.retain(|_, t0| maintenant - *t0 < 1.0);
        let apparition = f64::from(a.mouvement.duree(a.mouvement.apparition));
        self.naissances
            .retain(|_, t0| maintenant - *t0 < apparition);
        self.en_mouvement |= !self.naissances.is_empty();

        // Front du courant : quand une source change de valeur, la nouvelle valeur parcourt ses fils.
        self.ondes
            .retain(|s, _| at.doc.largeur_source(*s).is_some());
        let rapide = at.rapide(a).is_some();
        for s in at.doc.prises() {
            let v = at.valeur(s);
            match self.ondes.get_mut(&s) {
                // En simulation rapide, les fils montrent leur valeur sans front qui court.
                Some(o) if o.vers != v && rapide => {
                    *o = Onde {
                        de: v,
                        vers: v,
                        debut: maintenant - 10.0,
                    }
                }
                Some(o) if o.vers != v => {
                    *o = Onde {
                        de: o.vers,
                        vers: v,
                        debut: maintenant,
                    }
                }
                Some(_) => {}
                None => {
                    self.ondes.insert(
                        s,
                        Onde {
                            de: v,
                            vers: v,
                            debut: maintenant - 10.0,
                        },
                    );
                }
            }
        }

        let toile = Toile::nouvelle(ui.painter().with_clip_rect(d.ecran), a);
        self.dessiner(&toile, a, at, anims, d, dt);
        // Le double-clic sur une brique l'ouvre maintenant que le plan qu'on quitte est dessiné :
        // il pourra s'effacer pendant la plongée.
        if let Some(id) = self.entree_demandee.take() {
            at.entrer(id);
        }

        let curseur = match (self.geste, self.survol) {
            (Geste::Element { .. }, _) => CursorIcon::Grabbing,
            (Geste::Fil { .. } | Geste::Cadre { .. }, _) => CursorIcon::Crosshair,
            (Geste::Plan { parcouru, .. }, _) if parcouru > 4.0 => CursorIcon::Grabbing,
            (_, Survol::Port(_)) => CursorIcon::Crosshair,
            (_, Survol::Element(id))
                if at
                    .doc
                    .element(id)
                    .is_some_and(|e| matches!(e.genre, Genre::Entree | Genre::Clavier)) =>
            {
                CursorIcon::PointingHand
            }
            (_, Survol::Element(_)) => CursorIcon::Grab,
            (_, Survol::Fil(_)) => CursorIcon::PointingHand,
            _ => CursorIcon::Default,
        };
        if curseur != CursorIcon::Default {
            ui.ctx().set_cursor_icon(curseur);
        }
    }

    fn gestes(
        &mut self,
        ui: &egui::Ui,
        a: &Apparence,
        at: &mut Atelier,
        ecran: Rect,
        maintenant: f64,
    ) {
        let (pointeur, appui, enfonce, delta, vitesse, molette, pincement, tactile) =
            ui.input(|i| {
                (
                    i.pointer.latest_pos(),
                    i.pointer.primary_pressed()
                        || i.pointer.button_pressed(egui::PointerButton::Middle),
                    i.pointer.any_down(),
                    i.pointer.delta(),
                    i.pointer.velocity(),
                    i.smooth_scroll_delta.y,
                    i.zoom_delta(),
                    i.multi_touch().map(|m| (m.center_pos, m.translation_delta)),
                )
            });
        let milieu = ui.input(|i| i.pointer.button_pressed(egui::PointerButton::Middle));
        let majuscule = ui.input(|i| i.modifiers.shift);
        // Sur un écran tactile, le pointeur disparaît à l'instant où le doigt se lève : le geste
        // se termine alors à la dernière position connue.
        let en_geste = self.geste != Geste::Aucun;
        let (pointeur, sur_plan) = match (pointeur, self.dernier_pointeur) {
            (Some(p), _) => (
                Some(p),
                ecran.contains(p) && !self.obstacles.iter().any(|r| r.contains(p)),
            ),
            (None, Some((p, sur_plan))) if en_geste => (Some(p), sur_plan),
            (None, _) => (None, false),
        };
        self.dernier_pointeur = pointeur.map(|p| (p, sur_plan));

        // Zoom : molette autour du pointeur, pincement autour du centre des doigts.
        if let Some((centre, glisse)) = tactile {
            self.terminer(a, at, None, Vec2::ZERO, maintenant);
            self.cam.deplacer(glisse);
            self.cam.zoomer(centre, pincement, &a.manipulation, true);
            self.en_mouvement = true;
            return;
        }
        if let Some(p) = pointeur.filter(|_| sur_plan || self.geste != Geste::Aucun) {
            if molette != 0.0 {
                self.cam.zoomer(
                    p,
                    (molette * a.manipulation.molette).exp(),
                    &a.manipulation,
                    false,
                );
            }
            if pincement != 1.0 {
                self.cam.zoomer(p, pincement, &a.manipulation, true);
            }
        }

        let Some(p) = pointeur else {
            if self.geste != Geste::Aucun && !enfonce {
                self.terminer(a, at, None, Vec2::ZERO, maintenant);
            }
            self.survol = Survol::Rien;
            return;
        };
        let m = self.cam.monde(p);
        self.survol = if sur_plan || self.geste != Geste::Aucun {
            self.viser(a, at, m)
        } else {
            Survol::Rien
        };

        // Début d'un geste.
        if appui && sur_plan && self.geste == Geste::Aucun {
            self.cam.vx = 0.0;
            self.cam.vy = 0.0;
            self.geste = match self.survol {
                _ if milieu => Geste::Plan {
                    parcouru: 0.0,
                    fil: None,
                },
                Survol::Port(Port::Sortie(source)) => Geste::Fil {
                    depuis: Port::Sortie(source),
                    aimant: None,
                },
                Survol::Port(Port::Entree(fil)) => match at.doc.debrancher(fil) {
                    // Reprendre un fil par son arrivée : il repart de sa source, à rebrancher ailleurs.
                    Some(source) => Geste::Fil {
                        depuis: Port::Sortie(source),
                        aimant: Some(Port::Entree(fil)),
                    },
                    None => Geste::Fil {
                        depuis: Port::Entree(fil),
                        aimant: None,
                    },
                },
                Survol::Element(id) if majuscule => {
                    at.basculer_selection(id);
                    Geste::Inerte
                }
                Survol::Element(id) => {
                    // Un élément déjà dans la sélection l'emmène tout entière avec lui.
                    if !at.est_selectionne(id) {
                        at.selectionner(id);
                    }
                    if at.doc.element(id).is_some_and(Element::est_poussoir) {
                        at.appuyer(id, true);
                        self.poussoir = Some(id);
                    }
                    let pos = self.position(at, id);
                    Geste::Element {
                        id,
                        decalage: Vec2::new(pos.x - m.x, pos.y - m.y),
                        appui: p,
                        bouge: false,
                        nouveau: false,
                        aimante: false,
                    }
                }
                Survol::Fil(fil) => Geste::Plan {
                    parcouru: 0.0,
                    fil: Some(fil),
                },
                Survol::Rien if majuscule => Geste::Cadre {
                    depart: m,
                    ajoute: true,
                },
                Survol::Rien => Geste::Plan {
                    parcouru: 0.0,
                    fil: None,
                },
            };
        }

        // Suite du geste.
        match &mut self.geste {
            Geste::Aucun | Geste::Inerte => {}
            Geste::Cadre { .. } => self.en_mouvement = true,
            Geste::Plan { parcouru, .. } => {
                *parcouru += delta.length();
                if *parcouru > 4.0 {
                    self.cam.deplacer(delta);
                    self.en_mouvement = true;
                }
            }
            Geste::Element {
                id,
                decalage,
                appui,
                bouge,
                aimante,
                ..
            } => {
                if !*bouge && p.distance(*appui) > 4.0 {
                    *bouge = true;
                }
                if *bouge {
                    let cible = Point::nouveau(m.x + decalage.x, m.y + decalage.y);
                    let (cible, colle) = Self::aimanter(a, at, *id, cible);
                    *aimante = colle;
                    let avant = at.doc.element(*id).map_or(cible, |e| e.pos);
                    let (dx, dy) = (cible.x - avant.x, cible.y - avant.y);
                    // Toute la sélection suit l'élément tenu.
                    for autre in at.elements_selectionnes().to_vec() {
                        if let Some(e) = at.doc.element_mut(autre).filter(|e| e.id != *id) {
                            e.pos = Point::nouveau(e.pos.x + dx, e.pos.y + dy);
                        }
                    }
                    if let Some(e) = at.doc.element_mut(*id) {
                        e.pos = cible;
                    }
                }
            }
            Geste::Fil { depuis, .. } => {
                let depuis = *depuis;
                let trouve = self.accrocher(a, at, depuis, m);
                if let Geste::Fil { aimant, .. } = &mut self.geste {
                    *aimant = trouve;
                }
                self.en_mouvement = true;
            }
        }

        if !enfonce {
            if let Some(id) = self.poussoir.take() {
                at.appuyer(id, false);
            }
        }
        if !enfonce && self.geste != Geste::Aucun {
            self.terminer(a, at, sur_plan.then_some(p), vitesse, maintenant);
        }
    }

    /// Termine le geste en cours. `lache` : position du pointeur s'il est encore sur le plan.
    fn terminer(
        &mut self,
        a: &Apparence,
        at: &mut Atelier,
        lache: Option<Pos2>,
        vitesse: Vec2,
        maintenant: f64,
    ) {
        match std::mem::replace(&mut self.geste, Geste::Aucun) {
            Geste::Aucun | Geste::Inerte => {}
            Geste::Cadre { depart, ajoute } => {
                let Some(p) = lache.or(self.dernier_pointeur.map(|(p, _)| p)) else {
                    return;
                };
                let fin = self.cam.monde(p);
                let cadre =
                    Rect::from_two_pos(Pos2::new(depart.x, depart.y), Pos2::new(fin.x, fin.y));
                let mut ids: Vec<Id> = if ajoute {
                    at.elements_selectionnes().to_vec()
                } else {
                    Vec::new()
                };
                ids.extend(
                    at.doc
                        .elements
                        .iter()
                        .filter(|e| cadre.contains(Pos2::new(e.pos.x, e.pos.y)))
                        .map(|e| e.id),
                );
                at.selectionner_tous(ids);
            }
            Geste::Plan { parcouru, fil } => {
                if parcouru > 4.0 {
                    if vitesse.length() > 60.0 {
                        self.cam.vx = vitesse.x;
                        self.cam.vy = vitesse.y;
                    }
                } else if let Some(fil) = fil {
                    at.selection = Some(Selection::Fil(fil));
                } else if let Some(p) = lache {
                    at.selection = None;
                    let double = self
                        .dernier_clic
                        .is_some_and(|(t, q)| maintenant - t < 0.4 && q.distance(p) < 8.0);
                    self.dernier_clic = if double { None } else { Some((maintenant, p)) };
                    if double {
                        self.cadrer(false);
                    }
                }
            }
            Geste::Element {
                id,
                bouge,
                nouveau,
                aimante,
                ..
            } => {
                if nouveau && lache.is_none() {
                    // Relâché sur un panneau : l'élément retourne dans la palette, et le plan
                    // redevient exactement ce qu'il était (rien à annuler).
                    match self.avant_pose.take() {
                        Some(avant) => at.doc = avant,
                        None => {
                            at.doc.retirer(id);
                        }
                    }
                    at.selection = None;
                    at.apres_modification(a);
                } else if !bouge {
                    // Un clic : il ne garde que cet élément dans la sélection.
                    at.selectionner(id);
                    let double = self
                        .dernier_clic_element
                        .is_some_and(|(t, e)| e == id && maintenant - t < 0.45);
                    self.dernier_clic_element = (!double).then_some((maintenant, id));
                    match at.doc.element(id).map(|e| (e.genre, e.est_poussoir())) {
                        Some((Genre::Entree, false)) => at.basculer(id),
                        Some((Genre::Brique, _)) if double => self.entree_demandee = Some(id),
                        _ => {}
                    }
                } else {
                    let g = a.manipulation.grille;
                    let mut deplaces = at.elements_selectionnes().to_vec();
                    if !deplaces.contains(&id) {
                        deplaces.push(id);
                    }
                    for autre in deplaces {
                        if let Some(e) = at.doc.element_mut(autre) {
                            e.pos = Point::nouveau(
                                arrondi(e.pos.x, g),
                                if aimante && autre == id {
                                    e.pos.y
                                } else {
                                    arrondi(e.pos.y, g)
                                },
                            );
                        }
                    }
                    at.apres_modification(a);
                }
            }
            Geste::Fil { depuis, aimant } => {
                match (depuis, aimant) {
                    (Port::Sortie(source), Some(Port::Entree(fil)))
                    | (Port::Entree(fil), Some(Port::Sortie(source))) => {
                        match at.doc.verifier(source, fil) {
                            Ok(()) => {
                                at.doc.relier(source, fil);
                                at.selection = Some(Selection::Fil(fil));
                            }
                            Err(Refus::Largeur { source, entree }) => at.dire(&tr!("Ce fil porte {source} bit{v} et cette entrée en attend {entree} : pose un découpeur ou une jonction entre les deux.", source = source, entree = entree, v = if source > 1 { "s" } else { "" })),
                            Err(Refus::Inconnu) => {}
                        }
                    }
                    _ => {}
                }
                at.apres_modification(a);
            }
        }
    }

    fn dessiner(
        &mut self,
        t: &Toile,
        a: &Apparence,
        at: &Atelier,
        anims: &mut Animations,
        d: &Disposition,
        dt: f32,
    ) {
        let (c, f, cam) = (t.c, &a.formes, self.cam);
        let m = &a.mouvement;
        let k = cam.k;
        let maintenant = at.horloge;
        let anime = !a.mouvement.sobre;

        // Fond : couleur, lueur, grille de points.
        t.p.rect_filled(d.ecran, 0.0, c.fond.c());
        t.p.add(halo(
            Rect::from_center_size(
                Pos2::new(d.ecran.center().x, d.ecran.top() + d.ecran.height() * 0.3),
                d.ecran.size() * Vec2::new(1.3, 1.25),
            ),
            c.fond_halo,
        ));
        let mut pas = f.pas_grille * k;
        while pas < 16.0 {
            pas *= 2.0;
        }
        let mut grille = Mesh::default();
        let mut y = d.ecran.top() + (cam.y - d.ecran.top()).rem_euclid(pas);
        while y < d.ecran.bottom() {
            let mut x = d.ecran.left() + (cam.x - d.ecran.left()).rem_euclid(pas);
            while x < d.ecran.right() {
                grille.add_colored_rect(
                    Rect::from_center_size(Pos2::new(x, y), Vec2::splat(2.0)),
                    c.grille.c(),
                );
                x += pas;
            }
            y += pas;
        }
        t.p.add(Shape::mesh(grille));
        // À partir d'ici, le circuit : ce qui grossit ou rapetisse quand on change de plan.
        let debut = calques::repere(&t.p);

        // Ce qui est visible, en unités du plan : le reste n'est pas dessiné.
        let (hg, bd) = (cam.monde(d.ecran.min), cam.monde(d.ecran.max));
        let vue = Rect::from_min_max(Pos2::new(hg.x, hg.y), Pos2::new(bd.x, bd.y)).expand(60.0);
        // Très dézoomé, on ne dessine plus que l'essentiel.
        let detail = k >= 0.28;
        let mut dessines = (0usize, 0usize);

        // Fils.
        self.router(at, anime);

        // Les fils qu'on vient de retirer se rétractent vers leur source en s'effaçant.
        let disparition = m.disparition.max(0.01);
        self.fils_partis
            .retain(|(_, t0)| maintenant - *t0 < f64::from(disparition));
        for (sommets, t0) in &self.fils_partis {
            let p = (((maintenant - *t0) as f32) / disparition).clamp(0.0, 1.0);
            let ligne = routage::arrondir(sommets, f.rayon_fil);
            let reste = routage::portion(
                &ligne,
                0.0,
                routage::longueur(&ligne) * (1.0 - Courbe::Entree.en(p)),
            );
            if reste.len() >= 2 {
                t.p.add(Shape::line(
                    reste.iter().map(|q| cam.ecran(*q)).collect(),
                    Stroke::new(
                        (f.fil * k).max(1.0),
                        c.zero.vers(c.danger, 0.6).alpha(1.0 - p),
                    ),
                ));
            }
            self.en_mouvement = true;
        }
        // Les fils qu'on vient de brancher se tracent de leur source à leur arrivée.
        let trace = m.trace_fil.max(0.01);
        self.fils_nes
            .retain(|_, t0| maintenant - *t0 < f64::from(trace + m.eclat));
        self.en_mouvement |= !self.fils_nes.is_empty();

        let faisceaux = std::mem::take(&mut self.faisceaux);
        let boites = std::mem::take(&mut self.boites);
        let duree_front = (at.duree_pas(a) * a.mouvement.part_front).clamp(0.12, 0.9);
        let fil_choisi = at.fil_selectionne();
        let mut courant = false;
        for (faisceau, boites) in faisceaux.iter().zip(&boites) {
            if faisceau.fils.is_empty() {
                continue;
            }
            let onde = self.ondes.get(&faisceau.source).copied().unwrap_or(Onde {
                de: Valeur::X,
                vers: Valeur::X,
                debut: 0.0,
            });
            let p = (((maintenant - onde.debut) as f32) / duree_front).clamp(0.0, 1.0);
            let bus = at.doc.largeur_source(faisceau.source).unwrap_or(1) > 1;
            let mut vu = false;
            for ((fil, sommets), boite) in faisceau.fils.iter().zip(boites) {
                if !boite.intersects(vue) {
                    // Hors de l'écran : pas dessiné, mais toujours là pour le pointeur.
                    continue;
                }
                vu = true;
                dessines.1 += 1;
                let mut ligne = if detail {
                    routage::arrondir(sommets, f.rayon_fil)
                } else {
                    sommets.clone()
                };
                let ne = (!self.fils_nes.is_empty())
                    .then(|| self.fils_nes.get(&(faisceau.source, *fil)))
                    .flatten();
                if let Some(t0) = ne {
                    let age = (maintenant - *t0) as f32;
                    if age < trace {
                        let longueur = routage::longueur(&ligne);
                        ligne = routage::portion(
                            &ligne,
                            0.0,
                            longueur * Courbe::Sortie.en(age / trace),
                        );
                        if ligne.len() < 2 {
                            continue;
                        }
                    } else if let Some(bout) = ligne.last() {
                        // Arrivé : un anneau s'ouvre sur le point de connexion.
                        let p = ((age - trace) / m.eclat.max(0.01)).clamp(0.0, 1.0);
                        t.p.circle_stroke(
                            cam.ecran(*bout),
                            (7.0 + 15.0 * Courbe::Sortie.en(p)) * k,
                            Stroke::new(2.5 * k * (1.0 - p), c.accent_clair.alpha(1.0 - p)),
                        );
                    }
                }
                let survole = self.survol == Survol::Fil(*fil) && self.geste == Geste::Aucun;
                // Un fil que personne ne survole n'a pas besoin d'animation.
                let cle = egui::Id::unique(("fil", fil.puits, fil.entree));
                let tau = a.mouvement.lissage_ui;
                let gras = if survole {
                    anims.vers(cle, 1.0, dt, tau)
                } else {
                    anims.vers_si_connu(cle, 0.0, dt, tau)
                };
                courant |= self.dessiner_fil(
                    t,
                    a,
                    &ligne,
                    onde,
                    p,
                    fil_choisi == Some(*fil),
                    gras,
                    bus,
                    detail,
                    maintenant,
                );
            }
            if vu && p < 1.0 {
                self.en_mouvement = true;
            }
            if vu && detail {
                let v = if p >= 0.5 { onde.vers } else { onde.de };
                let rayon = f.fil * if bus { 1.35 } else { 0.95 } * k;
                for j in &faisceau.jonctions {
                    t.p.circle_filled(cam.ecran(*j), rayon, couleur_valeur(c, v));
                }
            }
        }
        if courant && anime {
            self.en_mouvement = true;
        }

        // Les éléments qu'on vient de retirer : leur silhouette rapetisse et s'efface.
        self.disparus
            .retain(|(_, t0)| maintenant - *t0 < f64::from(disparition));
        for (vu, t0) in &self.disparus {
            let p = (((maintenant - *t0) as f32) / disparition).clamp(0.0, 1.0);
            let echelle = k * (1.0 - 0.3 * Courbe::Entree.en(p));
            let centre = cam.ecran(vu.pos);
            let (gauche, haut, droite, bas) = vu.corps;
            let r = Rect::from_min_max(
                centre + Vec2::new(gauche, haut) * echelle,
                centre + Vec2::new(droite, bas) * echelle,
            );
            let rayon = if vu.rond {
                r.height() / 2.0
            } else {
                f.rayon_porte * echelle
            };
            t.p.rect(
                r,
                rayon,
                c.carte.alpha(0.7 * (1.0 - p)),
                Stroke::new(2.0 * echelle, c.danger.alpha(1.0 - p)),
                StrokeKind::Middle,
            );
            self.en_mouvement = true;
        }

        // Éléments.
        let (depuis, aimant) = match self.geste {
            Geste::Fil { depuis, aimant } => (Some(depuis), aimant),
            _ => (None, None),
        };
        // Une porte « attend » quand sa sortie n'a pas encore rejoint ce que valent ses entrées.
        let suivre_attente = at.vue_t.is_none() && at.rapide(a).is_none();
        for e in &at.doc.elements {
            let pos = self.position(at, e.id);
            let (g, h, dr, b) = e.corps(pos);
            let corps = Rect::from_min_max(Pos2::new(g, h), Pos2::new(dr, b)).expand(30.0);
            if !corps.intersects(vue) {
                continue;
            }
            dessines.0 += 1;
            self.dessiner_element(
                t,
                a,
                at,
                anims,
                e,
                suivre_attente
                    && e.genre == Genre::Nand
                    && at.session.cible(&at.doc, e.id) != at.session.valeur(Source::de(e.id)),
                depuis,
                aimant,
                detail,
                dt,
            );
        }
        self.dessines = dessines;

        // Étiquettes des signaux.
        if at.volets.etiquettes && k * 11.0 >= 6.0 {
            for faisceau in &faisceaux {
                let Some(e) = at.doc.element(faisceau.source.id) else {
                    continue;
                };
                // Un découpeur ou une jonction ne fait que répartir des fils : pas d'étiquette.
                // Une sortie de brique n'en a une que si elle est branchée.
                let etiquete = match e.genre {
                    Genre::Decoupe | Genre::Jonction => false,
                    Genre::Brique => !faisceau.fils.is_empty(),
                    _ => true,
                };
                let port = e.point_sortie(self.position(at, e.id), faisceau.source.sortie);
                if !etiquete || !vue.contains(Pos2::new(port.x, port.y)) {
                    continue;
                }
                let lecture = at.lecture(faisceau.source);
                let v = lecture.valeur();
                let texte = format!(
                    "{} = {}",
                    at.doc.nom_signal(faisceau.source),
                    lecture.texte()
                );
                let largeur = (t.largeur(&texte, Police::MonoGras, 11.0) + 14.0) * k;
                let dy = if e.broches_s.len() > 1 {
                    // Plusieurs sorties l'une sous l'autre : chaque étiquette se range juste
                    // au-dessus de son fil, entre deux sorties, sans mordre sur la voisine.
                    10.0 - PAS_BROCHE / 2.0
                } else if faisceau.etiquette_dessous {
                    21.0
                } else {
                    -21.0
                };
                // L'étiquette suit le fil : à gauche de la sortie d'un élément retourné.
                let x = if e.miroir {
                    port.x - 6.0 - largeur / k
                } else {
                    port.x + 6.0
                };
                let r = Rect::from_min_size(
                    cam.ecran(Point::nouveau(x, port.y + dy - 10.0)),
                    Vec2::new(largeur, 20.0 * k),
                );
                t.p.rect(
                    r,
                    7.0 * k,
                    c.carte.c(),
                    Stroke::new(1.0, c.bord.c()),
                    StrokeKind::Inside,
                );
                let couleur = match symbole(v) {
                    '1' => c.un.c(),
                    '0' => c.doux.c(),
                    _ => c.x.c(),
                };
                t.texte(
                    r.center(),
                    Align2::CENTER_CENTER,
                    &texte,
                    Police::MonoGras,
                    11.0 * k,
                    couleur,
                );
            }
        }

        self.faisceaux = faisceaux;
        self.boites = boites;

        // Cadre de sélection en cours.
        if let (Geste::Cadre { depart, .. }, Some(pointeur)) =
            (self.geste, t.p.ctx().pointer_latest_pos())
        {
            let r = Rect::from_two_pos(cam.ecran(depart), pointeur);
            t.p.rect(
                r,
                6.0,
                c.accent_clair.alpha(0.10),
                Stroke::new(1.5, c.accent_clair.c()),
                StrokeKind::Inside,
            );
        }

        // Fil en cours de tirage : des perles qui courent du bout fixé vers le pointeur.
        if let (Some(depuis), Some(pointeur)) = (depuis, t.p.ctx().pointer_latest_pos()) {
            let bout = |port: Port| match port {
                Port::Sortie(s) => at
                    .doc
                    .element(s.id)
                    .map(|e| e.sortie_en(self.position(at, s.id), s.sortie))
                    .unwrap_or_default(),
                port @ Port::Entree(_) => self.port_monde(at, port),
            };
            let fixe = bout(depuis);
            let libre = aimant.map_or(cam.monde(pointeur), bout);
            // Le sens de l'élément d'où part le fil (retourné ou non), et celui de l'élément
            // visé quand l'aimant en a trouvé un.
            let sens = |port: Port| {
                let id = match port {
                    Port::Sortie(s) => s.id,
                    Port::Entree(f) => f.puits,
                };
                at.doc.element(id).map_or(1.0, Element::sens)
            };
            let s_fixe = sens(depuis);
            let s_libre = aimant.map_or(s_fixe, sens);
            let sommets = match depuis {
                Port::Sortie(_) => routage::itineraire_oriente(fixe, s_fixe, libre, s_libre, None),
                Port::Entree(_) => routage::itineraire_oriente(libre, s_libre, fixe, s_fixe, None),
            };
            let ligne = routage::arrondir(&sommets, f.rayon_fil);
            let longueur = routage::longueur(&ligne);
            let sens = if matches!(depuis, Port::Sortie(_)) {
                1.0
            } else {
                -1.0
            };
            let glisse = if anime {
                ((maintenant * 26.0) as f32 * sens).rem_euclid(13.0)
            } else {
                0.0
            };
            let mut s = glisse;
            while s < longueur {
                if let Some(q) = routage::point_a(&ligne, s) {
                    t.p.circle_filled(cam.ecran(q), 2.5 * k.max(0.6), c.accent_clair.c());
                }
                s += 13.0;
            }
        }

        // Quête réussie : chaque lampe lance des ondes et des étincelles.
        if let Some(age) = at
            .fete
            .map(|t0| (maintenant - t0) as f32)
            .filter(|age| anime && *age < m.fete)
        {
            let p = (age / m.fete.max(0.01)).clamp(0.0, 1.0);
            self.en_mouvement = true;
            let mut foyers: Vec<(u64, Pos2)> = at
                .doc
                .elements
                .iter()
                .filter(|e| e.genre == Genre::Sortie)
                .take(12)
                .map(|e| (u64::from(e.id), cam.ecran(self.position(at, e.id))))
                .collect();
            if foyers.is_empty() {
                foyers.push((0, d.libre.center()));
            }
            for (graine, centre) in foyers {
                for retard in [0.0, 0.16] {
                    let q = (p - retard) / 0.55;
                    if q > 0.0 && q < 1.0 {
                        t.p.circle_stroke(
                            centre,
                            (30.0 + 95.0 * Courbe::Sortie.en(q)) * k,
                            Stroke::new(3.5 * k * (1.0 - q), c.un.alpha(0.85 * (1.0 - q))),
                        );
                    }
                }
                for i in 0..m.etincelles {
                    let g = graine.wrapping_mul(1_000_003).wrapping_add(i as u64);
                    let retard = 0.14 * hasard(g ^ 0x51);
                    let q = ((p - retard) / (0.75 - retard)).clamp(0.0, 1.0);
                    if q <= 0.0 || q >= 1.0 {
                        continue;
                    }
                    let angle =
                        std::f32::consts::TAU * (i as f32 + hasard(g)) / m.etincelles.max(1) as f32;
                    let portee = (70.0 + 110.0 * hasard(g ^ 0xA7)) * Courbe::Sortie.en(q);
                    let ici = centre
                        + Vec2::new(angle.cos() * portee, angle.sin() * portee + 55.0 * q * q) * k;
                    let couleur = [c.un, c.accent_clair, c.perle][i % 3];
                    t.p.circle_filled(
                        ici,
                        (2.0 + 2.4 * hasard(g ^ 0x3C)) * (1.0 - 0.6 * q) * k.max(0.6),
                        couleur.alpha(1.0 - q * q),
                    );
                }
            }
        }

        // Chapitre ou branche terminés : une pluie de confettis sur tout le plan, et un
        // bandeau qui arrive en rebondissant.
        if let Some((t0, titre, phrase)) = at
            .grande_fete
            .as_ref()
            .filter(|(t0, ..)| anime && ((maintenant - t0) as f32) < m.grande_fete)
        {
            let age = (maintenant - t0) as f32;
            let p = (age / m.grande_fete.max(0.01)).clamp(0.0, 1.0);
            self.en_mouvement = true;
            let zone = d.libre;
            for i in 0..m.confettis {
                let g = (i as u64).wrapping_mul(0x2545_F491_4F6C_DD1D) ^ 0xC0FE;
                let depart = 0.3 * hasard(g);
                let q = (p - depart) / (1.0 - depart);
                if q <= 0.0 || q >= 1.0 {
                    continue;
                }
                let chute = (q * (0.8 + 0.5 * hasard(g ^ 0x22))).min(1.0);
                let x = zone.left()
                    + zone.width() * hasard(g ^ 0x11)
                    + 22.0 * (age * (1.5 + 2.5 * hasard(g ^ 0x33)) + 6.3 * hasard(g ^ 0x44)).sin();
                let y = zone.top() - 16.0 + (zone.height() + 32.0) * Courbe::Douce.en(chute);
                let angle = age * (2.0 + 5.0 * hasard(g ^ 0x55)) + 6.3 * hasard(g ^ 0x66);
                let (ca, sa) = (angle.cos(), angle.sin());
                // Un confetti qui tourne : on le voit tantôt de face, tantôt de profil.
                let (lx, ly) = (
                    6.0,
                    3.0 * (age * 6.0 + 9.0 * hasard(g ^ 0x77)).cos().abs() + 0.8,
                );
                let points: Vec<Pos2> = [(-lx, -ly), (lx, -ly), (lx, ly), (-lx, ly)]
                    .iter()
                    .map(|(a, b)| Pos2::new(x + a * ca - b * sa, y + a * sa + b * ca))
                    .collect();
                let couleur = [c.un, c.accent_clair, c.perle, c.accent][i % 4];
                let opacite = ((1.0 - q) / 0.15).min(1.0);
                t.p.add(egui::Shape::convex_polygon(
                    points,
                    couleur.alpha(opacite),
                    Stroke::NONE,
                ));
            }
            // Le bandeau : il arrive en rebondissant, reste, puis s'efface.
            let arrivee = Courbe::Rebond.en(age / 0.55);
            let depart = ((m.grande_fete - age) / 0.45).clamp(0.0, 1.0);
            let largeur =
                t.largeur(titre, Police::Titre, 22.0)
                    .max(t.largeur(phrase, Police::Corps, 14.0))
                    + 120.0;
            let largeur = largeur.min(zone.width() - 32.0);
            let centre = Pos2::new(zone.center().x, zone.top() + 96.0);
            let rect =
                Rect::from_center_size(centre, Vec2::new(largeur, 86.0) * (0.6 + 0.4 * arrivee));
            let pouls = m.pouls(maintenant, 0.6);
            t.p.rect_stroke(
                rect.expand(4.0 + 8.0 * pouls),
                22.0 + 8.0 * pouls,
                Stroke::new(2.0, c.un.alpha(0.5 * (1.0 - pouls) * depart)),
                StrokeKind::Outside,
            );
            t.p.rect(
                rect,
                18.0,
                c.carte.alpha(0.97 * depart),
                Stroke::new(2.0, c.un.alpha(depart)),
                StrokeKind::Inside,
            );
            if arrivee > 0.6 {
                let rond = Pos2::new(rect.left() + 44.0, rect.center().y);
                t.p.circle_filled(rond, 22.0, c.un.alpha(0.18 * depart));
                t.icone(Icone::Coche, rond, 24.0, c.un.alpha(depart));
                let lt = rect.width() - 92.0;
                t.texte(
                    Pos2::new(rect.left() + 78.0, rect.center().y - 12.0),
                    Align2::LEFT_CENTER,
                    &t.abreger(titre, Police::Titre, 22.0, lt),
                    Police::Titre,
                    22.0,
                    c.texte.alpha(depart),
                );
                t.texte(
                    Pos2::new(rect.left() + 78.0, rect.center().y + 16.0),
                    Align2::LEFT_CENTER,
                    &t.abreger(phrase, Police::Corps, 14.0, lt),
                    Police::Corps,
                    14.0,
                    c.doux.alpha(depart),
                );
            }
        }

        // Plongée dans une brique, ou remontée : le circuit qui arrive part de la brique (ou
        // se resserre vers elle), celui qu'on quitte s'efface par-dessus en grossissant.
        let fin = calques::repere(&t.p);
        self.plage = (debut, fin);
        if let Some(plongee) = &self.plongee {
            let p = m.avancement(maintenant - plongee.ne, m.plongee, m.courbe_plongee);
            if p >= 1.0 {
                self.plongee = None;
            } else {
                calques::voiler(
                    &t.p,
                    debut,
                    p,
                    calques::zoom_autour(plongee.foyer, plongee.grossit.powf(p - 1.0)),
                );
                if let Some(fantome) = &plongee.fantome {
                    fantome.peindre(
                        &t.p,
                        (1.0 - p) * (1.0 - p),
                        plongee.grossit.powf(p),
                        Vec2::ZERO,
                    );
                }
                self.en_mouvement = true;
            }
        }
    }

    /// À appeler à la fin de l'image, une fois les panneaux et le clavier traités : si le plan
    /// affiché vient de changer parce qu'on entre dans une brique ou qu'on en sort, garde
    /// l'image du circuit qu'on quitte, pour la plongée. Visuel seulement.
    pub fn quitter(&mut self, p: &egui::Painter, a: &Apparence, at: &Atelier) {
        if at.deplacements == self.lieu_vu {
            return;
        }
        let m = &a.mouvement;
        let (foyer, grossit) = match at.trajet {
            Trajet::Entree(id) => (
                self.affiche
                    .get(&id)
                    .map_or(self.libre.center(), |vu| self.cam.ecran(vu.pos)),
                m.zoom_plongee,
            ),
            Trajet::Dedans => (self.libre.center(), m.zoom_plongee),
            // Le foyer sera la brique d'où l'on sort, une fois le plan du dessus cadré.
            Trajet::Sortie(_) => (self.libre.center(), 1.0 / m.zoom_plongee),
            Trajet::Autre => {
                self.plongee = None;
                return;
            }
        };
        self.plongee = (m.duree(m.plongee) > 0.0).then(|| Plongee {
            fantome: Fantome::figer(p, self.plage.0, self.plage.1, at.horloge, foyer),
            ne: at.horloge,
            foyer,
            grossit,
        });
    }

    /// Vrai pendant qu'on plonge dans une brique ou qu'on en remonte.
    pub fn en_plongee(&self) -> bool {
        self.plongee.is_some()
    }

    /// Retrace les fils si le câblage, la forme d'un élément ou une position affichée a changé
    /// depuis la dernière image. Sur un grand plan immobile, rien n'est recalculé.
    fn router(&mut self, at: &Atelier, anime: bool) {
        const PREMIER: u64 = 0x0000_0100_0000_01b3;
        let mut h: u64 = 0xcbf2_9ce4_8422_2325 ^ at.doc.elements.len() as u64;
        // Le câblage seul, sans les positions : il dit si des fils ont été posés ou retirés.
        let mut hc: u64 = h;
        for e in &at.doc.elements {
            let pos = self.affiche.get(&e.id).map_or(e.pos, |vu| vu.pos);
            let mut meler = |x: u64| h = (h ^ x).wrapping_mul(PREMIER);
            meler(u64::from(e.id));
            meler(u64::from(pos.x.to_bits()) << 32 | u64::from(pos.y.to_bits()));
            meler(u64::from(e.largeur().to_bits()) << 32 | u64::from(e.hauteur().to_bits()));
            meler((e.broches_e.len() as u64) << 32 | e.broches_s.len() as u64);
            // Retourner un élément change le côté de ses broches, donc ses fils.
            meler(u64::from(e.miroir));
            hc = (hc ^ u64::from(e.id)).wrapping_mul(PREMIER);
            for s in &e.entrees {
                let x = s.map_or(u64::MAX, |s| u64::from(s.id) << 16 | s.sortie as u64);
                h = (h ^ x).wrapping_mul(PREMIER);
                hc = (hc ^ x).wrapping_mul(PREMIER);
            }
        }
        if h == self.cle_routage && !self.faisceaux.is_empty() {
            return;
        }
        self.cle_routage = h;
        let anciens = (anime && self.cablage_connu && hc != self.cle_cablage)
            .then(|| std::mem::take(&mut self.faisceaux));
        self.cle_cablage = hc;
        self.cablage_connu = true;
        self.faisceaux = routage::tracer(&at.doc, |id| self.position(at, id));
        if let Some(anciens) = anciens {
            // Le câblage a changé : quels fils sont partis, lesquels sont nouveaux ?
            let maintenant = at.horloge;
            let nouveaux: HashSet<(Source, Fil)> = self
                .faisceaux
                .iter()
                .flat_map(|f| f.fils.iter().map(|(fil, _)| (f.source, *fil)))
                .collect();
            let mut vieux = HashSet::new();
            let mut partis = Vec::new();
            for f in &anciens {
                for (fil, sommets) in &f.fils {
                    vieux.insert((f.source, *fil));
                    if !nouveaux.contains(&(f.source, *fil)) {
                        partis.push((sommets.clone(), maintenant));
                    }
                }
            }
            if partis.len() <= ANIMES_MAX {
                self.fils_partis.extend(partis);
            }
            let nes: Vec<(Source, Fil)> = nouveaux.difference(&vieux).copied().collect();
            if nes.len() <= ANIMES_MAX {
                self.fils_nes
                    .extend(nes.into_iter().map(|cle| (cle, maintenant)));
            }
        }
        self.boites = self
            .faisceaux
            .iter()
            .map(|f| {
                f.fils
                    .iter()
                    .map(|(_, sommets)| {
                        let mut r = Rect::NOTHING;
                        for q in sommets {
                            r.extend_with(Pos2::new(q.x, q.y));
                        }
                        r
                    })
                    .collect()
            })
            .collect();
    }

    /// Dessine un fil. Renvoie `true` si du courant y circule (il faut alors animer les perles).
    #[allow(clippy::too_many_arguments)]
    fn dessiner_fil(
        &self,
        t: &Toile,
        a: &Apparence,
        ligne: &[Point],
        onde: Onde,
        p: f32,
        choisi: bool,
        gras: f32,
        bus: bool,
        detail: bool,
        maintenant: f64,
    ) -> bool {
        let (c, f, cam) = (t.c, &a.formes, &self.cam);
        let k = cam.k;
        let ecran = |pts: &[Point]| -> Vec<Pos2> { pts.iter().map(|q| cam.ecran(*q)).collect() };
        let longueur = routage::longueur(ligne);
        let tout = ecran(ligne);
        // Un bus est plus épais qu'un fil simple.
        let epaisseur = if bus { f.fil * 1.6 } else { f.fil };
        let largeur = ((epaisseur + 3.0 * gras) * k).max(1.0);
        if choisi {
            t.p.add(Shape::line(
                tout.clone(),
                Stroke::new((epaisseur + 5.0) * k, c.zero.vers(c.accent_clair, 0.6).c()),
            ));
        }
        t.p.add(Shape::line(tout, Stroke::new(largeur, c.zero.c())));
        let front = p * longueur;
        let mut courant = false;
        for (debut, fin, v) in [(0.0, front, onde.vers), (front, longueur, onde.de)] {
            if fin - debut < 0.01 {
                continue;
            }
            let morceau = routage::portion(ligne, debut, fin);
            if morceau.len() < 2 {
                continue;
            }
            let pts = ecran(&morceau);
            match symbole(v) {
                '1' => {
                    courant = detail;
                    if detail {
                        t.p.add(Shape::line(
                            pts.clone(),
                            Stroke::new(f.lueur_fil * k, c.un.alpha(f.opacite_lueur)),
                        ));
                    }
                    t.p.add(Shape::line(pts, Stroke::new(largeur, c.un.c())));
                    if !a.mouvement.sobre && detail {
                        // Perles de courant : elles glissent le long du fil, de la source vers l'arrivée.
                        let glisse = ((maintenant * f64::from(a.mouvement.vitesse_perles))
                            % f64::from(f.pas_perles)) as f32;
                        let mut s =
                            (debut - glisse).div_euclid(f.pas_perles) * f.pas_perles + glisse;
                        while s < fin {
                            if s > debut + 1.0 {
                                if let Some(q) = routage::point_a(ligne, s) {
                                    t.p.circle_filled(
                                        cam.ecran(q),
                                        (f.rayon_perle * k).max(0.9),
                                        c.perle.alpha(0.95),
                                    );
                                }
                            }
                            s += f.pas_perles;
                        }
                    }
                }
                '0' => {}
                _ if v == Valeur::Z => {
                    // Haute impédance : pointillés serrés, gris, immobiles. Le X reste en
                    // tirets longs qui défilent : on les distingue sans la couleur.
                    t.p.extend(Shape::dashed_line(
                        &pts,
                        Stroke::new(largeur, c.z.c()),
                        (2.0 * k).max(1.5),
                        (5.0 * k).max(2.5),
                    ));
                }
                _ if !detail => {
                    t.p.add(Shape::line(pts, Stroke::new(largeur, c.x.alpha(0.6))));
                }
                _ => {
                    let glisse = if a.mouvement.sobre {
                        0.0
                    } else {
                        ((maintenant * 31.0) % 22.0) as f32
                    };
                    t.p.extend(Shape::dashed_line_with_offset(
                        &pts,
                        Stroke::new(largeur, c.x.c()),
                        &[10.0 * k],
                        &[12.0 * k],
                        (22.0 - glisse) * k,
                    ));
                }
            }
        }
        if p < 1.0 && symbole(onde.vers) == '1' {
            if let Some(q) = routage::point_a(ligne, front) {
                t.p.circle_filled(cam.ecran(q), largeur * 1.5, c.un.alpha(0.3));
                t.p.circle_filled(cam.ecran(q), largeur * 0.5, c.perle.c());
            }
        }
        courant
    }

    #[allow(clippy::too_many_arguments)]
    fn dessiner_element(
        &self,
        t: &Toile,
        a: &Apparence,
        at: &Atelier,
        anims: &mut Animations,
        e: &Element,
        attend: bool,
        depuis: Option<Port>,
        aimant: Option<Port>,
        detail: bool,
        dt: f32,
    ) {
        let (c, m, cam) = (t.c, &a.mouvement, &self.cam);
        let maintenant = at.horloge;
        let anime = !m.sobre;
        let pos = self.position(at, e.id);
        let centre = cam.ecran(pos);
        let choisi = at.est_selectionne(e.id);
        let tenu = matches!(self.geste, Geste::Element { id, bouge: true, .. } if id == e.id || (choisi && at.est_selectionne(id)));
        let leve = tenu;
        let survole = self.survol == Survol::Element(e.id)
            && matches!(self.geste, Geste::Aucun | Geste::Element { .. });

        // Échelle : ressort quand on soulève, apparition quand on pose.
        let cle = |nom: &str| egui::Id::unique((nom, e.id));
        let soulevement = anims.ressort(
            cle("leve"),
            if leve { 1.0 } else { 0.0 },
            dt,
            m.raideur,
            m.amortissement,
        );
        let pop = self.naissances.get(&e.id).map_or(1.0, |t0| {
            m.avancement(maintenant - *t0, m.apparition, m.courbe_apparition)
        });
        let echelle = (1.0 + (m.echelle_leve - 1.0) * soulevement) * (0.4 + 0.6 * pop);
        let s = cam.k * echelle;
        let chaud = anims.vers(
            cle("chaud"),
            if survole || leve { 1.0 } else { 0.0 },
            dt,
            m.lissage_ui,
        );
        // Seul un élément en train d'apparaître demande un pinceau voilé.
        let voile;
        let t = if pop < 1.0 {
            voile = t.sur(pop.clamp(0.02, 1.0), t.p.clip_rect());
            &voile
        } else {
            t
        };
        // L'élément se dessine dans son repère, puis un élément retourné passe dans un miroir :
        // les abscisses changent de signe, les textes restent à l'endroit et changent de côté.
        let sx = e.sens();
        let q = |x: f32, y: f32| Pos2::new(centre.x + sx * x * s, centre.y + y * s);
        let r = |x: f32, y: f32, l: f32, h: f32| Rect::from_two_pos(q(x, y), q(x + l, y + h));
        let cote = |a: Align2| {
            if e.miroir {
                Align2([
                    match a.x() {
                        egui::Align::Min => egui::Align::Max,
                        egui::Align::Max => egui::Align::Min,
                        centre => centre,
                    },
                    a.y(),
                ])
            } else {
                a
            }
        };
        // Le contour de la sélection s'allume et s'éteint en douceur.
        let selection = if choisi {
            anims.vers_depuis(cle("choisi"), 0.0, 1.0, dt, m.lissage_ui)
        } else {
            anims.vers_si_connu(cle("choisi"), 0.0, dt, m.lissage_ui)
        };
        let contour = c.bord.vers(c.accent_clair, selection.max(chaud));
        let ombre = |rect: Rect, rayon: f32| {
            if !detail {
                return;
            }
            let ombre = Shadow {
                offset: [0, ((5.0 + 9.0 * soulevement) * s).clamp(0.0, 120.0) as i8],
                blur: ((12.0 + 8.0 * soulevement) * s).clamp(0.0, 250.0) as u8,
                spread: 0,
                color: c.ombre_plan.c(),
            };
            t.p.add(ombre.as_shape(rect, rayon));
        };
        // Le rectangle du corps, relatif au centre.
        let (cg, ch, cd, cb) = e.corps_local();
        let corps = r(cg, ch, cd - cg, cb - ch);
        let trait_corps = Stroke::new((2.0 + selection) * s, contour.c());
        let couleur_de = |v: Valeur| match symbole(v) {
            '1' => c.un.c(),
            '0' => c.doux.c(),
            _ => c.x.c(),
        };

        match e.genre {
            Genre::Nand => {
                // Anneau d'attente : la sortie n'a pas encore rejoint sa cible.
                let doute = anims.vers(
                    cle("attend"),
                    if attend { 1.0 } else { 0.0 },
                    dt,
                    m.lissage_ui,
                );
                if doute > 0.02 && detail {
                    let anneau = fermer(contour_arrondi(r(-53.0, -40.0, 96.0, 80.0), 24.0 * s));
                    let glisse = if anime {
                        ((maintenant * 13.0) % 13.0) as f32
                    } else {
                        0.0
                    };
                    t.p.extend(Shape::dashed_line_with_offset(
                        &anneau,
                        Stroke::new(2.5 * s, c.attente.alpha(doute)),
                        &[7.0 * s],
                        &[6.0 * s],
                        (13.0 - glisse) * s,
                    ));
                }
                ombre(corps, a.formes.rayon_porte * s);
                // Éclat du basculement : le contour s'allume puis retombe.
                let depuis_tir = self.eclats.get(&e.id).map(|t0| (maintenant - *t0) as f32);
                let flash = depuis_tir.map_or(0.0, |dt| {
                    (1.0 - dt / m.eclat_contour.max(0.01)).clamp(0.0, 1.0)
                });
                let trait_ = Stroke::new(
                    (2.0 + selection + 2.0 * flash) * s,
                    contour.vers(c.un, flash).c(),
                );
                t.p.rect(
                    corps,
                    a.formes.rayon_porte * s,
                    c.carte.c(),
                    trait_,
                    StrokeKind::Middle,
                );
                if detail {
                    t.p.rect_filled(
                        r(-43.0, -30.0, 76.0, 22.0),
                        coins((a.formes.rayon_porte - 2.0) * s, 0.0),
                        c.carte_2.c(),
                    );
                    // Le glyphe : un D et un rond, la porte NAND.
                    let glyphe = Stroke::new(2.4 * s, c.texte.c());
                    let mut d = vec![q(-21.0, -13.0), q(-6.0, -13.0)];
                    d.extend((1..12).map(|i| {
                        let angle = (-90.0 + 180.0 * i as f32 / 12.0).to_radians();
                        q(-6.0 + 13.0 * angle.cos(), 13.0 * angle.sin())
                    }));
                    d.extend([q(-6.0, 13.0), q(-21.0, 13.0)]);
                    t.p.add(Shape::closed_line(d, glyphe));
                    t.p.circle_stroke(q(12.0, 0.0), 3.4 * s, glyphe);
                }
                if let Some(dt) = depuis_tir.filter(|dt| *dt < m.eclat && anime && detail) {
                    let p = dt / m.eclat.max(0.01);
                    t.p.circle_stroke(
                        q(40.0, 0.0),
                        9.0 * (0.4 + 2.4 * p) * s,
                        Stroke::new(3.0 * s, c.un.alpha(0.9 * (1.0 - p))),
                    );
                }
                if s * 12.0 >= 6.0 {
                    t.texte(
                        q(-5.0, 54.0),
                        Align2::CENTER_CENTER,
                        &e.nom,
                        Police::MonoGras,
                        12.0 * s,
                        c.doux.c(),
                    );
                }
            }
            Genre::Sortie if e.est_afficheur() => {
                let lecture = at.lecture_lue(Fil {
                    puits: e.id,
                    entree: 0,
                });
                let rayon = 12.0 * s;
                ombre(corps, rayon);
                if choisi || chaud > 0.02 {
                    t.p.rect_stroke(
                        corps.expand(5.0 * s),
                        rayon + 5.0 * s,
                        Stroke::new(
                            2.0 * s,
                            c.accent_clair.alpha(if choisi { 1.0 } else { chaud * 0.6 }),
                        ),
                        StrokeKind::Middle,
                    );
                }
                t.p.rect(corps, rayon, c.carte.c(), trait_corps, StrokeKind::Middle);
                // Les sept segments, a en haut puis b à f dans le sens des aiguilles d'une
                // montre, g au milieu ; le point (8e bit) en bas à droite.
                const SEGMENTS: [((f32, f32), (f32, f32)); 7] = [
                    ((-12.0, -38.0), (12.0, -38.0)),
                    ((17.0, -33.0), (17.0, -11.0)),
                    ((17.0, -1.0), (17.0, 21.0)),
                    ((-12.0, 26.0), (12.0, 26.0)),
                    ((-17.0, -1.0), (-17.0, 21.0)),
                    ((-17.0, -33.0), (-17.0, -11.0)),
                    ((-12.0, -6.0), (12.0, -6.0)),
                ];
                for (k, ((x0, y0), (x1, y1))) in SEGMENTS.into_iter().enumerate() {
                    let allume = anims.vers(
                        egui::Id::unique(("segment", e.id, k)),
                        if lecture.uns >> k & 1 == 1 { 1.0 } else { 0.0 },
                        dt,
                        m.allumage,
                    );
                    let flou = lecture.flous >> k & 1 == 1;
                    let (a0, a1) = (q(x0, y0), q(x1, y1));
                    if allume > 0.01 && detail {
                        t.p.line_segment(
                            [a0, a1],
                            Stroke::new(11.0 * s, c.un.alpha(0.22 * allume)),
                        );
                    }
                    let couleur = if flou {
                        c.x.alpha(0.55)
                    } else {
                        c.bord.vers(c.un, allume).c()
                    };
                    t.p.line_segment([a0, a1], Stroke::new(6.0 * s, couleur));
                }
                if lecture.largeur == 8 {
                    let allume = lecture.uns >> 7 & 1 == 1;
                    t.p.circle_filled(
                        q(25.0, 27.0),
                        3.6 * s,
                        if allume { c.un.c() } else { c.bord.c() },
                    );
                }
                if s * 14.0 >= 6.0 {
                    let legende = if e.legende.is_empty() {
                        &e.nom
                    } else {
                        &e.legende
                    };
                    t.texte(
                        q(0.0, 68.0),
                        Align2::CENTER_CENTER,
                        legende,
                        Police::CorpsDemi,
                        14.0 * s,
                        c.doux.c(),
                    );
                }
            }
            Genre::Entree | Genre::Sortie | Genre::Horloge => {
                let lecture = if e.genre == Genre::Sortie {
                    at.lecture_lue(Fil {
                        puits: e.id,
                        entree: 0,
                    })
                } else {
                    at.lecture(Source::de(e.id))
                };
                let symb = symbole(lecture.valeur());
                let texte = lecture.texte();
                let allume = anims.vers(
                    cle("allume"),
                    if symb == '1' { 1.0 } else { 0.0 },
                    dt,
                    m.allumage,
                );
                let rond = e.genre == Genre::Sortie;
                let forme = |marge: f32| {
                    if rond {
                        Rect::from_center_size(centre, Vec2::splat((52.0 + 2.0 * marge) * s))
                    } else {
                        r(
                            -31.0 - marge,
                            -26.0 - marge,
                            62.0 + 2.0 * marge,
                            52.0 + 2.0 * marge,
                        )
                    }
                };
                let rayon = |marge: f32| {
                    if rond {
                        (26.0 + marge) * s
                    } else {
                        (17.0 + marge * 0.9) * s
                    }
                };
                if at.invite && e.genre == Genre::Entree && anime {
                    // Invitation à cliquer : un anneau qui s'élargit.
                    let periode = f64::from(m.pulsation);
                    let p = ((maintenant % periode) / periode) as f32;
                    t.p.rect_stroke(
                        forme(15.0 * p),
                        rayon(15.0 * p),
                        Stroke::new(2.0 * s, c.accent_clair.alpha(0.9 * (1.0 - p))),
                        StrokeKind::Middle,
                    );
                }
                if rond && symb == '1' && anime && detail && lecture.largeur == 1 {
                    // La lampe allumée respire.
                    let p = m.pouls(maintenant, 1.0) * allume;
                    t.p.circle_stroke(
                        centre,
                        26.0 * (1.12 + 0.22 * p) * s,
                        Stroke::new(2.0 * s, c.un.alpha(0.55 - 0.45 * p)),
                    );
                }
                if allume > 0.01 {
                    t.p.rect_filled(forme(8.0), rayon(8.0), c.un.alpha(0.16 * allume));
                }
                if choisi || chaud > 0.02 {
                    t.p.rect_stroke(
                        forme(5.0),
                        rayon(5.0),
                        Stroke::new(
                            2.0 * s,
                            c.accent_clair.alpha(if choisi { 1.0 } else { chaud * 0.6 }),
                        ),
                        StrokeKind::Middle,
                    );
                }
                let fond = c.carte.vers(c.un, allume);
                if symb == 'X' {
                    t.p.rect_filled(forme(0.0), rayon(0.0), fond.c());
                    let bord = fermer(contour_arrondi(forme(0.0), rayon(0.0)));
                    t.p.extend(Shape::dashed_line(
                        &bord,
                        Stroke::new(3.0 * s, c.x.c()),
                        6.0 * s,
                        5.0 * s,
                    ));
                } else {
                    t.p.rect(
                        forme(0.0),
                        rayon(0.0),
                        fond.c(),
                        Stroke::new(3.0 * s, c.zero.vers(c.un, allume).c()),
                        StrokeKind::Middle,
                    );
                }
                // Le chiffre saute quand il change.
                let cle_chiffre = cle("chiffre");
                let memoire = egui::Id::unique(("valeur", e.id));
                let avant = t.p.ctx().data_mut(|d| {
                    let avant = d.get_temp::<char>(memoire);
                    d.insert_temp(memoire, symb);
                    avant.unwrap_or(symb)
                });
                if avant != symb {
                    anims.placer(cle_chiffre, 0.6);
                }
                let saut = anims.ressort(cle_chiffre, 1.0, dt, m.raideur, m.amortissement);
                let couleur = match symb {
                    '1' => c.sur_un.c(),
                    '0' => c.doux.c(),
                    _ => c.x.c(),
                };
                let taille = match texte.chars().count() {
                    0 | 1 => 23.0,
                    2 => 20.0,
                    3 | 4 => 14.0,
                    _ => 9.0,
                };
                let horloge = e.genre == Genre::Horloge;
                if s * taille >= 6.0 {
                    t.texte(
                        if horloge { q(0.0, -7.0) } else { centre },
                        Align2::CENTER_CENTER,
                        &texte,
                        Police::Titre,
                        taille * if horloge { 0.8 } else { 1.0 } * s * saut,
                        couleur,
                    );
                }
                if horloge && detail {
                    // Un créneau : c'est une horloge.
                    let y = |haut: bool| if haut { 9.0 } else { 17.0 };
                    let pts = vec![
                        q(-16.0, y(false)),
                        q(-8.0, y(false)),
                        q(-8.0, y(true)),
                        q(0.0, y(true)),
                        q(0.0, y(false)),
                        q(8.0, y(false)),
                        q(8.0, y(true)),
                        q(16.0, y(true)),
                    ];
                    t.p.add(Shape::line(pts, Stroke::new(2.0 * s, couleur)));
                }
                if e.est_poussoir() && detail {
                    // Bouton poussoir : un capuchon rond, enfoncé tant qu'on appuie.
                    t.p.circle_stroke(
                        centre,
                        (if symb == '1' { 17.0 } else { 19.5 }) * s,
                        Stroke::new(2.0 * s, couleur),
                    );
                }
                if e.genre == Genre::Entree && e.param > 0 && detail {
                    // Bouton de démarrage : une petite flèche de mise en route.
                    t.p.add(Shape::convex_polygon(
                        vec![q(-23.0, 12.0), q(-23.0, 20.0), q(-16.0, 16.0)],
                        couleur,
                        Stroke::NONE,
                    ));
                }
                if s * 14.0 >= 6.0 {
                    if rond {
                        let legende = if e.legende.is_empty() {
                            &e.nom
                        } else {
                            &e.legende
                        };
                        t.texte(
                            q(0.0, 46.0),
                            Align2::CENTER_CENTER,
                            legende,
                            Police::CorpsDemi,
                            14.0 * s,
                            c.doux.c(),
                        );
                    } else {
                        t.texte(
                            q(0.0, -43.0),
                            Align2::CENTER_CENTER,
                            &e.nom,
                            Police::CorpsDemi,
                            14.0 * s,
                            c.doux.c(),
                        );
                    }
                }
            }
            Genre::Brique => {
                let rayon = a.formes.rayon_porte * s;
                ombre(corps, rayon);
                // Une brique qui ne peut pas être simulée (modèle disparu) se voit.
                let connu = at.projet.modele(&e.modele).is_some();
                t.p.rect(
                    corps,
                    rayon,
                    c.carte.c(),
                    if connu {
                        trait_corps
                    } else {
                        Stroke::new(2.0 * s, c.danger.c())
                    },
                    StrokeKind::Middle,
                );
                if detail {
                    let bandeau = Rect::from_min_size(
                        corps.min + Vec2::splat(2.0 * s),
                        Vec2::new(corps.width() - 4.0 * s, 24.0 * s),
                    );
                    t.p.rect_filled(bandeau, coins(rayon - 2.0 * s, 0.0), c.carte_2.c());
                    // Deux petits carrés emboîtés : on peut ouvrir.
                    let coin = Pos2::new(corps.right() - 15.0 * s, bandeau.center().y);
                    let carre = |d: f32, taille: f32| {
                        Rect::from_center_size(coin + Vec2::splat(d * s), Vec2::splat(taille * s))
                    };
                    let trait_icone = Stroke::new(1.4 * s, c.doux.c());
                    t.p.rect_stroke(carre(1.5, 7.0), 1.5 * s, trait_icone, StrokeKind::Middle);
                    t.p.rect_stroke(carre(-1.5, 7.0), 1.5 * s, trait_icone, StrokeKind::Middle);
                }
                if s * 12.0 >= 6.0 {
                    t.texte(
                        Pos2::new(corps.left() + 12.0 * s, corps.top() + 14.0 * s),
                        Align2::LEFT_CENTER,
                        &e.modele,
                        Police::MonoGras,
                        12.0 * s,
                        c.texte.c(),
                    );
                    t.texte(
                        Pos2::new(corps.center().x, corps.bottom() + 14.0 * s),
                        Align2::CENTER_CENTER,
                        &e.nom,
                        Police::MonoGras,
                        12.0 * s,
                        c.doux.c(),
                    );
                }
                if s * 11.0 >= 6.0 {
                    for (i, b) in e.broches_e.iter().enumerate() {
                        let v = at.valeur_lue(Fil {
                            puits: e.id,
                            entree: i,
                        });
                        t.texte(
                            q(cg + 11.0, e.decalage_entree(i)),
                            cote(Align2::LEFT_CENTER),
                            &b.nom,
                            Police::Mono,
                            11.0 * s,
                            couleur_de(v),
                        );
                    }
                    for (j, b) in e.broches_s.iter().enumerate() {
                        let v = at.valeur(Source::nouvelle(e.id, j));
                        t.texte(
                            q(cd - 11.0, e.decalage_sortie(j)),
                            cote(Align2::RIGHT_CENTER),
                            &b.nom,
                            Police::Mono,
                            11.0 * s,
                            couleur_de(v),
                        );
                    }
                }
            }
            Genre::Ecran => {
                let rayon = 14.0 * s;
                ombre(corps, rayon);
                t.p.rect(corps, rayon, c.carte.c(), trait_corps, StrokeKind::Middle);
                let vitre = corps.shrink2(Vec2::new(26.0 * s, 12.0 * s));
                let vitre = Rect::from_min_max(
                    Pos2::new(corps.left() + 44.0 * s, vitre.top()),
                    Pos2::new(corps.right() - 12.0 * s, vitre.bottom()),
                );
                t.p.rect(
                    vitre,
                    8.0 * s,
                    c.fond.c(),
                    Stroke::new(1.0 * s, c.bord.c()),
                    StrokeKind::Inside,
                );
                if s * 13.0 >= 6.0 {
                    // Les dernières lignes écrites, comme sur un terminal.
                    const COLONNES: usize = 24;
                    const LIGNES: usize = 7;
                    let mut lignes: Vec<String> = Vec::new();
                    for brute in at.session.ecran(e.id).split('\n') {
                        let caracteres: Vec<char> = brute.chars().collect();
                        if caracteres.is_empty() {
                            lignes.push(String::new());
                        }
                        for morceau in caracteres.chunks(COLONNES) {
                            lignes.push(morceau.iter().collect());
                        }
                    }
                    let debut = lignes.len().saturating_sub(LIGNES);
                    let curseur = !anime || (maintenant * 2.0) as i64 % 2 == 0;
                    let visibles = &lignes[debut..];
                    for (n, ligne) in visibles.iter().enumerate() {
                        let dernier = n + 1 == visibles.len();
                        let texte = if dernier && curseur {
                            format!("{ligne}▌")
                        } else {
                            ligne.clone()
                        };
                        t.texte(
                            Pos2::new(
                                vitre.left() + 10.0 * s,
                                vitre.top() + (15.0 + 19.0 * n as f32) * s,
                            ),
                            Align2::LEFT_CENTER,
                            &texte,
                            Police::MonoGras,
                            13.0 * s,
                            c.un.c(),
                        );
                    }
                }
                if s * 10.0 >= 6.0 {
                    for (i, b) in e.broches_e.iter().enumerate() {
                        t.texte(
                            q(cg + 9.0, e.decalage_entree(i)),
                            cote(Align2::LEFT_CENTER),
                            &b.nom,
                            Police::Mono,
                            10.0 * s,
                            c.doux.c(),
                        );
                    }
                    t.texte(
                        Pos2::new(corps.center().x, corps.bottom() + 14.0 * s),
                        Align2::CENTER_CENTER,
                        &e.nom,
                        Police::MonoGras,
                        12.0 * s,
                        c.doux.c(),
                    );
                }
            }
            Genre::Clavier => {
                let rayon = 14.0 * s;
                ombre(corps, rayon);
                let actif = at.clavier_actif() == Some(e.id);
                t.p.rect(
                    corps,
                    rayon,
                    c.carte.c(),
                    if actif && !choisi {
                        Stroke::new(2.0 * s, c.bord.vers(c.accent_clair, 0.5).c())
                    } else {
                        trait_corps
                    },
                    StrokeKind::Middle,
                );
                let lecture = at.lecture(Source::de(e.id));
                let enfoncee = lecture.nombre().filter(|n| *n != 0);
                if detail {
                    // Trois rangées de touches ; elles s'allument quand une touche est enfoncée.
                    for rangee in 0..3 {
                        for touche in 0..8 {
                            let x = cg + 13.0 + touche as f32 * 12.0 + rangee as f32 * 3.0;
                            let y = ch + 14.0 + rangee as f32 * 12.0;
                            t.p.rect_filled(
                                r(x, y, 9.0, 9.0),
                                2.5 * s,
                                if enfoncee.is_some() {
                                    c.un.alpha(0.55)
                                } else {
                                    c.carte_2.c()
                                },
                            );
                        }
                    }
                    t.p.rect_filled(r(cg + 30.0, ch + 50.0, 62.0, 8.0), 2.5 * s, c.carte_2.c());
                }
                if s * 15.0 >= 6.0 {
                    let touche = match enfoncee {
                        Some(n @ 33..=126) => char::from(n as u8).to_string(),
                        Some(32) => "␣".to_owned(),
                        Some(n) => format!("{n:02X}"),
                        None => "·".to_owned(),
                    };
                    t.texte(
                        q(cd - 22.0, 0.0),
                        Align2::CENTER_CENTER,
                        &touche,
                        Police::Titre,
                        17.0 * s,
                        if enfoncee.is_some() {
                            c.un.c()
                        } else {
                            c.doux.c()
                        },
                    );
                    let attente = at.session.touches_en_attente(e.id);
                    let legende = if attente > 0 {
                        tr!("{nom} · {attente} en attente", attente = attente, nom = e.nom)
                    } else if actif {
                        tr!("{nom} · tape au clavier", nom = e.nom)
                    } else {
                        e.nom.clone()
                    };
                    t.texte(
                        Pos2::new(corps.center().x, corps.bottom() + 14.0 * s),
                        Align2::CENTER_CENTER,
                        &legende,
                        Police::MonoGras,
                        12.0 * s,
                        if actif { c.texte.c() } else { c.doux.c() },
                    );
                }
            }
            Genre::Decoupe | Genre::Jonction => {
                let barre =
                    Rect::from_center_size(centre, Vec2::new(8.0 * s, (e.hauteur() - 4.0) * s));
                t.p.rect(
                    barre,
                    4.0 * s,
                    c.bord.vers(c.texte, 0.35).c(),
                    Stroke::new(
                        if choisi { 2.0 * s } else { 0.0 },
                        c.accent_clair.alpha(if choisi { 1.0 } else { chaud }),
                    ),
                    StrokeKind::Outside,
                );
                if s * 9.0 >= 6.0 {
                    // Le numéro des bits, du côté où le bus est découpé.
                    if e.genre == Genre::Decoupe {
                        for (j, b) in e.broches_s.iter().enumerate() {
                            t.texte(
                                q(13.0, e.decalage_sortie(j) - 8.0),
                                cote(Align2::LEFT_CENTER),
                                &b.nom,
                                Police::Mono,
                                9.0 * s,
                                c.doux.c(),
                            );
                        }
                    } else {
                        for (i, b) in e.broches_e.iter().enumerate() {
                            t.texte(
                                q(-13.0, e.decalage_entree(i) - 8.0),
                                cote(Align2::RIGHT_CENTER),
                                &b.nom,
                                Police::Mono,
                                9.0 * s,
                                c.doux.c(),
                            );
                        }
                    }
                }
            }
        }

        // Points de connexion.
        if !detail && depuis.is_none() {
            return;
        }
        let mut point = |port: Port, p: Point, bus: bool| {
            let centre = cam.ecran(p);
            let libre = self.geste == Geste::Aucun && self.survol == Survol::Port(port);
            let candidat = match (depuis, port) {
                (Some(Port::Sortie(source)), Port::Entree(fil))
                | (Some(Port::Entree(fil)), Port::Sortie(source)) => {
                    at.doc.verifier(source, fil).is_ok()
                }
                _ => false,
            };
            let colle = aimant == Some(port);
            let cible = if colle {
                10.0
            } else if libre {
                7.5
            } else if candidat {
                7.0
            } else if bus {
                5.8
            } else {
                5.0
            };
            let cle = match port {
                Port::Sortie(s) => egui::Id::unique(("port-s", s.id, s.sortie)),
                Port::Entree(f) => egui::Id::unique(("port-e", f.puits, f.entree)),
            };
            let vif = colle || libre || candidat;
            // Au repos, un point de connexion n'a pas besoin d'animation.
            let rayon = if vif {
                anims.ressort(cle, cible, dt, m.raideur, m.amortissement)
            } else {
                anims.ressort_si_connu(cle, cible, dt, m.raideur, m.amortissement)
            }
            .max(2.0)
                * cam.k;
            t.p.circle(
                centre,
                rayon,
                if colle {
                    c.accent_clair.c()
                } else {
                    c.fond.c()
                },
                Stroke::new(
                    (if bus { 3.0 } else { 2.0 }) * cam.k,
                    if vif { c.accent_clair.c() } else { c.bord.c() },
                ),
            );
        };
        for (j, b) in e.broches_s.iter().enumerate() {
            point(
                Port::Sortie(Source::nouvelle(e.id, j)),
                e.point_sortie(pos, j),
                b.largeur > 1,
            );
        }
        for (i, b) in e.broches_e.iter().enumerate() {
            point(
                Port::Entree(Fil {
                    puits: e.id,
                    entree: i,
                }),
                e.entree_en(pos, i),
                b.largeur > 1,
            );
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn le_zoom_garde_le_point_vise_immobile() {
        let m = Manipulation::default();
        let mut cam = Camera {
            x: 40.0,
            y: 10.0,
            k: 1.0,
            tx: 40.0,
            ty: 10.0,
            tk: 1.0,
            vx: 0.0,
            vy: 0.0,
        };
        let ancre = Pos2::new(500.0, 300.0);
        let avant = cam.monde(ancre);
        cam.zoomer(ancre, 1.7, &m, true);
        assert!((cam.k - 1.7).abs() < 1e-6);
        let apres = cam.ecran(avant);
        assert!(
            apres.distance(ancre) < 1e-3,
            "le point sous le pointeur n'a pas bougé : {apres:?}"
        );
        // Le zoom est borné.
        cam.zoomer(ancre, 100.0, &m, true);
        assert_eq!(cam.k, m.zoom_max);
        cam.zoomer(ancre, 1e-6, &m, true);
        assert_eq!(cam.k, m.zoom_min);
    }

    #[test]
    fn le_zoom_lisse_garde_aussi_le_point_vise_pendant_l_animation() {
        let (m, mv) = (Manipulation::default(), Mouvement::default());
        let mut cam = Camera::default();
        let ancre = Pos2::new(320.0, 240.0);
        let avant = cam.monde(ancre);
        cam.zoomer(ancre, 2.0, &m, false);
        assert_eq!(cam.k, 1.0, "le zoom affiché n'a pas encore bougé");
        let mut images = 0;
        while cam.animer(1.0 / 60.0, &mv) {
            assert!(cam.ecran(avant).distance(ancre) < 0.01);
            images += 1;
            assert!(images < 600);
        }
        assert_eq!(cam.k, 2.0);
        assert!(images > 5, "le zoom est animé sur plusieurs images");
    }

    #[test]
    fn le_cadrage_centre_le_circuit_sans_depasser_le_zoom_maximal() {
        let m = Manipulation::default();
        let mut cam = Camera::default();
        let libre = Rect::from_min_max(Pos2::new(260.0, 90.0), Pos2::new(1080.0, 620.0));
        let boite = Rect::from_min_max(Pos2::new(0.0, 60.0), Pos2::new(900.0, 540.0));
        cam.cadrer(boite, libre, &m, true);
        let (hg, bd) = (
            cam.ecran(Point::nouveau(0.0, 60.0)),
            cam.ecran(Point::nouveau(900.0, 540.0)),
        );
        assert!(libre.expand(0.5).contains(hg) && libre.expand(0.5).contains(bd));
        assert!(((hg.x + bd.x) / 2.0 - libre.center().x).abs() < 0.5);
        // Un circuit minuscule n'est pas grossi au-delà du zoom de cadrage.
        cam.cadrer(
            Rect::from_min_size(Pos2::ZERO, Vec2::splat(10.0)),
            libre,
            &m,
            true,
        );
        assert_eq!(cam.k, m.zoom_cadrage);
    }

    #[test]
    fn la_glissade_ralentit_et_s_arrete() {
        let mv = Mouvement::default();
        let mut cam = Camera {
            vx: 900.0,
            ..Camera::default()
        };
        let mut images = 0;
        while cam.animer(1.0 / 60.0, &mv) {
            images += 1;
            assert!(images < 600);
        }
        assert!(
            cam.x > 100.0 && cam.x < 900.0 * mv.inertie * 1.1,
            "glissade de {} points",
            cam.x
        );
        assert_eq!(cam.x, cam.tx);
    }

    #[test]
    fn un_element_presque_aligne_se_redresse() {
        let a = Apparence::default();
        let at = Atelier::nouveau(&a);
        let id = |nom: &str| at.doc.elements.iter().find(|e| e.nom == nom).unwrap().id;
        // La lampe s est branchée sur g4 (y = 240) : à 4 unités près, elle s'aligne.
        let (p, colle) = Plan::aimanter(&a, &at, id("s"), Point::nouveau(900.0, 244.0));
        assert!(colle);
        assert_eq!(p, Point::nouveau(900.0, 240.0));
        let (p, colle) = Plan::aimanter(&a, &at, id("s"), Point::nouveau(900.0, 270.0));
        assert!(!colle);
        assert_eq!(p.y, 270.0);
        // g2 lit a (y = 135) sur son entrée du haut, décalée de -15 : alignée quand g2 est à y = 150.
        let (p, colle) = Plan::aimanter(&a, &at, id("g2"), Point::nouveau(460.0, 153.0));
        assert!(colle);
        assert_eq!(p.y, 150.0);
    }
}
