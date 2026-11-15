//! Tout ce qui règle l'esthétique et le toucher de l'application, en un seul endroit.
//!
//! Rien d'autre dans la caisse ne contient de couleur, de rayon, de durée ou de seuil de
//! manipulation en dur : le dessin et les gestes lisent une [`Apparence`]. Elle se sérialise
//! en JSON ; un fichier `apparence.json` posé à côté de l'exécutable remplace les valeurs par
//! défaut (les champs absents gardent leur valeur) et il est relu à chaud pendant que
//! l'application tourne.

use std::path::{Path, PathBuf};
use std::time::SystemTime;

use egui::Color32;
use serde::{Deserialize, Serialize};

/// Une couleur, écrite `#rrggbb` ou `#rrggbbaa` dans le fichier d'apparence.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Couleur(pub [u8; 4]);

impl Couleur {
    /// Couleur opaque à partir de `0xRRGGBB`.
    pub const fn rvb(hex: u32) -> Self {
        Couleur([(hex >> 16) as u8, (hex >> 8) as u8, hex as u8, 255])
    }

    /// Couleur avec opacité à partir de `0xRRGGBB` et d'un alpha de 0 à 255.
    pub const fn rvba(hex: u32, alpha: u8) -> Self {
        Couleur([(hex >> 16) as u8, (hex >> 8) as u8, hex as u8, alpha])
    }

    /// La couleur pour egui.
    pub fn c(self) -> Color32 {
        let [r, v, b, a] = self.0;
        Color32::from_rgba_unmultiplied(r, v, b, a)
    }

    /// La même couleur, d'opacité multipliée par `facteur` (de 0 à 1).
    pub fn alpha(self, facteur: f32) -> Color32 {
        let [r, v, b, a] = self.0;
        let a = (f32::from(a) * facteur.clamp(0.0, 1.0)).round() as u8;
        Color32::from_rgba_unmultiplied(r, v, b, a)
    }

    /// Mélange vers `autre` : 0 donne `self`, 1 donne `autre`.
    pub fn vers(self, autre: Couleur, t: f32) -> Couleur {
        let t = t.clamp(0.0, 1.0);
        let m = |a: u8, b: u8| (f32::from(a) + (f32::from(b) - f32::from(a)) * t).round() as u8;
        Couleur([
            m(self.0[0], autre.0[0]),
            m(self.0[1], autre.0[1]),
            m(self.0[2], autre.0[2]),
            m(self.0[3], autre.0[3]),
        ])
    }
}

impl Serialize for Couleur {
    fn serialize<S: serde::Serializer>(&self, s: S) -> Result<S::Ok, S::Error> {
        let [r, v, b, a] = self.0;
        if a == 255 {
            s.serialize_str(&format!("#{r:02x}{v:02x}{b:02x}"))
        } else {
            s.serialize_str(&format!("#{r:02x}{v:02x}{b:02x}{a:02x}"))
        }
    }
}

impl<'de> Deserialize<'de> for Couleur {
    fn deserialize<D: serde::Deserializer<'de>>(d: D) -> Result<Self, D::Error> {
        let texte = String::deserialize(d)?;
        let hex = texte.trim().trim_start_matches('#');
        let octet = |i: usize| u8::from_str_radix(hex.get(i..i + 2).unwrap_or(""), 16);
        let erreur = || {
            serde::de::Error::custom(tr!(
                "couleur invalide : « {texte} » (attendu #rrggbb ou #rrggbbaa)",
                texte = texte
            ))
        };
        match hex.len() {
            6 => Ok(Couleur([
                octet(0).map_err(|_| erreur())?,
                octet(2).map_err(|_| erreur())?,
                octet(4).map_err(|_| erreur())?,
                255,
            ])),
            8 => Ok(Couleur([
                octet(0).map_err(|_| erreur())?,
                octet(2).map_err(|_| erreur())?,
                octet(4).map_err(|_| erreur())?,
                octet(6).map_err(|_| erreur())?,
            ])),
            _ => Err(erreur()),
        }
    }
}

/// Les couleurs d'un mode (sombre ou clair).
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct Palette {
    /// Fond du plan.
    pub fond: Couleur,
    /// Lueur au centre du fond.
    pub fond_halo: Couleur,
    /// Points de la grille.
    pub grille: Couleur,
    /// Fond des panneaux et des portes.
    pub carte: Couleur,
    /// Fond d'un élément au repos dans un panneau (pas 3 de l'échelle neutre).
    pub carte_2: Couleur,
    /// Fond d'un élément survolé (pas 4).
    pub survol: Couleur,
    /// Fond d'un élément enfoncé, ouvert ou choisi (pas 5).
    pub enfonce: Couleur,
    /// Séparateurs fins à l'intérieur d'un panneau (pas 6).
    #[serde(rename = "trait")]
    pub trait_: Couleur,
    /// Bordures des panneaux et des éléments (pas 7).
    pub bord: Couleur,
    /// Bordures appuyées : élément survolé, champ (pas 8).
    pub bord_fort: Couleur,
    /// Reflet d'arête : trait clair sur le bord haut des surfaces surélevées.
    pub reflet: Couleur,
    /// Texte principal.
    pub texte: Couleur,
    /// Texte secondaire.
    pub doux: Couleur,
    /// Texte discret : légendes, raccourcis, rubriques (toujours lisible, AA).
    pub discret: Couleur,
    /// Signal à 1.
    pub un: Couleur,
    /// Texte posé sur la couleur du 1.
    pub sur_un: Couleur,
    /// Perles de courant.
    pub perle: Couleur,
    /// Signal à 0.
    pub zero: Couleur,
    /// Signal indéterminé (X).
    pub x: Couleur,
    /// Haute impédance (Z) et tracé du 0 dans le chronogramme.
    pub z: Couleur,
    /// Porte en attente de basculement.
    pub attente: Couleur,
    /// Couleur d'action.
    pub accent: Couleur,
    /// Couleur d'action, version lisible sur le fond (sélection, survol).
    pub accent_clair: Couleur,
    /// Texte posé sur la couleur d'action.
    pub sur_accent: Couleur,
    /// Actions destructrices.
    pub danger: Couleur,
    /// Ombre des panneaux.
    pub ombre: Couleur,
    /// Ombre des éléments du plan.
    pub ombre_plan: Couleur,
}

