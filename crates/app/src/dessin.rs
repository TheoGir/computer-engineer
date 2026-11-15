//! Outils de dessin partagés par le plan et les panneaux : polices, icônes, petites formes,
//! et le magasin d'animations. Aucune couleur ni dimension n'est décidée ici : tout vient de
//! l'[`Apparence`].

use std::collections::HashMap;

use egui::epaint::{Mesh, Shadow, Vertex, WHITE_UV};
use egui::{
    Align2, Color32, CornerRadius, FontFamily, FontId, Painter, Pos2, Rect, Shape, Stroke,
    StrokeKind, Vec2,
};

use crate::apparence::{Apparence, Couleur, Courbe, Palette, Typo};

/// Les graisses et familles de texte disponibles.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Police {
    /// Texte courant.
    Corps,
    /// Texte courant un peu appuyé.
    CorpsMoyen,
    /// Texte courant demi-gras (boutons, libellés).
    CorpsDemi,
    /// Titres.
    Titre,
    /// Chiffres et noms de signaux.
    Mono,
    /// Chiffres et noms de signaux, en gras.
    MonoGras,
}

impl Police {
    const TOUTES: [(Police, &'static str, &'static str, f32); 6] = [
        (Police::Corps, "corps", "inter", 400.0),
        (Police::CorpsMoyen, "corps-moyen", "inter", 500.0),
        (Police::CorpsDemi, "corps-demi", "inter", 600.0),
        (Police::Titre, "titre", "space-grotesk", 700.0),
        (Police::Mono, "mono", "jetbrains-mono", 500.0),
        (Police::MonoGras, "mono-gras", "jetbrains-mono", 700.0),
    ];

    fn nom(self) -> &'static str {
        Police::TOUTES
            .iter()
            .find(|(p, ..)| *p == self)
            .map_or("corps", |(_, n, ..)| n)
    }

    /// La police à la taille demandée, en points.
    pub fn taille(self, taille: f32) -> FontId {
        // Taille arrondie au quart de point : un zoom continu ne remplit pas l'atlas de glyphes.
        FontId::new(
            (taille * 4.0).round() / 4.0,
            FontFamily::Name(self.nom().into()),
        )
    }
}

/// Installe les polices embarquées (Inter, Space Grotesk, JetBrains Mono, licence OFL).
pub fn installer_polices(ctx: &egui::Context) {
    use egui::epaint::text::{FontData, FontDefinitions, FontTweak, VariationCoords};
    let fichiers: [(&str, &'static [u8]); 3] = [
        ("inter", include_bytes!("../polices/Inter.ttf")),
        (
            "space-grotesk",
            include_bytes!("../polices/SpaceGrotesk.ttf"),
        ),
        (
            "jetbrains-mono",
            include_bytes!("../polices/JetBrainsMono.ttf"),
        ),
    ];
    let mut definitions = FontDefinitions::default();
    definitions.font_data.clear();
    definitions.families.clear();
    let wght = u32::from_be_bytes(*b"wght");
    for (_, famille, fichier, graisse) in Police::TOUTES {
        let octets = fichiers
            .iter()
            .find(|(n, _)| *n == fichier)
            .expect("police embarquée")
            .1;
        let tweak = FontTweak {
            coords: VariationCoords::new([(wght, graisse)]),
            ..Default::default()
        };
        definitions.font_data.insert(
            famille.to_owned(),
            std::sync::Arc::new(FontData::from_static(octets).tweak(tweak)),
        );
        definitions
            .families
            .insert(FontFamily::Name(famille.into()), vec![famille.to_owned()]);
    }
    definitions
        .families
        .insert(FontFamily::Proportional, vec!["corps".to_owned()]);
    definitions
        .families
        .insert(FontFamily::Monospace, vec!["mono".to_owned()]);
    ctx.set_fonts(definitions);
}

/// De quoi dessiner : le pinceau d'egui et l'apparence courante.
pub struct Toile<'a> {
    /// Le pinceau.
    pub p: Painter,
    /// L'apparence complète.
    pub a: &'a Apparence,
    /// La palette du mode courant.
    pub c: &'a Palette,
}

impl<'a> Toile<'a> {
    /// Une toile sur le pinceau donné.
    pub fn nouvelle(p: Painter, a: &'a Apparence) -> Self {
        Toile {
            p,
            a,
            c: a.palette(),
        }
    }

