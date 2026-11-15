//! Mise en page : où se trouve chaque panneau, calculé d'un seul coup à partir de la taille de
//! la fenêtre, des dimensions de l'apparence et de l'ouverture de chaque panneau.
//!
//! Le dessin des panneaux ne décide d'aucune position : il reçoit un rectangle. Changer la
//! disposition (panneau à gauche plutôt qu'à droite, barre plus haute…) se fait ici.

use egui::{Pos2, Rect, Vec2};

use crate::apparence::Formes;

/// Ouverture de chaque panneau : 0 = caché, 1 = affiché, entre les deux pendant l'animation.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Ouvertures {
    /// Panneau des briques.
    pub briques: f32,
    /// Panneau de la quête.
    pub quete: f32,
    /// Panneau des infos.
    pub infos: f32,
    /// Chronogramme et journal.
    pub temps: f32,
}

/// Tailles qui dépendent du contenu, mesurées par le dessin et fournies à la mise en page.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Contenus {
    /// Largeur de la carte du nom de l'application.
    pub marque: f32,
    /// Largeur de cette carte réduite à son logo, quand la fenêtre est très étroite.
    pub marque_logo: f32,
    /// Largeur de la carte de lecture.
    pub transport: f32,
    /// Largeur de cette carte, la vitesse réduite à son icône.
    pub transport_reduit: f32,
    /// Largeur de la carte des boutons de panneaux, avec leurs libellés.
    pub volets: f32,
    /// Largeur de cette carte réduite aux icônes des boutons.
    pub volets_icones: f32,
    /// Hauteur du panneau des briques.
    pub briques: f32,
    /// Hauteur du panneau de la quête.
    pub quete: f32,
    /// Hauteur du panneau du temps.
    pub temps: f32,
    /// Largeur de la pilule des outils de vue.
    pub outils: f32,
}

impl Default for Contenus {
    fn default() -> Self {
        // Les tailles mesurées avec le texte à sa taille normale.
        Contenus {
            marque: 255.0,
            marque_logo: 99.0,
            transport: 537.0,
            transport_reduit: 463.0,
            volets: 570.0,
            volets_icones: 310.0,
            briques: 640.0,
            quete: 300.0,
            temps: 242.0,
            outils: 282.0,
        }
    }
}

/// Le rectangle de chaque zone de l'interface.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Disposition {
    /// Toute la fenêtre.
    pub ecran: Rect,
    /// Carte du nom de l'application.
    pub marque: Rect,
    /// Carte de lecture (pas, pause, vitesse, état).
    pub transport: Rect,
    /// Carte des boutons qui montrent ou cachent les panneaux.
    pub volets: Rect,
    /// Panneau des briques.
    pub briques: Rect,
    /// Panneau de la quête.
    pub quete: Rect,
    /// Panneau des infos.
    pub infos: Rect,
    /// Chronogramme et journal.
    pub temps: Rect,
    /// La zone du plan que rien ne recouvre.
    pub libre: Rect,
    /// Pilule des outils de vue (annuler, zoom, cadrer), en bas à droite de la zone libre.
    pub outils: Rect,
    /// Emplacement de la barre de sélection, en bas à gauche de la zone libre.
    pub selection: Pos2,
    /// Hauteur des pilules du bas de la zone libre.
    pub hauteur_pilule: f32,
    /// Vrai si la fenêtre est trop étroite pour les libellés des boutons de panneaux.
    pub compact: bool,
    /// Vrai si elle l'est même pour le nom de l'application : la marque se réduit à son logo.
    pub etroit: bool,
}