impl Palette {
    /// Néon sombre : la direction choisie.
    pub fn sombre() -> Self {
        Palette {
            fond: Couleur::rvb(0x0b0f17),
            fond_halo: Couleur::rvb(0x131a2b),
            grille: Couleur::rvb(0x1e2534),
            carte: Couleur::rvb(0x121722),
            carte_2: Couleur::rvb(0x191f2c),
            survol: Couleur::rvb(0x1f2635),
            enfonce: Couleur::rvb(0x273043),
            trait_: Couleur::rvb(0x1d2431),
            bord: Couleur::rvb(0x2a3243),
            bord_fort: Couleur::rvb(0x3a4459),
            reflet: Couleur::rvba(0xffffff, 13),
            texte: Couleur::rvb(0xe8ecf4),
            doux: Couleur::rvb(0xa3acbd),
            discret: Couleur::rvb(0x8e99ac),
            un: Couleur::rvb(0x4dffb5),
            sur_un: Couleur::rvb(0x04170e),
            perle: Couleur::rvb(0xf2fff9),
            zero: Couleur::rvb(0x3b4560),
            x: Couleur::rvb(0xff9b62),
            z: Couleur::rvb(0x8c96b3),
            attente: Couleur::rvb(0xffd166),
            accent: Couleur::rvb(0x6d5df5),
            accent_clair: Couleur::rvb(0xa99cff),
            sur_accent: Couleur::rvb(0xffffff),
            danger: Couleur::rvb(0xff6b81),
            ombre: Couleur::rvba(0x000308, 150),
            ombre_plan: Couleur::rvba(0x000308, 120),
        }
    }

    /// Néon clair.
    pub fn clair() -> Self {
        Palette {
            fond: Couleur::rvb(0xeef1f6),
            fond_halo: Couleur::rvb(0xffffff),
            grille: Couleur::rvb(0xcfd6e3),
            carte: Couleur::rvb(0xffffff),
            carte_2: Couleur::rvb(0xf4f6fa),
            survol: Couleur::rvb(0xeaeef5),
            enfonce: Couleur::rvb(0xdfe5ef),
            trait_: Couleur::rvb(0xe3e7ef),
            bord: Couleur::rvb(0xd3d9e5),
            bord_fort: Couleur::rvb(0xb9c2d3),
            reflet: Couleur::rvba(0xffffff, 200),
            texte: Couleur::rvb(0x141a28),
            doux: Couleur::rvb(0x4f5a70),
            discret: Couleur::rvb(0x58637a),
            un: Couleur::rvb(0x007a52),
            sur_un: Couleur::rvb(0xffffff),
            perle: Couleur::rvb(0xffffff),
            zero: Couleur::rvb(0xb3bdd1),
            x: Couleur::rvb(0xa8430f),
            z: Couleur::rvb(0x6f7990),
            attente: Couleur::rvb(0x865600),
            accent: Couleur::rvb(0x5a43e0),
            accent_clair: Couleur::rvb(0x5a43e0),
            sur_accent: Couleur::rvb(0xffffff),
            danger: Couleur::rvb(0xc8283f),
            ombre: Couleur::rvba(0x1a2440, 40),
            ombre_plan: Couleur::rvba(0x1a2440, 36),
        }
    }
}

impl Default for Palette {
    fn default() -> Self {
        Palette::sombre()
    }
}