    /// La même toile, d'opacité multipliée et limitée à un rectangle.
    pub fn sur(&self, opacite: f32, decoupe: Rect) -> Toile<'a> {
        let mut p = self.p.with_clip_rect(decoupe.intersect(self.p.clip_rect()));
        p.multiply_opacity(opacite);
        Toile {
            p,
            a: self.a,
            c: self.c,
        }
    }

    /// Écrit un texte et renvoie le rectangle occupé.
    pub fn texte(
        &self,
        pos: Pos2,
        ancre: Align2,
        texte: &str,
        police: Police,
        taille: f32,
        couleur: Color32,
    ) -> Rect {
        self.p.text(
            pos,
            ancre,
            texte,
            police.taille(taille * self.a.formes.echelle_texte),
            couleur,
        )
    }

    /// Largeur d'un texte sur une ligne.
    pub fn largeur(&self, texte: &str, police: Police, taille: f32) -> f32 {
        self.p
            .layout_no_wrap(
                texte.to_owned(),
                police.taille(taille * self.a.formes.echelle_texte),
                Color32::WHITE,
            )
            .size()
            .x
    }

    /// Le texte, raccourci et terminé par « … » s'il dépasse `largeur`.
    pub fn abreger(&self, texte: &str, police: Police, taille: f32, largeur: f32) -> String {
        if self.largeur(texte, police, taille) <= largeur {
            return texte.to_owned();
        }
        let mut court: Vec<char> = texte.chars().collect();
        while court.pop().is_some() {
            let essai: String = court.iter().chain(std::iter::once(&'…')).collect();
            if self.largeur(&essai, police, taille) <= largeur {
                return essai;
            }
        }
        String::new()
    }

    /// Une légende en capitales, raccourcie par « … » si elle dépasse `largeur` (espacement
    /// des lettres compris).
    pub fn abreger_legende(&self, texte: &str, largeur: f32) -> String {
        if self.largeur_legende(texte) <= largeur {
            return texte.to_owned();
        }
        let mut court: Vec<char> = texte.chars().collect();
        while court.pop().is_some() {
            let essai: String = court.iter().chain(std::iter::once(&'…')).collect();
            if self.largeur_legende(&essai) <= largeur {
                return essai;
            }
        }
        String::new()
    }

    /// Écrit un texte qui passe à la ligne dans `largeur`, et renvoie sa hauteur.
    pub fn paragraphe(
        &self,
        pos: Pos2,
        largeur: f32,
        texte: &str,
        police: Police,
        taille: f32,
        couleur: Color32,
    ) -> f32 {
        let galley = self.p.layout(
            texte.to_owned(),
            police.taille(taille * self.a.formes.echelle_texte),
            couleur,
            largeur,
        );
        let h = galley.size().y;
        self.p.galley(pos, galley, couleur);
        h
    }

    /// L'échelle typographique.
    pub fn ty(&self) -> &Typo {
        &self.a.typo
    }

    /// Une petite capitale de rubrique : texte en capitales, lettres espacées. Renvoie le
    /// rectangle occupé.
    pub fn legende(&self, pos: Pos2, ancre: Align2, texte: &str, couleur: Color32) -> Rect {
        let mut job = egui::text::LayoutJob::simple_singleline(
            texte.to_uppercase(),
            Police::CorpsDemi.taille(self.a.typo.legende * self.a.formes.echelle_texte),
            couleur,
        );
        for section in &mut job.sections {
            section.format.extra_letter_spacing = self.a.typo.espacement_capitales;
        }
        let galley = self.p.layout_job(job);
        let rect = ancre.anchor_size(pos, galley.size());
        self.p.galley(rect.min, galley, couleur);
        rect
    }

    /// Largeur d'une petite capitale de rubrique.
    pub fn largeur_legende(&self, texte: &str) -> f32 {
        let n = texte.chars().count() as f32;
        self.largeur(
            &texte.to_uppercase(),
            Police::CorpsDemi,
            self.a.typo.legende,
        ) + n * self.a.typo.espacement_capitales
    }

    /// Un panneau flottant : ombre en deux couches (ambiance large et contact net), fond,
    /// bordure fine et reflet clair sur l'arête du haut.
    pub fn carte(&self, rect: Rect, rayon: f32) {
        let ambiance = Shadow {
            offset: [0, 12],
            blur: 32,
            spread: 0,
            color: self.c.ombre.alpha(0.75),
        };
        let contact = Shadow {
            offset: [0, 1],
            blur: 3,
            spread: 0,
            color: self.c.ombre.alpha(0.9),
        };
        self.p.add(ambiance.as_shape(rect, rayon));
        self.p.add(contact.as_shape(rect, rayon));
        self.p.rect(
            rect,
            rayon,
            self.c.carte.alpha(0.97),
            Stroke::new(1.0, self.c.bord.c()),
            StrokeKind::Inside,
        );
        self.reflet(rect, rayon);
    }

    /// Le reflet d'arête : un trait clair qui suit le haut d'une surface surélevée.
    pub fn reflet(&self, rect: Rect, rayon: f32) {
        let rayon = rayon.min(rect.height() / 2.0).min(rect.width() / 2.0);
        let r = rect.shrink(1.0);
        let rayon = (rayon - 1.0).max(0.0);
        let mut pts = Vec::with_capacity(20);
        for i in 0..=8 {
            let t = (180.0 + 90.0 * i as f32 / 8.0_f32).to_radians();
            pts.push(Pos2::new(
                r.left() + rayon + rayon * t.cos(),
                r.top() + rayon + rayon * t.sin(),
            ));
        }
        for i in 0..=8 {
            let t = (270.0 + 90.0 * i as f32 / 8.0_f32).to_radians();
            pts.push(Pos2::new(
                r.right() - rayon + rayon * t.cos(),
                r.top() + rayon + rayon * t.sin(),
            ));
        }
        self.p
            .add(Shape::line(pts, Stroke::new(1.0, self.c.reflet.c())));
    }

    /// Le fond d'un élément cliquable dans un panneau, selon son état : au repos, survolé
    /// (`chaud` de 0 à 1) ou choisi (enfoncé, avec une bordure d'accent discrète).
    pub fn element(&self, rect: Rect, rayon: f32, chaud: f32, choisi: bool) {
        let c = self.c;
        let (fond, bord) = if choisi {
            (
                c.enfonce.vers(c.accent, 0.10),
                c.accent_clair.vers(c.bord, 0.35),
            )
        } else {
            (
                c.carte_2.vers(c.survol, chaud),
                c.bord.vers(c.bord_fort, chaud),
            )
        };
        self.p.rect(
            rect,
            rayon,
            fond.c(),
            Stroke::new(1.0, bord.c()),
            StrokeKind::Inside,
        );
    }

    /// Un trait de séparation horizontal, fin, de `gauche` à `droite`.
    pub fn separateur(&self, gauche: f32, droite: f32, y: f32) {
        let y = y.round() + 0.5;
        self.p.line_segment(
            [Pos2::new(gauche, y), Pos2::new(droite, y)],
            Stroke::new(1.0, self.c.trait_.c()),
        );
    }

    /// Une pastille de valeur : `0`, `1` ou `X`.
    pub fn pastille(&self, centre: Pos2, v: char, taille: f32) {
        let r = Rect::from_center_size(
            centre,
            Vec2::new(
                taille.max(self.largeur("0", Police::MonoGras, taille * 0.55) + taille * 0.55),
                taille,
            ),
        );
        let rayon = (taille * 0.3).min(self.a.formes.rayon_petit + 1.0);
        match v {
            '1' => {
                self.p.rect_filled(r, rayon, self.c.un.c());
                self.texte(
                    centre,
                    Align2::CENTER_CENTER,
                    "1",
                    Police::MonoGras,
                    taille * 0.55,
                    self.c.sur_un.c(),
                );
            }
            '0' => {
                self.p.rect(
                    r,
                    rayon,
                    self.c.carte_2.c(),
                    Stroke::new(1.0, self.c.bord_fort.c()),
                    StrokeKind::Inside,
                );
                self.texte(
                    centre,
                    Align2::CENTER_CENTER,
                    "0",
                    Police::MonoGras,
                    taille * 0.55,
                    self.c.doux.c(),
                );
            }
            _ => {
                let contour = contour_arrondi(r.shrink(0.75), rayon);
                self.p.extend(Shape::dashed_line(
                    &fermer(contour),
                    Stroke::new(1.5, self.c.x.c()),
                    3.0,
                    2.5,
                ));
                self.texte(
                    centre,
                    Align2::CENTER_CENTER,
                    "X",
                    Police::MonoGras,
                    taille * 0.55,
                    self.c.x.c(),
                );
            }
        }
    }

    /// Dessine une icône de 20 unités de côté, centrée, à l'échelle `taille / 20`.
    pub fn icone(&self, icone: Icone, centre: Pos2, taille: f32, couleur: Color32) {
        let k = taille / 20.0;
        // Trait de 1,75 sur la grille de 20 : 1,4 point à 16, jamais moins de 1,25.
        let trait_ = Stroke::new((1.75 * k).max(1.25), couleur);
        let vers = |x: f32, y: f32| Pos2::new(centre.x + (x - 10.0) * k, centre.y + (y - 10.0) * k);
        let ligne = |pts: &[(f32, f32)], ferme: bool| {
            let pts: Vec<Pos2> = pts.iter().map(|(x, y)| vers(*x, *y)).collect();
            if ferme {
                self.p.add(Shape::closed_line(pts, trait_));
            } else {
                self.p.add(Shape::line(pts, trait_));
            }
        };
        let cercle = |x: f32, y: f32, r: f32| {
            self.p.circle_stroke(vers(x, y), r * k, trait_);
        };
        let arc = |cx: f32, cy: f32, r: f32, de: f32, a: f32| -> Vec<(f32, f32)> {
            (0..=10)
                .map(|i| {
                    let t = (de + (a - de) * i as f32 / 10.0).to_radians();
                    (cx + r * t.cos(), cy + r * t.sin())
                })
                .collect()
        };
        match icone {
            Icone::Zero => {
                ligne(&[(5.0, 4.0), (5.0, 16.0)], false);
                ligne(&[(15.0, 4.0), (8.0, 10.0), (15.0, 16.0)], true);
            }
            Icone::Pas => {
                ligne(&[(5.0, 4.0), (12.0, 10.0), (5.0, 16.0)], true);
                ligne(&[(15.0, 4.0), (15.0, 16.0)], false);
            }
            Icone::Pause => {
                ligne(&[(7.0, 4.0), (7.0, 16.0)], false);
                ligne(&[(13.0, 4.0), (13.0, 16.0)], false);
            }
            Icone::Lecture => ligne(&[(7.0, 4.0), (16.0, 10.0), (7.0, 16.0)], true),
            Icone::Vitesse => {
                ligne(&[(3.0, 5.0), (9.0, 10.0), (3.0, 15.0)], true);
                ligne(&[(10.0, 5.0), (16.0, 10.0), (10.0, 15.0)], true);
            }
            Icone::Briques => {
                for (x, y) in [(3.0, 3.0), (11.0, 3.0), (3.0, 11.0), (11.0, 11.0)] {
                    ligne(
                        &[(x, y), (x + 6.0, y), (x + 6.0, y + 6.0), (x, y + 6.0)],
                        true,
                    );
                }
            }
            Icone::Quete => {
                ligne(&[(5.0, 18.0), (5.0, 3.0)], false);
                ligne(
                    &[
                        (5.0, 4.0),
                        (16.0, 4.0),
                        (13.0, 7.5),
                        (16.0, 11.0),
                        (5.0, 11.0),
                    ],
                    false,
                );
            }
            Icone::Infos => {
                cercle(10.0, 10.0, 7.5);
                ligne(&[(10.0, 9.0), (10.0, 14.0)], false);
                ligne(&[(10.0, 6.0), (10.0, 6.2)], false);
            }
            Icone::Temps => ligne(
                &[
                    (2.0, 13.0),
                    (6.0, 13.0),
                    (6.0, 6.0),
                    (10.0, 6.0),
                    (10.0, 13.0),
                    (14.0, 13.0),
                    (14.0, 6.0),
                    (18.0, 6.0),
                ],
                false,
            ),
            Icone::Etiquettes => {
                ligne(
                    &[
                        (3.0, 4.0),
                        (11.0, 4.0),
                        (17.0, 10.0),
                        (11.0, 16.0),
                        (3.0, 16.0),
                    ],
                    true,
                );
                cercle(7.0, 10.0, 1.0);
            }
            Icone::Fermer => {
                ligne(&[(5.0, 5.0), (15.0, 15.0)], false);
                ligne(&[(15.0, 5.0), (5.0, 15.0)], false);
            }
            Icone::Annuler => {
                ligne(&[(7.0, 5.0), (3.0, 9.0), (7.0, 13.0)], false);
                let mut pts = vec![(3.0, 9.0)];
                pts.extend(arc(12.0, 13.0, 4.0, -90.0, 90.0));
                pts.push((9.0, 17.0));
                ligne(&pts, false);
            }
            Icone::Retablir => {
                ligne(&[(13.0, 5.0), (17.0, 9.0), (13.0, 13.0)], false);
                let mut pts = vec![(17.0, 9.0)];
                pts.extend(arc(8.0, 13.0, 4.0, 270.0, 90.0));
                pts.push((11.0, 17.0));
                ligne(&pts, false);
            }
            Icone::Moins => ligne(&[(5.0, 10.0), (15.0, 10.0)], false),
            Icone::Plus => {
                ligne(&[(5.0, 10.0), (15.0, 10.0)], false);
                ligne(&[(10.0, 5.0), (10.0, 15.0)], false);
            }
            Icone::Cadrer => {
                ligne(&[(3.0, 7.0), (3.0, 3.0), (7.0, 3.0)], false);
                ligne(&[(13.0, 3.0), (17.0, 3.0), (17.0, 7.0)], false);
                ligne(&[(17.0, 13.0), (17.0, 17.0), (13.0, 17.0)], false);
                ligne(&[(7.0, 17.0), (3.0, 17.0), (3.0, 13.0)], false);
                cercle(10.0, 10.0, 2.0);
            }
            Icone::Poubelle => {
                ligne(&[(4.0, 6.0), (16.0, 6.0)], false);
                ligne(&[(8.0, 6.0), (8.0, 4.0), (12.0, 4.0), (12.0, 6.0)], false);
                ligne(&[(6.0, 6.0), (7.0, 17.0), (13.0, 17.0), (14.0, 6.0)], false);
            }
            Icone::Nand => {
                let mut pts = vec![(3.0, 4.0)];
                pts.extend(arc(9.0, 10.0, 6.0, -90.0, 90.0));
                pts.push((3.0, 16.0));
                ligne(&pts, true);
                cercle(17.0, 10.0, 1.6);
            }
            Icone::Verrou => {
                ligne(&[(4.0, 9.0), (16.0, 9.0), (16.0, 17.0), (4.0, 17.0)], true);
                let mut pts = vec![(7.0, 9.0)];
                pts.extend(arc(10.0, 6.0, 3.0, 180.0, 360.0));
                pts.push((13.0, 9.0));
                ligne(&pts, false);
            }
            Icone::Coche => ligne(&[(4.0, 10.0), (8.0, 14.0), (16.0, 6.0)], false),
            Icone::Rond => cercle(10.0, 10.0, 6.0),
            Icone::Soleil => {
                cercle(10.0, 10.0, 3.5);
                for i in 0..8 {
                    let t = (i as f32 * 45.0).to_radians();
                    ligne(
                        &[
                            (10.0 + 6.0 * t.cos(), 10.0 + 6.0 * t.sin()),
                            (10.0 + 8.0 * t.cos(), 10.0 + 8.0 * t.sin()),
                        ],
                        false,
                    );
                }
            }
            Icone::Lune => {
                let mut pts = arc(10.0, 10.0, 7.0, 60.0, 300.0);
                pts.extend(arc(14.0, 10.0, 6.0, 240.0, 120.0));
                ligne(&pts, true);
            }
            Icone::Interrupteur => {
                let mut pts = arc(6.5, 10.0, 4.5, 90.0, 270.0);
                pts.extend(arc(13.5, 10.0, 4.5, -90.0, 90.0));
                ligne(&pts, true);
                cercle(13.5, 10.0, 1.3);
            }
            Icone::Lampe => {
                cercle(10.0, 8.0, 5.0);
                ligne(&[(8.0, 15.5), (12.0, 15.5)], false);
                ligne(&[(9.0, 18.0), (11.0, 18.0)], false);
            }
            Icone::Horloge => {
                cercle(10.0, 10.0, 7.5);
                ligne(&[(10.0, 5.5), (10.0, 10.0), (13.0, 12.0)], false);
            }
            Icone::Demarrage => {
                ligne(&arc(10.0, 11.0, 6.5, -55.0, 235.0), false);
                ligne(&[(10.0, 2.5), (10.0, 10.0)], false);
            }
            Icone::Ecran => {
                ligne(&[(3.0, 4.0), (17.0, 4.0), (17.0, 14.0), (3.0, 14.0)], true);
                ligne(&[(7.0, 17.5), (13.0, 17.5)], false);
            }
            Icone::Clavier => {
                ligne(&[(2.0, 5.5), (18.0, 5.5), (18.0, 15.5), (2.0, 15.5)], true);
                for x in [5.5, 8.5, 11.5, 14.5] {
                    ligne(&[(x, 9.0), (x, 9.2)], false);
                }
                ligne(&[(6.5, 12.5), (13.5, 12.5)], false);
            }
            Icone::Decoupe => {
                ligne(&[(2.0, 10.0), (18.0, 10.0)], false);
                ligne(&[(7.0, 10.0), (12.0, 4.5), (18.0, 4.5)], false);
                ligne(&[(7.0, 10.0), (12.0, 15.5), (18.0, 15.5)], false);
            }
            Icone::Jonction => {
                ligne(&[(2.0, 10.0), (18.0, 10.0)], false);
                ligne(&[(2.0, 4.5), (8.0, 4.5), (13.0, 10.0)], false);
                ligne(&[(2.0, 15.5), (8.0, 15.5), (13.0, 10.0)], false);
            }
            Icone::Brique => {
                ligne(&[(5.0, 3.5), (15.0, 3.5), (15.0, 16.5), (5.0, 16.5)], true);
                ligne(&[(1.5, 7.5), (5.0, 7.5)], false);
                ligne(&[(1.5, 12.5), (5.0, 12.5)], false);
                ligne(&[(15.0, 10.0), (18.5, 10.0)], false);
            }
            Icone::Ouvrir => {
                ligne(&[(9.0, 4.0), (16.0, 4.0), (16.0, 16.0), (9.0, 16.0)], false);
                ligne(&[(3.0, 10.0), (12.0, 10.0)], false);
                ligne(&[(9.0, 7.0), (12.0, 10.0), (9.0, 13.0)], false);
            }
            Icone::Chevron => ligne(&[(8.0, 5.0), (13.0, 10.0), (8.0, 15.0)], false),
            Icone::Grouper => {
                ligne(&[(3.0, 3.0), (17.0, 3.0), (17.0, 17.0), (3.0, 17.0)], true);
                ligne(&[(7.0, 7.0), (13.0, 7.0), (13.0, 13.0), (7.0, 13.0)], true);
            }
            Icone::Dupliquer => {
                ligne(&[(3.0, 7.0), (13.0, 7.0), (13.0, 17.0), (3.0, 17.0)], true);
                ligne(
                    &[
                        (7.0, 7.0),
                        (7.0, 3.0),
                        (17.0, 3.0),
                        (17.0, 13.0),
                        (13.0, 13.0),
                    ],
                    false,
                );
            }
            Icone::Maison => {
                ligne(&[(3.0, 10.0), (10.0, 3.5), (17.0, 10.0)], false);
                ligne(&[(5.5, 8.0), (5.5, 16.5), (14.5, 16.5), (14.5, 8.0)], false);
                ligne(
                    &[(8.5, 16.5), (8.5, 12.0), (11.5, 12.0), (11.5, 16.5)],
                    false,
                );
            }
            Icone::Dossier => {
                ligne(
                    &[
                        (3.0, 5.0),
                        (8.0, 5.0),
                        (10.0, 7.0),
                        (17.0, 7.0),
                        (17.0, 16.0),
                        (3.0, 16.0),
                    ],
                    true,
                );
                ligne(&[(3.0, 9.5), (17.0, 9.5)], false);
            }
            Icone::Engrenage => {
                cercle(10.0, 10.0, 2.5);
                let mut pts = Vec::with_capacity(49);
                for i in 0..48 {
                    let t = (i as f32 * 7.5).to_radians();
                    // Huit dents : le rayon passe de 5,5 à 7,5 par créneaux arrondis.
                    let dent = ((i as f32 * 7.5 + 11.25).rem_euclid(45.0) < 22.5) as u8 as f32;
                    let r = 5.6 + 1.9 * dent;
                    pts.push((10.0 + r * t.cos(), 10.0 + r * t.sin()));
                }
                ligne(&pts, true);
            }
            Icone::Sortie => {
                ligne(&[(11.0, 4.0), (4.0, 4.0), (4.0, 16.0), (11.0, 16.0)], false);
                ligne(&[(8.0, 10.0), (17.0, 10.0)], false);
                ligne(&[(14.0, 7.0), (17.0, 10.0), (14.0, 13.0)], false);
            }
            Icone::Crayon => {
                ligne(
                    &[
                        (13.0, 4.0),
                        (16.0, 7.0),
                        (7.0, 16.0),
                        (4.0, 16.0),
                        (4.0, 13.0),
                    ],
                    true,
                );
                ligne(&[(11.0, 6.0), (14.0, 9.0)], false);
            }
            Icone::Fleche => {
                ligne(&[(4.0, 10.0), (16.0, 10.0)], false);
                ligne(&[(11.0, 5.0), (16.0, 10.0), (11.0, 15.0)], false);
            }
            Icone::Retour => {
                ligne(&[(16.0, 10.0), (4.0, 10.0)], false);
                ligne(&[(9.0, 5.0), (4.0, 10.0), (9.0, 15.0)], false);
            }
        }
    }
}

