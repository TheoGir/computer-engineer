//! Transitions faites sur ce qui est déjà peint : on retouche, ou on recopie, une plage des
//! formes de l'image en cours.
//!
//! Deux outils :
//! - [`voiler`] change l'opacité d'une plage et la déplace ou l'agrandit (ce qui arrive en
//!   glissant, ce qui apparaît) ;
//! - [`Fantome`] garde une copie de ce qui était affiché juste avant un changement (écran, plan
//!   d'une brique, thème) et la repeint par-dessus le nouveau contenu pendant qu'elle s'efface.
//!
//! Rien ici ne touche à l'état de l'application ni aux zones cliquables : seules les formes
//! bougent, le temps d'une transition courte. La simulation n'en sait rien.

use egui::epaint::{shape_transform::adjust_colors, ClippedShape};
use egui::layers::ShapeIdx;
use egui::{emath::TSTransform, Color32, Painter, Pos2, Vec2};

/// Au-delà de ce nombre de formes, on renonce à une transition : sur un très grand plan,
/// recopier l'image coûterait plus que la transition n'apporte.
pub const FORMES_MAX: usize = 30_000;

/// Le nombre de formes déjà peintes sur la couche de ce pinceau : un repère pour désigner
/// ensuite « tout ce qui a été peint depuis ».
pub fn repere(p: &Painter) -> usize {
    p.ctx()
        .graphics(|g| g.get(p.layer_id()).map_or(0, |l| l.next_idx().0))
}

/// L'agrandissement de facteur `echelle` qui laisse le point `centre` immobile.
pub fn zoom_autour(centre: Pos2, echelle: f32) -> TSTransform {
    TSTransform::new(centre.to_vec2() * (1.0 - echelle), echelle)
}

fn opacifier(forme: &mut egui::Shape, opacite: f32) {
    if opacite < 1.0 {
        adjust_colors(forme, move |couleur| {
            if *couleur != Color32::PLACEHOLDER {
                *couleur = couleur.gamma_multiply(opacite);
            }
        });
    }
}

/// Retouche tout ce qui a été peint depuis `debut` : opacité multipliée par `opacite`, formes
/// déplacées par `transformation`. Les zones cliquables ne bougent pas.
pub fn voiler(p: &Painter, debut: usize, opacite: f32, transformation: TSTransform) {
    let opacite = opacite.clamp(0.0, 1.0);
    if opacite >= 1.0 && transformation == TSTransform::IDENTITY {
        return;
    }
    p.ctx().graphics_mut(|g| {
        let liste = g.entry(p.layer_id());
        let fin = liste.next_idx().0;
        if debut >= fin {
            return;
        }
        if opacite < 1.0 {
            for i in debut..fin {
                liste.mutate_shape(ShapeIdx(i), |f| {
                    if opacite <= 0.0 {
                        f.shape = egui::Shape::Noop;
                    } else {
                        opacifier(&mut f.shape, opacite);
                    }
                });
            }
        }
        if transformation != TSTransform::IDENTITY {
            // Après l'arrondi au pixel : un texte qui glisse ne tremble pas, et l'animation
            // finit exactement sur l'image sans transformation.
            liste.transform_after_rounding_range(ShapeIdx(debut), ShapeIdx(fin), transformation);
        }
    });
}

/// Une copie de ce qui était affiché, qui s'efface par-dessus ce qui l'a remplacé.
#[derive(Clone)]
pub struct Fantome {
    formes: Vec<ClippedShape>,
    /// L'instant de la copie, sur l'horloge de l'application.
    pub ne: f64,
    /// Le point de l'écran autour duquel le fantôme grossit ou rapetisse.
    pub centre: Pos2,
}

impl std::fmt::Debug for Fantome {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Fantome")
            .field("formes", &self.formes.len())
            .field("ne", &self.ne)
            .field("centre", &self.centre)
            .finish()
    }
}

impl Fantome {
    /// Copie les formes peintes de `debut` à `fin` sur la couche du pinceau. `None` s'il n'y a
    /// rien à copier, ou beaucoup trop.
    pub fn figer(p: &Painter, debut: usize, fin: usize, ne: f64, centre: Pos2) -> Option<Fantome> {
        let formes: Vec<ClippedShape> = p.ctx().graphics(|g| {
            g.get(p.layer_id()).map_or_else(Vec::new, |l| {
                let fin = fin.min(l.next_idx().0);
                if debut >= fin || fin - debut > FORMES_MAX {
                    return Vec::new();
                }
                l.all_entries()
                    .skip(debut)
                    .take(fin - debut)
                    .filter(|f| !matches!(f.shape, egui::Shape::Noop | egui::Shape::Callback(_)))
                    .cloned()
                    .collect()
            })
        });
        (!formes.is_empty()).then_some(Fantome { formes, ne, centre })
    }

    /// Le nombre de formes copiées.
    pub fn taille(&self) -> usize {
        self.formes.len()
    }

    /// Repeint la copie par-dessus ce qui est déjà peint, à cette opacité, agrandie de
    /// `echelle` autour de son centre et décalée de `glisse`.
    pub fn peindre(&self, p: &Painter, opacite: f32, echelle: f32, glisse: Vec2) {
        let opacite = opacite.clamp(0.0, 1.0);
        if opacite <= 0.0 {
            return;
        }
        let transformation =
            TSTransform::from_translation(glisse) * zoom_autour(self.centre, echelle);
        p.ctx().graphics_mut(|g| {
            let liste = g.entry(p.layer_id());
            for f in &self.formes {
                let mut f = f.clone();
                opacifier(&mut f.shape, opacite);
                if transformation != TSTransform::IDENTITY {
                    f.transform(transformation);
                }
                // Une forme copiée pendant une autre transition garde sa retouche d'après l'arrondi.
                let apres = f.transform_after_tessellation;
                let rang = liste.add(f.clip_rect, f.shape);
                if apres != TSTransform::IDENTITY {
                    liste.mutate_shape(rang, |c| c.transform_after_tessellation = apres);
                }
            }
        });
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn le_zoom_garde_son_centre_immobile() {
        let centre = Pos2::new(300.0, 120.0);
        for echelle in [0.5, 1.0, 2.4] {
            let t = zoom_autour(centre, echelle);
            let image = t * centre;
            assert!((image - centre).length() < 1e-3, "{echelle} : {image:?}");
            let autre = t * Pos2::new(310.0, 120.0);
            assert!((autre.x - (300.0 + 10.0 * echelle)).abs() < 1e-3);
        }
    }
}