/// Dimensions et rayons, en points d'écran pour l'interface et en unités du plan pour le circuit.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct Formes {
    /// Marge entre la fenêtre et les panneaux.
    pub marge: f32,
    /// Espace entre deux panneaux.
    pub gouttiere: f32,
    /// Rayon des panneaux.
    pub rayon_carte: f32,
    /// Rayon des boutons et des champs.
    pub rayon_bouton: f32,
    /// Rayon des petites pièces : pastilles, étiquettes, puces.
    pub rayon_petit: f32,
    /// Rayon des grandes cartes de l'accueil.
    pub rayon_vitrine: f32,
    /// Hauteur d'un contrôle compact (ligne d'un panneau, champ).
    pub controle: f32,
    /// Rembourrage intérieur d'un panneau.
    pub rembourrage: f32,
    /// Hauteur de la barre du haut.
    pub hauteur_barre: f32,
    /// Côté des boutons carrés.
    pub bouton: f32,
    /// Largeur du panneau des briques.
    pub largeur_gauche: f32,
    /// Largeur de la colonne de droite (quête, infos).
    pub largeur_droite: f32,
    /// Hauteur d'une ligne du chronogramme.
    pub ligne_chrono: f32,
    /// Nombre d'instants visibles dans le chronogramme.
    pub instants_chrono: usize,
    /// Épaisseur des fils, en unités du plan.
    pub fil: f32,
    /// Épaisseur de la lueur autour d'un fil à 1.
    pub lueur_fil: f32,
    /// Opacité de cette lueur.
    pub opacite_lueur: f32,
    /// Rayon des coudes des fils.
    pub rayon_fil: f32,
    /// Rayon du petit pont qu'un fil fait pour enjamber un fil d'une autre source qu'il croise
    /// sans s'y relier. 0 : pas de pont.
    pub rayon_pont: f32,
    /// Rayon des pastilles de jonction (là où un fil se sépare en deux), en épaisseurs de fil.
    pub pastille: f32,
    /// Espace entre deux perles de courant.
    pub pas_perles: f32,
    /// Rayon d'une perle.
    pub rayon_perle: f32,
    /// Pas de la grille de points du fond.
    pub pas_grille: f32,
    /// Rayon du corps d'une porte.
    pub rayon_porte: f32,
    /// Facteur de taille de tout le texte de l'interface.
    pub echelle_texte: f32,
    /// Rayon des étoiles de la construction minimale, dans la fiche de la quête.
    pub etoile: f32,
    /// Rayon des petites étoiles, dans la liste du parcours.
    pub etoile_petite: f32,
}

impl Default for Formes {
    fn default() -> Self {
        Formes {
            marge: 8.0,
            gouttiere: 8.0,
            rayon_carte: 10.0,
            rayon_bouton: 6.0,
            rayon_petit: 4.0,
            rayon_vitrine: 12.0,
            controle: 28.0,
            rembourrage: 14.0,
            hauteur_barre: 48.0,
            bouton: 32.0,
            largeur_gauche: 248.0,
            largeur_droite: 304.0,
            ligne_chrono: 24.0,
            instants_chrono: 16,
            fil: 6.0,
            lueur_fil: 17.0,
            opacite_lueur: 0.14,
            rayon_fil: 14.0,
            rayon_pont: 9.0,
            pastille: 1.25,
            pas_perles: 24.0,
            rayon_perle: 1.5,
            pas_grille: 28.0,
            rayon_porte: 18.0,
            echelle_texte: 1.0,
            etoile: 6.5,
            etoile_petite: 4.5,
        }
    }
}

/// L'échelle typographique, en points (avant le facteur `echelle_texte`). Sept tailles et pas
/// une de plus : la hiérarchie vient de la taille et du poids, pas de la couleur.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct Typo {
    /// Petites capitales de rubrique, raccourcis, étiquettes de fils.
    pub legende: f32,
    /// Textes secondaires, sous-titres, info-bulles.
    pub petit: f32,
    /// Texte courant, boutons, champs, listes.
    pub corps: f32,
    /// Titres de carte, entrées du menu.
    pub fort: f32,
    /// Titre de section, nom d'une quête dans sa fiche.
    pub titre: f32,
    /// Titre d'une carte vitrine.
    pub grand: f32,
    /// Titre d'un écran de l'accueil.
    pub affiche: f32,
    /// Espacement ajouté entre les lettres des petites capitales, en points.
    pub espacement_capitales: f32,
}

impl Default for Typo {
    fn default() -> Self {
        Typo {
            legende: 11.0,
            petit: 12.0,
            corps: 13.0,
            fort: 14.0,
            titre: 17.0,
            grand: 22.0,
            affiche: 30.0,
            espacement_capitales: 0.6,
        }
    }
}

/// Une courbe d'animation : comment une transition avance entre son départ et son arrivée.
///
/// Dans le fichier d'apparence : `"lineaire"`, `"sortie"`, `"entree"`, `"douce"` ou `"rebond"`.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "lowercase")]
pub enum Courbe {
    /// Vitesse constante.
    Lineaire,
    /// Part vite et arrive en douceur : ce qui entre à l'écran.
    #[default]
    Sortie,
    /// Part en douceur et arrive vite : ce qui quitte l'écran.
    Entree,
    /// Douce au départ comme à l'arrivée : ce qui se déplace d'un endroit à un autre.
    Douce,
    /// Dépasse un peu sa cible puis y revient : ce qu'on pose.
    Rebond,
}

impl Courbe {
    /// L'avancement affiché pour une part `t` du temps écoulé (de 0 à 1). Vaut 0 en 0 et 1 en 1.
    pub fn en(self, t: f32) -> f32 {
        let t = if t.is_finite() {
            t.clamp(0.0, 1.0)
        } else {
            1.0
        };
        match self {
            Courbe::Lineaire => t,
            Courbe::Sortie => 1.0 - (1.0 - t).powi(3),
            Courbe::Entree => t.powi(3),
            Courbe::Douce => {
                if t < 0.5 {
                    4.0 * t.powi(3)
                } else {
                    1.0 - (2.0 - 2.0 * t).powi(3) / 2.0
                }
            }
            Courbe::Rebond => {
                const RECUL: f32 = 1.70158;
                1.0 + (RECUL + 1.0) * (t - 1.0).powi(3) + RECUL * (t - 1.0).powi(2)
            }
        }
    }
}