/// Les icônes de l'interface.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Icone {
    /// Revenir à t = 0.
    Zero,
    /// Avancer d'un pas.
    Pas,
    /// Mettre en pause.
    Pause,
    /// Lancer.
    Lecture,
    /// Changer de vitesse.
    Vitesse,
    /// Panneau des briques.
    Briques,
    /// Panneau de la quête.
    Quete,
    /// Panneau des infos.
    Infos,
    /// Panneau du chronogramme.
    Temps,
    /// Étiquettes des signaux.
    Etiquettes,
    /// Fermer un panneau.
    Fermer,
    /// Annuler.
    Annuler,
    /// Rétablir.
    Retablir,
    /// Dézoomer.
    Moins,
    /// Zoomer.
    Plus,
    /// Recentrer le circuit.
    Cadrer,
    /// Supprimer.
    Poubelle,
    /// Porte NAND.
    Nand,
    /// Brique pas encore disponible.
    Verrou,
    /// Cas validé.
    Coche,
    /// Cas à essayer.
    Rond,
    /// Passer en clair.
    Soleil,
    /// Passer en sombre.
    Lune,
    /// Un interrupteur.
    Interrupteur,
    /// Une lampe.
    Lampe,
    /// Une horloge.
    Horloge,
    /// Un bouton de démarrage.
    Demarrage,
    /// Un écran.
    Ecran,
    /// Un clavier.
    Clavier,
    /// Un découpeur de bus.
    Decoupe,
    /// Une jonction de fils en bus.
    Jonction,
    /// Une brique.
    Brique,
    /// Entrer dans une brique.
    Ouvrir,
    /// Séparateur du fil d'Ariane.
    Chevron,
    /// Regrouper en brique.
    Grouper,
    /// Dupliquer.
    Dupliquer,
    /// L'accueil.
    Maison,
    /// Les projets enregistrés.
    Dossier,
    /// Les paramètres.
    Engrenage,
    /// Quitter l'application.
    Sortie,
    /// Renommer.
    Crayon,
    /// Aller vers.
    Fleche,
    /// Revenir.
    Retour,
}