/// Calcule la disposition.
pub fn disposer(ecran: Rect, f: &Formes, o: Ouvertures, c: Contenus) -> Disposition {
    let (m, g) = (f.marge, f.gouttiere);
    let haut = ecran.top() + m;
    let sous_barre = haut + f.hauteur_barre + g;
    let bas = ecran.bottom() - m;
    let barre =
        |x: f32, l: f32| Rect::from_min_size(Pos2::new(x, haut), Vec2::new(l, f.hauteur_barre));

    // Barre du haut : le nom à gauche, les panneaux à droite, la lecture au milieu de ce qui reste.
    // Quand la rangée ne tient pas (fenêtre étroite, grand texte), les boutons de panneaux
    // perdent leurs libellés, puis le nom se réduit à son logo. Les largeurs comparées ne
    // dépendent pas de ces choix : la décision ne bascule pas d'une image à l'autre.
    let rangee = |volets: f32| 2.0 * (m + g) + c.marque + c.transport + volets;
    let compact = ecran.width() < rangee(c.volets);
    let etroit = ecran.width() < rangee(c.volets_icones);
    let (l_marque, l_transport) = if etroit {
        (c.marque_logo, c.transport_reduit)
    } else {
        (c.marque, c.transport)
    };
    let marque = barre(ecran.left() + m, l_marque);
    let l_volets = if compact { c.volets_icones } else { c.volets };
    let volets = barre(ecran.right() - m - l_volets, l_volets);
    let (libre_g, libre_d) = (marque.right() + g, volets.left() - g);
    // Dernier recours (fenêtre minimale, très grand texte, temps très long) : la carte de
    // lecture prend la place qui reste, et abrège son état, plutôt que de recouvrir ses voisines.
    let l_transport = l_transport.min((libre_d - libre_g).max(0.0));
    let centre = ((libre_g + libre_d) / 2.0)
        .max(libre_g + l_transport / 2.0)
        .min((libre_d - l_transport / 2.0).max(libre_g + l_transport / 2.0));
    let transport = barre(centre - l_transport / 2.0, l_transport);

    // Colonne de gauche.
    let h_briques = c.briques.min(bas - sous_barre);
    let briques = Rect::from_min_size(
        Pos2::new(
            ecran.left() + m - (1.0 - o.briques) * (f.largeur_gauche + m + 8.0),
            sous_barre,
        ),
        Vec2::new(f.largeur_gauche, h_briques),
    );

    // Colonne de droite : la quête au-dessus, les infos prennent le reste.
    let o_droite = o.quete.max(o.infos);
    let x_droite = ecran.right() - m - f.largeur_droite;
    let cache = f.largeur_droite + m + 8.0;
    let h_quete = c.quete.min((bas - sous_barre) * 0.62);
    let quete = Rect::from_min_size(
        Pos2::new(x_droite + (1.0 - o.quete) * cache, sous_barre),
        Vec2::new(f.largeur_droite, h_quete),
    );
    let y_infos = sous_barre + o.quete * (h_quete + g);
    let infos = Rect::from_min_max(
        Pos2::new(x_droite + (1.0 - o.infos) * cache, y_infos),
        Pos2::new(
            x_droite + (1.0 - o.infos) * cache + f.largeur_droite,
            bas.max(y_infos + 80.0),
        ),
    );

    // Bande libre entre les colonnes.
    let gauche = ecran.left() + m + o.briques * (f.largeur_gauche + g);
    let droite = (ecran.right() - m - o_droite * (f.largeur_droite + g)).max(gauche + 120.0);
    let h_temps = c.temps.min((bas - sous_barre) * 0.5);
    let temps = Rect::from_min_size(
        Pos2::new(
            gauche,
            bas - h_temps + (1.0 - o.temps) * (h_temps + m + 8.0),
        ),
        Vec2::new(droite - gauche, h_temps),
    );
    let bas_libre = (bas - o.temps * (h_temps + g)).max(sous_barre + 120.0);
    let libre = Rect::from_min_max(Pos2::new(gauche, sous_barre), Pos2::new(droite, bas_libre));

    let hauteur_pilule = f.bouton + 12.0;
    let outils = Rect::from_min_size(
        Pos2::new(libre.right() - c.outils, libre.bottom() - hauteur_pilule),
        Vec2::new(c.outils, hauteur_pilule),
    );
    let selection = Pos2::new(libre.left(), libre.bottom() - hauteur_pilule);

    Disposition {
        ecran,
        marque,
        transport,
        volets,
        briques,
        quete,
        infos,
        temps,
        libre,
        outils,
        selection,
        hauteur_pilule,
        compact,
        etroit,
    }
}