/// Durées, courbes et vitesses des animations. Les durées sont en secondes.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct Mouvement {
    /// Constante de temps du lissage de la caméra (plus petit = plus nerveux).
    pub lissage_camera: f32,
    /// Constante de temps du retour d'un élément vers sa case.
    pub lissage_element: f32,
    /// Constante de temps des survols et des petits changements d'état.
    pub lissage_ui: f32,
    /// Durée de la glissade après un déplacement du plan.
    pub inertie: f32,
    /// Durée d'ouverture et de fermeture d'un panneau.
    pub panneau: f32,
    /// Courbe de cette ouverture.
    pub courbe_panneau: Courbe,
    /// Durée du fondu entre l'accueil et l'atelier.
    pub ecran: f32,
    /// Courbe de ce fondu.
    pub courbe_ecran: Courbe,
    /// Pendant ce fondu, de combien l'écran qui s'en va grossit ou rapetisse (0,05 = 5 %).
    pub zoom_ecran: f32,
    /// Durée de l'arrivée d'une rubrique de l'accueil.
    pub rubrique: f32,
    /// Distance que parcourt, en points, ce qui arrive en glissant (rubriques, messages).
    pub glissement: f32,
    /// Durée de la plongée dans une brique, et de la remontée.
    pub plongee: f32,
    /// Courbe de cette plongée.
    pub courbe_plongee: Courbe,
    /// Grossissement du plan qu'on quitte en plongeant dans une brique (2 = deux fois plus gros).
    pub zoom_plongee: f32,
    /// Durée du fondu quand on change de thème.
    pub theme: f32,
    /// Part du pas de simulation que met le courant à parcourir un fil (de 0 à 1).
    pub part_front: f32,
    /// Vitesse des perles de courant, en unités du plan par seconde.
    pub vitesse_perles: f32,
    /// Durée de l'éclat d'une porte qui bascule.
    pub eclat: f32,
    /// Agrandissement d'un élément soulevé.
    pub echelle_leve: f32,
    /// Raideur du ressort des éléments soulevés et des boutons (plus grand = plus vif).
    pub raideur: f32,
    /// Amortissement de ce ressort (1 = sans rebond, moins = rebondit).
    pub amortissement: f32,
    /// Durée d'apparition d'un élément posé.
    pub apparition: f32,
    /// Courbe de cette apparition.
    pub courbe_apparition: Courbe,
    /// Durée de la disparition d'un élément ou d'un fil supprimé.
    pub disparition: f32,
    /// Temps que met un fil qu'on vient de brancher à se tracer de sa source à son arrivée.
    pub trace_fil: f32,
    /// Durée de la retombée du contour d'une porte qui vient de basculer.
    pub eclat_contour: f32,
    /// Constante de temps de l'allumage d'un interrupteur ou d'une lampe.
    pub allumage: f32,
    /// Période des pulsations : invitation à cliquer, lampe allumée, bouton conseillé.
    pub pulsation: f32,
    /// Durée de la fête d'une quête réussie (étincelles, ondes).
    pub fete: f32,
    /// Nombre d'étincelles lancées par chaque lampe à la fête.
    pub etincelles: usize,
    /// Durée de la grande fête d'un chapitre ou d'une branche terminés (confettis, bandeau).
    pub grande_fete: f32,
    /// Nombre de confettis de cette grande fête.
    pub confettis: usize,
    /// Durée de l'arrivée du badge de la brique gagnée, dans le panneau de la quête.
    pub badge: f32,
    /// Durée du surlignage d'un cas de la quête qui vient d'être validé.
    pub cas_valide: f32,
    /// Durée de l'éclair d'un bouton qu'on vient de cliquer.
    pub eclair_clic: f32,
    /// Durée d'arrivée d'un message.
    pub message_entree: f32,
    /// Durée de départ d'un message.
    pub message_sortie: f32,
    /// Constante de temps des barres d'avancement.
    pub lissage_barre: f32,
    /// Temps que mettent les barres d'avancement à se remplir quand une page arrive.
    pub remplissage: f32,
    /// Constante de temps du défilement du chronogramme.
    pub lissage_chrono: f32,
    /// Durées d'un pas de simulation proposées par le bouton de vitesse, en secondes.
    pub vitesses: Vec<f32>,
    /// Vitesse choisie au lancement (index dans `vitesses`).
    pub vitesse_initiale: usize,
    /// Vitesses rapides proposées après celles-ci, en pas de simulation par seconde : le
    /// courant ne s'y regarde plus porte par porte, le circuit tourne. 0 veut dire « autant
    /// que la machine le permet ».
    pub vitesses_rapides: Vec<f32>,
    /// Temps de calcul accordé à la simulation rapide dans chaque image, en millisecondes.
    /// Plus petit : l'affichage reste fluide. Plus grand : le circuit tourne plus vite.
    pub budget_rapide: f32,
    /// Animations réduites : ni perles, ni pulsations, ni transitions (fondus d'écran, plongée
    /// dans une brique, glissements, fête). Les survols et les lissages restent.
    pub sobre: bool,
}