/// Le contour d'un rectangle arrondi, sommets dans le sens horaire (ouvert : le dernier point
/// ne répète pas le premier).
pub fn contour_arrondi(r: Rect, rayon: f32) -> Vec<Pos2> {
    let rayon = rayon.min(r.width() / 2.0).min(r.height() / 2.0).max(0.0);
    let coins = [
        (Pos2::new(r.right() - rayon, r.top() + rayon), -90.0_f32),
        (Pos2::new(r.right() - rayon, r.bottom() - rayon), 0.0),
        (Pos2::new(r.left() + rayon, r.bottom() - rayon), 90.0),
        (Pos2::new(r.left() + rayon, r.top() + rayon), 180.0),
    ];
    let mut pts = Vec::with_capacity(36);
    for (centre, debut) in coins {
        for i in 0..=8 {
            let t = (debut + 90.0 * i as f32 / 8.0).to_radians();
            pts.push(Pos2::new(
                centre.x + rayon * t.cos(),
                centre.y + rayon * t.sin(),
            ));
        }
    }
    pts
}

/// Ferme un contour en répétant son premier point à la fin.
pub fn fermer(mut pts: Vec<Pos2>) -> Vec<Pos2> {
    if let Some(p) = pts.first().copied() {
        pts.push(p);
    }
    pts
}

