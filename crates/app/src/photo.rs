//! Exporter le plan en image : une photo du plan seul, sans les panneaux, cadrée sur tout le
//! circuit, découpée autour de lui et écrite en PNG (bibliothèque `image`).
//!
//! La photo se prend en quelques images : on cache les panneaux et on cadre le circuit sur
//! toute la fenêtre, on demande une capture d'écran à egui, puis on attend qu'elle arrive.
//! La caméra et la sélection sont rendues telles qu'elles étaient.

use std::sync::{Arc, Mutex};

use egui::{ColorImage, Rect};

use crate::etat::Selection;
use crate::plan::Camera;

/// Le nombre d'images dessinées avant la capture, le temps que le cadrage se pose.
const PREPARATION: u32 = 2;
/// Le nombre d'images au bout duquel on renonce si la capture n'arrive pas.
const PATIENCE: u32 = 120;

/// Une photo du plan en cours.
pub struct Photo {
    /// La caméra d'avant, rendue à la fin.
    pub camera: Camera,
    /// La sélection d'avant, cachée pendant la photo et rendue à la fin.
    pub selection: Option<Selection>,
    /// Les images dessinées depuis le début.
    pub images: u32,
    /// La zone du circuit à l'écran, en points, notée au moment de la capture.
    pub zone: Option<Rect>,
    /// La capture, quand elle est arrivée.
    recue: Arc<Mutex<Option<Arc<ColorImage>>>>,
}

/// Où en est la photo, après une image de plus.
pub enum Suite {
    /// Rien à faire de plus pour cette image.
    Attendre,
    /// Demander la capture de cette image.
    Capturer(egui::ScreenshotCallback),
    /// La capture est là : la découper dans cette zone (en points).
    Terminer(Arc<ColorImage>, Rect),
    /// La capture n'est jamais arrivée.
    Renoncer,
}

impl Photo {
    /// Une photo qui commence, avec ce qu'il faudra rendre à la fin.
    pub fn nouvelle(camera: Camera, selection: Option<Selection>) -> Self {
        Photo {
            camera,
            selection,
            images: 0,
            zone: None,
            recue: Arc::default(),
        }
    }

    /// Une image de plus a été dessinée ; `zone` est la place du circuit à l'écran.
    pub fn avancer(&mut self, zone: Rect) -> Suite {
        self.images += 1;
        let recue = self.recue.lock().ok().and_then(|mut r| r.take());
        if let (Some(image), Some(zone)) = (recue, self.zone) {
            return Suite::Terminer(image, zone);
        }
        if self.images == PREPARATION {
            self.zone = Some(zone);
            let boite = Arc::clone(&self.recue);
            return Suite::Capturer(egui::ScreenshotCallback::new(move |image| {
                if let Ok(mut r) = boite.lock() {
                    *r = Some(image);
                }
            }));
        }
        if self.images > PREPARATION + PATIENCE {
            return Suite::Renoncer;
        }
        Suite::Attendre
    }
}

/// Découpe dans la capture `image` (qui montre l'écran `ecran`, en points) la zone `zone`,
/// en pixels de l'image. `None` si la zone est vide.
pub fn decouper(image: &ColorImage, ecran: Rect, zone: Rect) -> Option<image::RgbaImage> {
    let [l, h] = image.size;
    if l == 0 || h == 0 || !ecran.is_positive() {
        return None;
    }
    let echelle = l as f32 / ecran.width();
    let z = zone.intersect(ecran).translate(-ecran.min.to_vec2()) * echelle;
    let x0 = (z.min.x.floor().max(0.0) as usize).min(l);
    let y0 = (z.min.y.floor().max(0.0) as usize).min(h);
    let x1 = (z.max.x.ceil().max(0.0) as usize).min(l);
    let y1 = (z.max.y.ceil().max(0.0) as usize).min(h);
    if x1 <= x0 || y1 <= y0 {
        return None;
    }
    let mut octets = Vec::with_capacity((x1 - x0) * (y1 - y0) * 4);
    for y in y0..y1 {
        for p in &image.pixels[y * l + x0..y * l + x1] {
            // Une capture d'écran est opaque : on écrit les couleurs sans transparence.
            let [r, g, b, _] = p.to_array();
            octets.extend_from_slice(&[r, g, b, 255]);
        }
    }
    image::RgbaImage::from_raw((x1 - x0) as u32, (y1 - y0) as u32, octets)
}

#[cfg(test)]
mod tests {
    use super::*;
    use egui::{Color32, Pos2, Vec2};

    #[test]
    fn la_decoupe_suit_l_echelle_de_l_ecran() {
        // Un écran de 100 × 50 points, capturé en 200 × 100 pixels.
        let mut image = ColorImage::filled([200, 100], Color32::BLACK);
        image.pixels[20 * 200 + 40] = Color32::from_rgb(1, 2, 3);
        let ecran = Rect::from_min_size(Pos2::ZERO, Vec2::new(100.0, 50.0));
        let zone = Rect::from_min_size(Pos2::new(20.0, 10.0), Vec2::new(30.0, 100.0));
        let png = decouper(&image, ecran, zone).expect("découpe");
        // La zone déborde en bas : elle s'arrête au bord de l'écran.
        assert_eq!(png.dimensions(), (60, 80));
        assert_eq!(png.get_pixel(0, 0).0, [1, 2, 3, 255]);
        assert_eq!(png.get_pixel(1, 0).0, [0, 0, 0, 255]);
        let dehors = Rect::from_min_size(Pos2::new(200.0, 0.0), Vec2::splat(10.0));
        assert!(decouper(&image, ecran, dehors).is_none());
    }

    #[test]
    fn la_photo_capture_puis_attend_la_capture() {
        let mut photo = Photo::nouvelle(Camera::default(), None);
        let zone = Rect::from_min_size(Pos2::ZERO, Vec2::splat(10.0));
        assert!(matches!(photo.avancer(zone), Suite::Attendre));
        let Suite::Capturer(rappel) = photo.avancer(zone) else {
            panic!("la capture est demandée après la préparation");
        };
        assert!(matches!(photo.avancer(zone), Suite::Attendre));
        rappel.complete(Arc::new(ColorImage::filled([4, 4], Color32::WHITE)));
        assert!(matches!(photo.avancer(zone), Suite::Terminer(_, z) if z == zone));
    }

    #[test]
    fn la_photo_renonce_si_rien_n_arrive() {
        let mut photo = Photo::nouvelle(Camera::default(), None);
        let zone = Rect::from_min_size(Pos2::ZERO, Vec2::splat(10.0));
        let mut renonce = false;
        for _ in 0..PREPARATION + PATIENCE + 1 {
            renonce = matches!(photo.avancer(zone), Suite::Renoncer);
        }
        assert!(renonce);
    }
}