impl Default for Mouvement {
    fn default() -> Self {
        Mouvement {
            lissage_camera: 0.08,
            lissage_element: 0.055,
            lissage_ui: 0.07,
            inertie: 0.26,
            panneau: 0.3,
            courbe_panneau: Courbe::Sortie,
            ecran: 0.36,
            courbe_ecran: Courbe::Douce,
            zoom_ecran: 0.05,
            rubrique: 0.3,
            glissement: 14.0,
            plongee: 0.5,
            courbe_plongee: Courbe::Douce,
            zoom_plongee: 2.4,
            theme: 0.4,
            part_front: 0.85,
            vitesse_perles: 32.0,
            eclat: 0.55,
            echelle_leve: 1.07,
            raideur: 520.0,
            amortissement: 0.62,
            apparition: 0.34,
            courbe_apparition: Courbe::Rebond,
            disparition: 0.24,
            trace_fil: 0.3,
            eclat_contour: 0.45,
            allumage: 0.09,
            pulsation: 1.6,
            fete: 1.9,
            etincelles: 18,
            grande_fete: 3.4,
            confettis: 120,
            badge: 0.8,
            cas_valide: 0.9,
            eclair_clic: 0.35,
            message_entree: 0.26,
            message_sortie: 0.3,
            remplissage: 0.7,
            lissage_barre: 0.18,
            lissage_chrono: 0.12,
            vitesses: vec![1.0, 0.6, 0.25],
            vitesse_initiale: 1,
            vitesses_rapides: vec![128.0, 2048.0, 32768.0, 0.0],
            budget_rapide: 9.0,
            sobre: false,
        }
    }
}

impl Mouvement {
    /// La durée d'une transition : nulle quand les animations sont réduites.
    pub fn duree(&self, secondes: f32) -> f32 {
        if self.sobre {
            0.0
        } else {
            secondes
        }
    }

    /// La pulsation, de 0 à 1 : une sinusoïde de période `pulsation × lenteur`. Au repos (0,5)
    /// quand les animations sont réduites.
    pub fn pouls(&self, horloge: f64, lenteur: f32) -> f32 {
        if self.sobre {
            return 0.5;
        }
        let periode = f64::from((self.pulsation * lenteur).max(0.05));
        0.5 - 0.5 * (horloge * std::f64::consts::TAU / periode).cos() as f32
    }

    /// L'avancement, de 0 à 1 et suivant `courbe`, d'une transition de `duree` secondes
    /// commencée il y a `age` secondes. Vaut 1 tout de suite quand les animations sont réduites.
    pub fn avancement(&self, age: f64, duree: f32, courbe: Courbe) -> f32 {
        let duree = self.duree(duree);
        if duree <= 0.0 || age >= f64::from(duree) {
            1.0
        } else {
            courbe.en((age.max(0.0) as f32) / duree)
        }
    }
}

/// Seuils de manipulation, en unités du plan sauf mention contraire.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct Manipulation {
    /// Pas d'alignement d'un élément relâché.
    pub grille: f32,
    /// Distance à laquelle un fil presque droit se redresse tout seul.
    pub aimant: f32,
    /// Distance d'accroche d'un fil sur un point de connexion.
    pub accroche: f32,
    /// Zone sensible autour d'un point de connexion.
    pub rayon_port: f32,
    /// Demi-largeur de la zone sensible d'un fil.
    pub touche_fil: f32,
    /// Zoom minimal.
    pub zoom_min: f32,
    /// Zoom maximal.
    pub zoom_max: f32,
    /// Zoom maximal choisi par le cadrage automatique.
    pub zoom_cadrage: f32,
    /// Sensibilité de la molette.
    pub molette: f32,
    /// Facteur d'un clic sur les boutons de zoom.
    pub cran_zoom: f32,
    /// Déplacement d'un élément par une flèche du clavier.
    pub pas_clavier: f32,
    /// Nombre d'actions conservées pour Annuler.
    pub profondeur_annuler: usize,
}

impl Default for Manipulation {
    fn default() -> Self {
        Manipulation {
            grille: 5.0,
            aimant: 7.0,
            accroche: 34.0,
            rayon_port: 13.0,
            touche_fil: 11.0,
            zoom_min: 0.3,
            zoom_max: 2.5,
            zoom_cadrage: 1.5,
            molette: 0.0025,
            cran_zoom: 1.25,
            pas_clavier: 10.0,
            profondeur_annuler: 80,
        }
    }
}

/// Mode de couleurs.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "lowercase")]
pub enum Mode {
    /// Fond sombre.
    #[default]
    Sombre,
    /// Fond clair.
    Clair,
}

/// L'apparence complète : les deux palettes, les formes, le mouvement, la manipulation.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct Apparence {
    /// Mode de couleurs au lancement.
    pub mode: Mode,
    /// Couleurs du mode sombre.
    #[serde(deserialize_with = "palette_sombre")]
    pub sombre: Palette,
    /// Couleurs du mode clair. Une couleur absente du fichier garde sa valeur claire.
    #[serde(deserialize_with = "palette_claire")]
    pub clair: Palette,
    /// Dimensions et rayons.
    pub formes: Formes,
    /// Tailles du texte.
    pub typo: Typo,
    /// Animations.
    pub mouvement: Mouvement,
    /// Seuils de manipulation.
    pub manipulation: Manipulation,
}

/// Une palette lue par-dessus `base` : les couleurs absentes du fichier gardent celles de `base`.
fn palette_sur<'de, D: serde::Deserializer<'de>>(d: D, base: Palette) -> Result<Palette, D::Error> {
    use serde::de::Error;
    let ecrit = serde_json::Value::deserialize(d)?;
    let mut tout = serde_json::to_value(base).map_err(D::Error::custom)?;
    if let (Some(tout), Some(ecrit)) = (tout.as_object_mut(), ecrit.as_object()) {
        for (cle, valeur) in ecrit {
            tout.insert(cle.clone(), valeur.clone());
        }
    }
    serde_json::from_value(tout).map_err(D::Error::custom)
}