/// Rayons de coins différents en haut et en bas.
pub fn coins(haut: f32, bas: f32) -> CornerRadius {
    let h = haut.round().clamp(0.0, 255.0) as u8;
    let b = bas.round().clamp(0.0, 255.0) as u8;
    CornerRadius {
        nw: h,
        ne: h,
        sw: b,
        se: b,
    }
}

/// Un halo elliptique : `centre` au milieu, transparent au bord.
pub fn halo(rect: Rect, centre: Couleur) -> Shape {
    let mut mesh = Mesh::default();
    let c = rect.center();
    mesh.vertices.push(Vertex {
        pos: c,
        uv: WHITE_UV,
        color: centre.c(),
    });
    const N: u32 = 48;
    for i in 0..N {
        let t = i as f32 / N as f32 * std::f32::consts::TAU;
        mesh.vertices.push(Vertex {
            pos: Pos2::new(
                c.x + rect.width() / 2.0 * t.cos(),
                c.y + rect.height() / 2.0 * t.sin(),
            ),
            uv: WHITE_UV,
            color: Color32::TRANSPARENT,
        });
    }
    for i in 0..N {
        mesh.indices.extend([0, 1 + i, 1 + (i + 1) % N]);
    }
    Shape::mesh(mesh)
}

/// Les valeurs animées de l'interface, lissées image après image.
///
/// Chaque valeur est repérée par une clé. Trois mouvements existent : le lissage exponentiel
/// (sans dépassement), le ressort (avec un léger rebond, réglé par l'apparence) et la
/// transition minutée (une durée et une courbe).
#[derive(Debug, Default)]
pub struct Animations {
    lisses: HashMap<egui::Id, f32>,
    ressorts: HashMap<egui::Id, (f32, f32)>,
    /// Transitions minutées : valeur de départ, cible, part du temps écoulée.
    minutees: HashMap<egui::Id, (f32, f32, f32)>,
    /// Vrai si une valeur a bougé pendant l'image : il faut redessiner.
    pub en_mouvement: bool,
}