/// La mise en page de l'écran d'accueil : le menu à gauche, le contenu de la rubrique à droite.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct DispositionAccueil {
    /// Toute la fenêtre.
    pub ecran: Rect,
    /// La carte du menu.
    pub menu: Rect,
    /// La zone de la rubrique choisie.
    pub contenu: Rect,
    /// Vrai si le menu est réduit à ses icônes (fenêtre étroite).
    pub compact: bool,
    /// Vrai si le contenu a la place de mettre deux colonnes côte à côte.
    pub large: bool,
}

/// Largeur du menu de l'accueil, déplié et réduit à ses icônes.
const MENU_ACCUEIL: (f32, f32) = (252.0, 64.0);
/// Largeur maximale du contenu de l'accueil : au-delà, il reste centré.
const CONTENU_ACCUEIL_MAX: f32 = 1180.0;

/// Calcule la mise en page de l'accueil.
pub fn disposer_accueil(ecran: Rect, f: &Formes) -> DispositionAccueil {
    let (m, g) = (f.marge, f.gouttiere);
    let compact = ecran.width() < 1180.0;
    let l_menu = if compact {
        MENU_ACCUEIL.1
    } else {
        MENU_ACCUEIL.0
    };
    let menu = Rect::from_min_max(
        Pos2::new(ecran.left() + m, ecran.top() + m),
        Pos2::new(ecran.left() + m + l_menu, ecran.bottom() - m),
    );
    let gauche = menu.right() + g + 2.0 * m;
    let droite = ecran.right() - 2.0 * m;
    let l = (droite - gauche).clamp(200.0, CONTENU_ACCUEIL_MAX);
    let x = gauche + ((droite - gauche) - l).max(0.0) / 2.0;
    let contenu = Rect::from_min_max(
        Pos2::new(x, ecran.top() + m + 12.0),
        Pos2::new(x + l, ecran.bottom() - m),
    );
    DispositionAccueil {
        ecran,
        menu,
        contenu,
        compact,
        large: contenu.width() >= 820.0,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn l_accueil_tient_dans_toutes_les_fenetres() {
        for (l, h) in [(1040.0, 660.0), (1440.0, 900.0), (2560.0, 1440.0)] {
            let d = disposer_accueil(ecran(l, h), &Formes::default());
            assert!(d.ecran.contains_rect(d.menu) && d.ecran.contains_rect(d.contenu));
            assert!(!d.menu.intersects(d.contenu));
            assert!(d.contenu.width() >= 600.0, "{l}×{h} : contenu trop étroit");
            assert_eq!(d.compact, l < 1180.0);
        }
    }

    const OUVERT: Ouvertures = Ouvertures {
        briques: 1.0,
        quete: 1.0,
        infos: 1.0,
        temps: 1.0,
    };
    const FERME: Ouvertures = Ouvertures {
        briques: 0.0,
        quete: 0.0,
        infos: 0.0,
        temps: 0.0,
    };

    fn ecran(l: f32, h: f32) -> Rect {
        Rect::from_min_size(Pos2::ZERO, Vec2::new(l, h))
    }

    /// Les tailles mesurées, le texte agrandi de ce facteur (seules les parties écrites grandissent).
    fn contenus(texte: f32) -> Contenus {
        let c = Contenus::default();
        Contenus {
            marque: 116.0 + (c.marque - 116.0) * texte,
            transport: c.transport * texte,
            transport_reduit: c.transport_reduit * texte,
            volets: c.volets_icones + (c.volets - c.volets_icones) * texte,
            ..c
        }
    }

    #[test]
    fn la_barre_du_haut_ne_se_chevauche_jamais() {
        let f = Formes::default();
        for texte in [1.0, 1.15, 1.3] {
            let c = contenus(texte);
            for l in [
                1040.0, 1100.0, 1180.0, 1280.0, 1366.0, 1400.0, 1440.0, 1920.0,
            ] {
                let d = disposer(ecran(l, 800.0), &f, OUVERT, c);
                let barre = [d.marque, d.transport, d.volets];
                for (i, a) in barre.iter().enumerate() {
                    assert!(
                        d.ecran.contains_rect(*a),
                        "{l} px, texte ×{texte} : la carte {i} sort de la fenêtre"
                    );
                    for b in &barre[i + 1..] {
                        assert!(
                            a.right() + f.gouttiere <= b.left() + 0.01,
                            "{l} px, texte ×{texte} : la barre du haut se chevauche ({d:?})"
                        );
                    }
                }
                // Les libellés ne disparaissent que faute de place.
                assert_eq!(
                    d.compact,
                    l < 2.0 * (f.marge + f.gouttiere) + c.marque + c.transport + c.volets
                );
            }
        }
        // À la taille habituelle tout a ses libellés ; plus étroit, les boutons d'abord.
        let d = disposer(ecran(1440.0, 900.0), &f, OUVERT, contenus(1.0));
        assert!(!d.compact && !d.etroit);
        let d = disposer(ecran(1366.0, 768.0), &f, OUVERT, contenus(1.0));
        assert!(d.compact && !d.etroit);
        let d = disposer(ecran(1040.0, 660.0), &f, OUVERT, contenus(1.0));
        assert!(d.compact && d.etroit);
    }

    #[test]
    fn tout_ouvert_rien_ne_se_chevauche_et_tout_tient_dans_la_fenetre() {
        for (l, h) in [
            (1440.0, 900.0),
            (1366.0, 768.0),
            (1280.0, 720.0),
            (1920.0, 1080.0),
        ] {
            let d = disposer(ecran(l, h), &Formes::default(), OUVERT, Contenus::default());
            let zones = [
                d.marque,
                d.transport,
                d.volets,
                d.briques,
                d.quete,
                d.infos,
                d.temps,
                d.libre,
            ];
            for (i, a) in zones.iter().enumerate() {
                assert!(
                    d.ecran.contains_rect(*a),
                    "{l}×{h} : la zone {i} sort de la fenêtre : {a:?}"
                );
                for (j, b) in zones.iter().enumerate().skip(i + 1) {
                    assert!(
                        !a.shrink(0.5).intersects(b.shrink(0.5)),
                        "{l}×{h} : les zones {i} et {j} se chevauchent"
                    );
                }
            }
            assert!(d.libre.contains_rect(d.outils));
            assert!(d.libre.width() > 300.0 && d.libre.height() > 200.0);
        }
    }

    #[test]
    fn tout_ferme_le_plan_prend_toute_la_place_sous_la_barre() {
        let f = Formes::default();
        let d = disposer(ecran(1440.0, 900.0), &f, FERME, Contenus::default());
        assert_eq!(d.libre.left(), f.marge);
        assert_eq!(d.libre.right(), 1440.0 - f.marge);
        assert_eq!(d.libre.bottom(), 900.0 - f.marge);
        for cache in [d.briques, d.quete, d.infos, d.temps] {
            assert!(
                !d.ecran.intersects(cache),
                "un panneau fermé est hors de la fenêtre : {cache:?}"
            );
        }
    }

    #[test]
    fn fermer_la_quete_fait_monter_les_infos() {
        let f = Formes::default();
        let ouvert = disposer(ecran(1440.0, 900.0), &f, OUVERT, Contenus::default());
        let sans_quete = disposer(
            ecran(1440.0, 900.0),
            &f,
            Ouvertures {
                quete: 0.0,
                ..OUVERT
            },
            Contenus::default(),
        );
        assert!(sans_quete.infos.top() < ouvert.infos.top());
        assert_eq!(sans_quete.infos.top(), ouvert.quete.top());
        assert_eq!(
            sans_quete.libre, ouvert.libre,
            "les infos gardent la colonne de droite occupée"
        );
    }
}