fn palette_sombre<'de, D: serde::Deserializer<'de>>(d: D) -> Result<Palette, D::Error> {
    palette_sur(d, Palette::sombre())
}

fn palette_claire<'de, D: serde::Deserializer<'de>>(d: D) -> Result<Palette, D::Error> {
    palette_sur(d, Palette::clair())
}

impl Default for Apparence {
    fn default() -> Self {
        Apparence {
            mode: Mode::Sombre,
            sombre: Palette::sombre(),
            clair: Palette::clair(),
            formes: Formes::default(),
            typo: Typo::default(),
            mouvement: Mouvement::default(),
            manipulation: Manipulation::default(),
        }
    }
}

impl Apparence {
    /// La palette du mode courant.
    pub fn palette(&self) -> &Palette {
        match self.mode {
            Mode::Sombre => &self.sombre,
            Mode::Clair => &self.clair,
        }
    }

    /// Lit une apparence au format JSON. Les champs absents gardent leur valeur par défaut.
    pub fn de_json(texte: &str) -> Result<Self, String> {
        let mut a: Apparence = serde_json::from_str(texte).map_err(|e| e.to_string())?;
        a.assainir();
        Ok(a)
    }

    /// L'apparence au format JSON, prête à être modifiée à la main.
    pub fn en_json(&self) -> String {
        serde_json::to_string_pretty(self).expect("une apparence se sérialise toujours")
    }

    /// Ramène les valeurs dans des bornes où l'application reste utilisable,
    /// quoi que contienne le fichier.
    fn assainir(&mut self) {
        let m = &mut self.mouvement;
        m.lissage_camera = m.lissage_camera.clamp(0.0, 2.0);
        m.lissage_element = m.lissage_element.clamp(0.0, 2.0);
        m.lissage_ui = m.lissage_ui.clamp(0.0, 2.0);
        m.part_front = m.part_front.clamp(0.05, 1.0);
        m.raideur = m.raideur.clamp(20.0, 5000.0);
        m.amortissement = m.amortissement.clamp(0.05, 2.0);
        m.vitesses.retain(|v| v.is_finite() && *v >= 0.02);
        if m.vitesses.is_empty() {
            m.vitesses = Mouvement::default().vitesses;
        }
        m.vitesse_initiale = m.vitesse_initiale.min(m.vitesses.len() - 1);
        m.vitesses_rapides.retain(|v| v.is_finite() && *v >= 0.0);
        m.budget_rapide = m.budget_rapide.clamp(0.5, 100.0);
        // Une durée négative, infinie ou absurde ne doit ni figer ni faire clignoter l'écran.
        for duree in [
            &mut m.panneau,
            &mut m.ecran,
            &mut m.rubrique,
            &mut m.plongee,
            &mut m.theme,
            &mut m.apparition,
            &mut m.disparition,
            &mut m.trace_fil,
            &mut m.eclat,
            &mut m.eclat_contour,
            &mut m.allumage,
            &mut m.fete,
            &mut m.grande_fete,
            &mut m.badge,
            &mut m.cas_valide,
            &mut m.eclair_clic,
            &mut m.message_entree,
            &mut m.message_sortie,
            &mut m.lissage_barre,
            &mut m.lissage_chrono,
            &mut m.remplissage,
        ] {
            *duree = if duree.is_finite() {
                duree.clamp(0.0, 5.0)
            } else {
                0.0
            };
        }
        m.pulsation = if m.pulsation.is_finite() {
            m.pulsation.clamp(0.2, 20.0)
        } else {
            Mouvement::default().pulsation
        };
        m.zoom_ecran = if m.zoom_ecran.is_finite() {
            m.zoom_ecran.clamp(0.0, 0.5)
        } else {
            0.0
        };
        m.zoom_plongee = if m.zoom_plongee.is_finite() {
            m.zoom_plongee.clamp(1.0, 8.0)
        } else {
            1.0
        };
        m.glissement = if m.glissement.is_finite() {
            m.glissement.clamp(0.0, 200.0)
        } else {
            0.0
        };
        m.etincelles = m.etincelles.min(200);
        m.confettis = m.confettis.min(600);
        let g = &mut self.manipulation;
        g.grille = g.grille.max(1.0);
        g.zoom_min = g.zoom_min.clamp(0.05, 1.0);
        g.zoom_max = g.zoom_max.clamp(1.0, 8.0);
        g.zoom_cadrage = g.zoom_cadrage.clamp(g.zoom_min, g.zoom_max);
        g.cran_zoom = g.cran_zoom.clamp(1.01, 4.0);
        let f = &mut self.formes;
        f.instants_chrono = f.instants_chrono.clamp(4, 64);
        f.echelle_texte = f.echelle_texte.clamp(0.6, 2.0);
        let t = &mut self.typo;
        for taille in [
            &mut t.legende,
            &mut t.petit,
            &mut t.corps,
            &mut t.fort,
            &mut t.titre,
            &mut t.grand,
            &mut t.affiche,
        ] {
            *taille = if taille.is_finite() {
                taille.clamp(6.0, 80.0)
            } else {
                13.0
            };
        }
        t.espacement_capitales = if t.espacement_capitales.is_finite() {
            t.espacement_capitales.clamp(0.0, 4.0)
        } else {
            0.0
        };
        f.fil = f.fil.clamp(1.0, 20.0);
        f.pas_perles = f.pas_perles.max(4.0);
        f.pas_grille = f.pas_grille.max(6.0);
    }
}