impl Animations {
    /// Rapproche la valeur `cle` de `cible` avec la constante de temps `tau` (secondes) et la renvoie.
    pub fn vers(&mut self, cle: egui::Id, cible: f32, dt: f32, tau: f32) -> f32 {
        let v = self.lisses.entry(cle).or_insert(cible);
        if (*v - cible).abs() < 0.002 || tau <= 0.0 {
            *v = cible;
        } else {
            *v += (cible - *v) * (1.0 - (-dt / tau).exp());
            self.en_mouvement = true;
        }
        *v
    }

    /// Comme [`Self::vers`], mais sans rien retenir pour une valeur encore inconnue : elle vaut
    /// alors `cible`, et s'oublie une fois arrivée. Pour les milliers de fils et de points de
    /// connexion d'un grand plan, dont presque aucun ne bouge.
    pub fn vers_si_connu(&mut self, cle: egui::Id, cible: f32, dt: f32, tau: f32) -> f32 {
        if !self.lisses.contains_key(&cle) {
            return cible;
        }
        let v = self.vers(cle, cible, dt, tau);
        if v == cible {
            self.lisses.remove(&cle);
        }
        v
    }

    /// Comme [`Self::ressort`], mais sans rien retenir pour une valeur encore inconnue.
    pub fn ressort_si_connu(
        &mut self,
        cle: egui::Id,
        cible: f32,
        dt: f32,
        raideur: f32,
        amortissement: f32,
    ) -> f32 {
        if !self.ressorts.contains_key(&cle) {
            return cible;
        }
        let x = self.ressort(cle, cible, dt, raideur, amortissement);
        if x == cible {
            self.ressorts.remove(&cle);
        }
        x
    }