/// Nom du fichier d'apparence cherché à côté de l'exécutable puis dans le dossier courant.
pub const FICHIER_APPARENCE: &str = "apparence.json";

/// Surveille le fichier d'apparence et le relit quand il change.
#[derive(Debug, Default)]
pub struct Surveillance {
    chemin: Option<PathBuf>,
    vu: Option<SystemTime>,
    prochain_controle: f64,
    /// Dernière erreur de lecture, affichée dans l'interface.
    pub erreur: Option<String>,
}

impl Surveillance {
    /// Cherche le fichier d'apparence aux emplacements habituels.
    pub fn nouvelle() -> Self {
        let mut candidats = Vec::new();
        if let Ok(exe) = std::env::current_exe() {
            if let Some(dossier) = exe.parent() {
                candidats.push(dossier.join(FICHIER_APPARENCE));
            }
        }
        candidats.push(PathBuf::from(FICHIER_APPARENCE));
        Surveillance {
            chemin: candidats.into_iter().find(|c| c.is_file()),
            ..Default::default()
        }
    }

    /// Surveille un fichier précis (qui peut ne pas exister encore).
    pub fn sur(chemin: &Path) -> Self {
        Surveillance {
            chemin: Some(chemin.to_path_buf()),
            ..Default::default()
        }
    }

    /// Le fichier surveillé, s'il y en a un.
    pub fn chemin(&self) -> Option<&Path> {
        self.chemin.as_deref()
    }

    /// Relit le fichier s'il a changé depuis la dernière lecture. `maintenant` est en secondes.
    /// Renvoie la nouvelle apparence, ou `None` si rien n'a changé ou si le fichier est invalide
    /// (l'erreur est alors dans [`Self::erreur`] et l'apparence en cours est conservée).
    pub fn relire(&mut self, maintenant: f64) -> Option<Apparence> {
        if maintenant < self.prochain_controle {
            return None;
        }
        self.prochain_controle = maintenant + 0.5;
        let chemin = self.chemin.as_ref()?;
        let modifie = std::fs::metadata(chemin).and_then(|m| m.modified()).ok()?;
        if self.vu == Some(modifie) {
            return None;
        }
        self.vu = Some(modifie);
        let lu = std::fs::read_to_string(chemin)
            .map_err(|e| e.to_string())
            .and_then(|t| Apparence::de_json(&t));
        match lu {
            Ok(a) => {
                self.erreur = None;
                Some(a)
            }
            Err(e) => {
                self.erreur = Some(format!("{} : {e}", chemin.display()));
                None
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Rapport de contraste WCAG 2.1 entre deux couleurs opaques.
    fn contraste(a: Couleur, b: Couleur) -> f64 {
        fn lum(c: Couleur) -> f64 {
            let canal = |v: u8| {
                let v = f64::from(v) / 255.0;
                if v <= 0.040_45 {
                    v / 12.92
                } else {
                    ((v + 0.055) / 1.055).powf(2.4)
                }
            };
            0.2126 * canal(c.0[0]) + 0.7152 * canal(c.0[1]) + 0.0722 * canal(c.0[2])
        }
        let (x, y) = (lum(a), lum(b));
        (x.max(y) + 0.05) / (x.min(y) + 0.05)
    }

    #[test]
    fn les_textes_passent_le_contraste_aa_dans_les_deux_themes() {
        // Règle de la direction artistique : tout texte de lecture atteint 4,5:1 (AA) sur
        // chaque surface où il peut se poser, en sombre comme en clair.
        for (theme, p) in [("sombre", Palette::sombre()), ("clair", Palette::clair())] {
            let surfaces = [
                ("fond", p.fond),
                ("carte", p.carte),
                ("carte_2", p.carte_2),
                ("survol", p.survol),
                ("enfonce", p.enfonce),
            ];
            for (nom_t, texte) in [("texte", p.texte), ("doux", p.doux), ("discret", p.discret)] {
                for (nom_s, surface) in surfaces {
                    let r = contraste(texte, surface);
                    assert!(
                        r >= 4.5,
                        "{theme} : {nom_t} sur {nom_s} = {r:.2}:1, il faut 4,5"
                    );
                }
            }
        }
    }

    #[test]
    fn l_apparence_fait_l_aller_retour_en_json() {
        let a = Apparence::default();
        assert_eq!(Apparence::de_json(&a.en_json()).unwrap(), a);
    }

    #[test]
    fn un_fichier_partiel_garde_les_valeurs_par_defaut() {
        let a = Apparence::de_json(r##"{ "mode": "clair", "sombre": { "un": "#ff00ff" }, "mouvement": { "inertie": 0.5 } }"##).unwrap();
        assert_eq!(a.mode, Mode::Clair);
        assert_eq!(a.sombre.un, Couleur::rvb(0xff00ff));
        assert_eq!(a.sombre.fond, Palette::sombre().fond);
        assert_eq!(a.clair, Palette::clair());
        assert_eq!(a.mouvement.inertie, 0.5);
        assert_eq!(a.formes, Formes::default());
    }

    #[test]
    fn les_couleurs_acceptent_l_opacite_et_refusent_le_reste() {
        let c: Couleur = serde_json::from_str(r##""#11223380""##).unwrap();
        assert_eq!(c, Couleur([0x11, 0x22, 0x33, 0x80]));
        assert_eq!(serde_json::to_string(&c).unwrap(), r##""#11223380""##);
        assert!(serde_json::from_str::<Couleur>(r#""rouge""#).is_err());
        assert!(serde_json::from_str::<Couleur>(r##""#12345""##).is_err());
    }

    #[test]
    fn les_valeurs_absurdes_sont_ramenees_dans_les_bornes() {
        let a = Apparence::de_json(r#"{ "mouvement": { "vitesses": [0, -1], "vitesse_initiale": 9 }, "manipulation": { "zoom_min": 0, "zoom_max": 100 } }"#).unwrap();
        assert_eq!(a.mouvement.vitesses, Mouvement::default().vitesses);
        assert!(a.mouvement.vitesse_initiale < a.mouvement.vitesses.len());
        assert!(a.manipulation.zoom_min > 0.0 && a.manipulation.zoom_max <= 8.0);
    }

    #[test]
    fn les_courbes_partent_de_zero_et_arrivent_a_un() {
        for courbe in [
            Courbe::Lineaire,
            Courbe::Sortie,
            Courbe::Entree,
            Courbe::Douce,
            Courbe::Rebond,
        ] {
            assert_eq!(courbe.en(0.0), 0.0, "{courbe:?}");
            assert!((courbe.en(1.0) - 1.0).abs() < 1e-6, "{courbe:?}");
            // Hors des bornes, ou pas un nombre : la courbe reste entre son départ et son arrivée.
            assert_eq!(courbe.en(-3.0), 0.0);
            assert!((courbe.en(7.0) - 1.0).abs() < 1e-6);
            assert!((courbe.en(f32::NAN) - 1.0).abs() < 1e-6);
            let mut avant = 0.0;
            for i in 0..=100 {
                let v = courbe.en(i as f32 / 100.0);
                assert!(
                    v.is_finite() && (-0.001..1.2).contains(&v),
                    "{courbe:?} {v}"
                );
                if courbe != Courbe::Rebond {
                    assert!(v >= avant - 1e-6, "{courbe:?} recule");
                }
                avant = v;
            }
        }
        // Le rebond dépasse sa cible, les autres jamais.
        assert!((0..=100).any(|i| Courbe::Rebond.en(i as f32 / 100.0) > 1.01));
        assert!((0..=100).all(|i| Courbe::Douce.en(i as f32 / 100.0) <= 1.0));
    }

    #[test]
    fn les_animations_reduites_rendent_les_transitions_immediates() {
        let mut m = Mouvement::default();
        assert!(m.avancement(0.0, m.ecran, m.courbe_ecran) < 0.01);
        assert!(m.avancement(f64::from(m.ecran) / 2.0, m.ecran, Courbe::Lineaire) > 0.49);
        assert_eq!(m.avancement(10.0, m.ecran, m.courbe_ecran), 1.0);
        m.sobre = true;
        assert_eq!(m.duree(m.plongee), 0.0);
        assert_eq!(m.avancement(0.0, m.ecran, m.courbe_ecran), 1.0);
    }

    #[test]
    fn les_durees_absurdes_sont_ramenees_dans_les_bornes() {
        let a = Apparence::de_json(
            r#"{ "mouvement": { "ecran": -4, "plongee": 1e9, "zoom_plongee": 0, "pulsation": 0,
                 "etincelles": 100000, "courbe_ecran": "rebond" } }"#,
        )
        .unwrap();
        let m = &a.mouvement;
        assert_eq!(m.ecran, 0.0);
        assert!(m.plongee <= 5.0);
        assert!(m.zoom_plongee >= 1.0 && m.pulsation > 0.0 && m.etincelles <= 200);
        assert_eq!(m.courbe_ecran, Courbe::Rebond);
        assert!(Apparence::de_json(r#"{ "mouvement": { "courbe_ecran": "zigzag" } }"#).is_err());
    }

    #[test]
    fn le_fichier_est_relu_quand_il_change() {
        let dossier = std::env::temp_dir().join(format!("ce-apparence-{}", std::process::id()));
        std::fs::create_dir_all(&dossier).unwrap();
        let chemin = dossier.join(FICHIER_APPARENCE);
        std::fs::write(&chemin, r#"{ "mode": "clair" }"#).unwrap();
        let mut s = Surveillance::sur(&chemin);
        assert_eq!(s.relire(0.0).unwrap().mode, Mode::Clair);
        assert!(s.relire(10.0).is_none(), "rien n'a changé");
        std::fs::write(&chemin, "{ pas du json").unwrap();
        // Certains systèmes de fichiers datent à la seconde : on force une date différente.
        let f = std::fs::File::options().write(true).open(&chemin).unwrap();
        f.set_modified(SystemTime::now() + std::time::Duration::from_secs(5))
            .unwrap();
        assert!(s.relire(20.0).is_none());
        assert!(
            s.erreur.is_some(),
            "l'erreur est signalée, l'apparence en cours est gardée"
        );
        std::fs::remove_dir_all(&dossier).ok();
    }
}