    /// Comme [`Self::vers`], mais part de `depart` la première fois (apparitions).
    pub fn vers_depuis(
        &mut self,
        cle: egui::Id,
        depart: f32,
        cible: f32,
        dt: f32,
        tau: f32,
    ) -> f32 {
        self.lisses.entry(cle).or_insert(depart);
        self.vers(cle, cible, dt, tau)
    }

    /// Ressort amorti vers `cible`. `raideur` en 1/s², `amortissement` relatif (1 = critique).
    pub fn ressort(
        &mut self,
        cle: egui::Id,
        cible: f32,
        dt: f32,
        raideur: f32,
        amortissement: f32,
    ) -> f32 {
        let (x, v) = self.ressorts.entry(cle).or_insert((cible, 0.0));
        if (*x - cible).abs() < 0.0005 && v.abs() < 0.005 {
            *x = cible;
            *v = 0.0;
            return *x;
        }
        // Intégration par petits pas : stable même si une image dure longtemps.
        let mut reste = dt.min(0.1);
        while reste > 0.0 {
            let h = reste.min(1.0 / 240.0);
            let acc = raideur * (cible - *x) - 2.0 * amortissement * raideur.sqrt() * *v;
            *v += acc * h;
            *x += *v * h;
            reste -= h;
        }
        self.en_mouvement = true;
        *x
    }

    /// Transition minutée : la valeur `cle` va de là où elle est jusqu'à `cible` en `duree`
    /// secondes, en suivant `courbe`. Si la cible change en route, elle repart de sa valeur du
    /// moment, sans saut. Une durée nulle donne la cible tout de suite.
    pub fn transition(
        &mut self,
        cle: egui::Id,
        cible: f32,
        dt: f32,
        duree: f32,
        courbe: Courbe,
    ) -> f32 {
        let (depart, vise, t) = self.minutees.entry(cle).or_insert((cible, cible, 1.0));
        if *vise != cible {
            *depart += (*vise - *depart) * courbe.en(*t);
            *vise = cible;
            *t = 0.0;
        }
        if *t >= 1.0 || duree <= 0.0 {
            *t = 1.0;
            *depart = cible;
            return cible;
        }
        *t = (*t + dt.max(0.0) / duree).min(1.0);
        self.en_mouvement = true;
        *depart + (cible - *depart) * courbe.en(*t)
    }

    /// Comme [`Self::transition`], mais part de `depart` la première fois (arrivées).
    pub fn transition_depuis(
        &mut self,
        cle: egui::Id,
        depart: f32,
        cible: f32,
        dt: f32,
        duree: f32,
        courbe: Courbe,
    ) -> f32 {
        self.minutees.entry(cle).or_insert((depart, depart, 1.0));
        self.transition(cle, cible, dt, duree, courbe)
    }

    /// Place une valeur lissée à `v` : elle repartira de là vers sa cible.
    pub fn poser(&mut self, cle: egui::Id, v: f32) {
        self.lisses.insert(cle, v);
    }

    /// Place un ressort à la valeur `x`, à l'arrêt : il repartira de là vers sa cible.
    pub fn placer(&mut self, cle: egui::Id, x: f32) {
        self.ressorts.insert(cle, (x, 0.0));
    }

    /// Oublie une valeur (élément supprimé).
    pub fn oublier(&mut self, cle: egui::Id) {
        self.lisses.remove(&cle);
        self.ressorts.remove(&cle);
        self.minutees.remove(&cle);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn le_lissage_converge_sans_depasser() {
        let mut a = Animations::default();
        let cle = egui::Id::unique("x");
        assert_eq!(a.vers_depuis(cle, 0.0, 1.0, 0.0, 0.1), 0.0);
        let mut avant = 0.0;
        for _ in 0..200 {
            let v = a.vers(cle, 1.0, 1.0 / 60.0, 0.1);
            assert!(v >= avant && v <= 1.0);
            avant = v;
        }
        assert_eq!(avant, 1.0);
    }

    #[test]
    fn la_transition_minutee_dure_son_temps_et_repart_sans_saut() {
        let mut a = Animations::default();
        let cle = egui::Id::unique("t");
        let dt = 1.0 / 60.0;
        // La première fois, la valeur est déjà à sa cible.
        assert_eq!(a.transition(cle, 0.0, dt, 0.5, Courbe::Douce), 0.0);
        assert!(!a.en_mouvement);
        // Une demi-seconde à 60 images par seconde : 30 images, pas une de plus.
        let mut images = 0;
        let mut avant = 0.0;
        loop {
            let v = a.transition(cle, 1.0, dt, 0.5, Courbe::Douce);
            assert!(v >= avant, "la courbe douce ne recule pas");
            avant = v;
            images += 1;
            if v == 1.0 {
                break;
            }
        }
        assert!((29..=31).contains(&images), "{images} images");
        // La cible change en route : la valeur repart de là où elle est.
        a.transition(cle, 0.0, dt, 0.5, Courbe::Douce);
        for _ in 0..10 {
            avant = a.transition(cle, 0.0, dt, 0.5, Courbe::Douce);
        }
        let retour = a.transition(cle, 1.0, dt, 0.5, Courbe::Douce);
        assert!((retour - avant).abs() < 0.05, "saut de {avant} à {retour}");
        // Durée nulle (animations réduites) : la cible tout de suite.
        assert_eq!(a.transition(cle, 0.25, dt, 0.0, Courbe::Rebond), 0.25);
        // Une arrivée part de la valeur donnée.
        let autre = egui::Id::unique("u");
        let v = a.transition_depuis(autre, 0.0, 1.0, dt, 0.5, Courbe::Lineaire);
        assert!(v > 0.0 && v < 0.1);
    }

    #[test]
    fn le_ressort_rebondit_puis_se_pose() {
        let mut a = Animations::default();
        let cle = egui::Id::unique("r");
        a.ressort(cle, 0.0, 0.016, 520.0, 0.62);
        let mut max: f32 = 0.0;
        let mut dernier = 0.0;
        for _ in 0..300 {
            dernier = a.ressort(cle, 1.0, 1.0 / 60.0, 520.0, 0.62);
            max = max.max(dernier);
        }
        assert!(
            max > 1.0 && max < 1.2,
            "léger dépassement attendu, obtenu {max}"
        );
        assert_eq!(dernier, 1.0);
        // Une image très longue ne fait pas diverger le ressort.
        let mut b = Animations::default();
        b.ressort(cle, 0.0, 0.016, 5000.0, 0.05);
        let v = b.ressort(cle, 1.0, 3.0, 5000.0, 0.05);
        assert!(v.is_finite() && v.abs() < 3.0);
    }

    #[test]
    fn le_contour_arrondi_reste_dans_son_rectangle() {
        let r = Rect::from_min_size(Pos2::new(10.0, 20.0), Vec2::new(80.0, 64.0));
        let pts = contour_arrondi(r, 18.0);
        assert_eq!(pts.len(), 36);
        assert!(pts.iter().all(|p| r.expand(0.01).contains(*p)));
        // Rayon plus grand que le rectangle : ramené à la moitié du petit côté.
        assert!(contour_arrondi(r, 500.0)
            .iter()
            .all(|p| r.expand(0.01).contains(*p)));
    }
}
