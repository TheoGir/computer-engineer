//! Tracé des fils : itinéraires en angle droit, coudes arrondis, pastilles aux séparations.
//!
//! Tout est calculé à partir des positions affichées, à chaque image où quelque chose bouge :
//! les fils suivent donc un élément pendant qu'on le déplace.

use std::collections::HashMap;

use crate::document::{Document, Fil, Id, Point, Source};

/// Distance horizontale minimale pour qu'un fil aille droit vers la droite.
const AVANCE_MIN: f32 = 44.0;
/// Longueur du tronçon horizontal qui sort d'une source ou entre dans une arrivée
/// quand le fil doit revenir en arrière.
const DEGAGEMENT: f32 = 24.0;

/// Itinéraire en angle droit de `p1` (une sortie) à `p2` (une entrée).
///
/// `tronc` est l'abscisse du tronçon vertical partagé par les fils d'une même source,
/// ou 0 pour le placer à mi-chemin.
pub fn itineraire(p1: Point, p2: Point, tronc: f32) -> Vec<Point> {
    if p2.x - p1.x >= AVANCE_MIN {
        if (p2.y - p1.y).abs() <= 0.5 {
            return vec![p1, p2];
        }
        let mx = if tronc > 0.0 {
            tronc
        } else {
            (p1.x + p2.x) / 2.0
        };
        return vec![p1, Point::nouveau(mx, p1.y), Point::nouveau(mx, p2.y), p2];
    }
    // L'arrivée est à gauche de la source : on contourne, entre les deux si la place existe,
    // sinon par en dessous.
    let ym = if (p2.y - p1.y).abs() > 76.0 {
        (p1.y + p2.y) / 2.0
    } else {
        p1.y.max(p2.y) + 64.0
    };
    vec![
        p1,
        Point::nouveau(p1.x + DEGAGEMENT, p1.y),
        Point::nouveau(p1.x + DEGAGEMENT, ym),
        Point::nouveau(p2.x - DEGAGEMENT, ym),
        Point::nouveau(p2.x - DEGAGEMENT, p2.y),
        p2,
    ]
}

/// Point où deux fils partis de la même source se séparent, s'ils ont un tronçon commun.
pub fn divergence(a: &[Point], b: &[Point]) -> Option<Point> {
    let signe = |v: f32| {
        if v.abs() < 0.5 {
            0
        } else if v > 0.0 {
            1
        } else {
            -1
        }
    };
    for i in 0..a.len().min(b.len()).saturating_sub(1) {
        let (a0, a1, b1) = (a[i], a[i + 1], b[i + 1]);
        let da = (signe(a1.x - a0.x), signe(a1.y - a0.y));
        let db = (signe(b1.x - a0.x), signe(b1.y - a0.y));
        if da != db {
            return if i == 0 { None } else { Some(a0) };
        }
        let (la, lb) = (a0.distance(a1), a0.distance(b1));
        if (la - lb).abs() > 0.5 {
            return Some(if la < lb { a1 } else { b1 });
        }
    }
    None
}

/// Les fils d'une source, prêts à dessiner.
#[derive(Clone, Debug, Default)]
pub struct Faisceau {
    /// La source.
    pub source: Source,
    /// Chaque fil avec ses sommets.
    pub fils: Vec<(Fil, Vec<Point>)>,
    /// Pastilles de jonction.
    pub jonctions: Vec<Point>,
    /// Vrai si l'étiquette du signal se place sous la sortie plutôt qu'au-dessus.
    pub etiquette_dessous: bool,
    /// Position de la sortie.
    pub depart: Point,
}

/// Calcule les fils de toutes les sources. `position` donne la position affichée d'un élément.
pub fn tracer(doc: &Document, position: impl Fn(Id) -> Point) -> Vec<Faisceau> {
    // Une seule passe sur les arrivées, rangées par source.
    let mut par_source: HashMap<Source, Vec<(Fil, Point)>> = HashMap::new();
    for e in &doc.elements {
        if e.entrees.iter().all(Option::is_none) {
            continue;
        }
        let pos = position(e.id);
        for (i, src) in e.entrees.iter().enumerate() {
            if let Some(src) = src {
                par_source.entry(*src).or_default().push((
                    Fil {
                        puits: e.id,
                        entree: i,
                    },
                    e.entree_en(pos, i),
                ));
            }
        }
    }
    let mut faisceaux = Vec::new();
    for s in doc.sources() {
        let pos = position(s.id);
        for j in 0..s.broches_s.len() {
            let source = Source::nouvelle(s.id, j);
            let p1 = s.sortie_en(pos, j);
            let arrivees = par_source.remove(&source).unwrap_or_default();
            faisceaux.push(faisceau(source, p1, arrivees));
        }
    }
    faisceaux
}

fn faisceau(source: Source, p1: Point, arrivees: Vec<(Fil, Point)>) -> Faisceau {
    {
        let plus_proche = arrivees
            .iter()
            .filter(|(_, p2)| p2.x - p1.x >= AVANCE_MIN && (p2.y - p1.y).abs() > 0.5)
            .map(|(_, p2)| p2.x)
            .fold(f32::INFINITY, f32::min);
        let tronc = if plus_proche.is_finite() {
            p1.x + ((plus_proche - p1.x) / 2.0).max(22.0)
        } else {
            0.0
        };
        let fils: Vec<(Fil, Vec<Point>)> = arrivees
            .into_iter()
            .map(|(f, p2)| (f, itineraire(p1, p2, tronc)))
            .collect();
        let mut jonctions: Vec<Point> = Vec::new();
        for i in 0..fils.len() {
            for j in i + 1..fils.len() {
                if let Some(q) = divergence(&fils[i].1, &fils[j].1) {
                    if !jonctions.iter().any(|p| p.distance(q) < 0.5) {
                        jonctions.push(q);
                    }
                }
            }
        }
        let monte = fils
            .iter()
            .any(|(_, pts)| pts.iter().any(|p| p.y < p1.y - 1.0));
        let descend = fils
            .iter()
            .any(|(_, pts)| pts.iter().any(|p| p.y > p1.y + 1.0));
        Faisceau {
            source,
            fils,
            jonctions,
            etiquette_dessous: monte && !descend,
            depart: p1,
        }
    }
}

/// Remplace chaque coude d'une ligne brisée par un arc, et renvoie la ligne fine obtenue.
pub fn arrondir(sommets: &[Point], rayon: f32) -> Vec<Point> {
    if sommets.len() < 3 {
        return sommets.to_vec();
    }
    let mut ligne = vec![sommets[0]];
    for i in 1..sommets.len() - 1 {
        let (p0, p1, p2) = (sommets[i - 1], sommets[i], sommets[i + 1]);
        let (l1, l2) = (p0.distance(p1).max(1e-3), p1.distance(p2).max(1e-3));
        let r = rayon.min(l1 / 2.0).min(l2 / 2.0);
        let a = Point::nouveau(p1.x - (p1.x - p0.x) / l1 * r, p1.y - (p1.y - p0.y) / l1 * r);
        let b = Point::nouveau(p1.x + (p2.x - p1.x) / l2 * r, p1.y + (p2.y - p1.y) / l2 * r);
        const PAS: usize = 8;
        for k in 0..=PAS {
            let t = k as f32 / PAS as f32;
            let u = 1.0 - t;
            // Courbe de Bézier quadratique de a à b, de point de contrôle p1.
            ligne.push(Point::nouveau(
                u * u * a.x + 2.0 * u * t * p1.x + t * t * b.x,
                u * u * a.y + 2.0 * u * t * p1.y + t * t * b.y,
            ));
        }
    }
    ligne.push(sommets[sommets.len() - 1]);
    ligne
}

/// Longueur d'une ligne brisée.
pub fn longueur(ligne: &[Point]) -> f32 {
    ligne.windows(2).map(|w| w[0].distance(w[1])).sum()
}

/// Le morceau d'une ligne brisée compris entre les abscisses curvilignes `debut` et `fin`.
pub fn portion(ligne: &[Point], debut: f32, fin: f32) -> Vec<Point> {
    let mut morceau = Vec::new();
    if fin <= debut || ligne.len() < 2 {
        return morceau;
    }
    let mut parcouru = 0.0;
    for w in ligne.windows(2) {
        let l = w[0].distance(w[1]);
        let (d, f) = (parcouru, parcouru + l);
        parcouru = f;
        if f < debut || l <= 0.0 {
            continue;
        }
        if d > fin {
            break;
        }
        let point = |s: f32| {
            let t = ((s - d) / l).clamp(0.0, 1.0);
            Point::nouveau(
                w[0].x + (w[1].x - w[0].x) * t,
                w[0].y + (w[1].y - w[0].y) * t,
            )
        };
        if morceau.is_empty() {
            morceau.push(point(debut.max(d)));
        }
        morceau.push(point(fin.min(f)));
    }
    morceau
}

/// Le point d'abscisse curviligne `s` sur une ligne brisée.
pub fn point_a(ligne: &[Point], s: f32) -> Option<Point> {
    let mut parcouru = 0.0;
    for w in ligne.windows(2) {
        let l = w[0].distance(w[1]);
        if s <= parcouru + l && l > 0.0 {
            let t = ((s - parcouru) / l).clamp(0.0, 1.0);
            return Some(Point::nouveau(
                w[0].x + (w[1].x - w[0].x) * t,
                w[0].y + (w[1].y - w[0].y) * t,
            ));
        }
        parcouru += l;
    }
    None
}

/// Distance d'un point à une ligne brisée.
pub fn distance_a(ligne: &[Point], p: Point) -> f32 {
    let mut min = f32::INFINITY;
    for w in ligne.windows(2) {
        let (dx, dy) = (w[1].x - w[0].x, w[1].y - w[0].y);
        let l2 = dx * dx + dy * dy;
        let t = if l2 > 0.0 {
            (((p.x - w[0].x) * dx + (p.y - w[0].y) * dy) / l2).clamp(0.0, 1.0)
        } else {
            0.0
        };
        min = min.min(p.distance(Point::nouveau(w[0].x + dx * t, w[0].y + dy * t)));
    }
    min
}

#[cfg(test)]
mod tests {
    use super::*;

    fn p(x: f32, y: f32) -> Point {
        Point::nouveau(x, y)
    }

    #[test]
    fn un_fil_aligne_est_droit() {
        assert_eq!(
            itineraire(p(0.0, 10.0), p(100.0, 10.0), 0.0),
            vec![p(0.0, 10.0), p(100.0, 10.0)]
        );
    }

    #[test]
    fn un_fil_vers_l_avant_fait_deux_coudes_sur_le_tronc() {
        assert_eq!(
            itineraire(p(0.0, 0.0), p(100.0, 40.0), 30.0),
            vec![p(0.0, 0.0), p(30.0, 0.0), p(30.0, 40.0), p(100.0, 40.0)]
        );
        assert_eq!(
            itineraire(p(0.0, 0.0), p(100.0, 40.0), 0.0)[1],
            p(50.0, 0.0)
        );
    }

    #[test]
    fn un_fil_vers_l_arriere_contourne() {
        let r = itineraire(p(100.0, 0.0), p(0.0, 200.0), 0.0);
        assert_eq!(r.len(), 6);
        assert_eq!(r[2], p(124.0, 100.0));
        assert_eq!(r[3], p(-24.0, 100.0));
        // Trop proches en hauteur : passage par en dessous.
        assert_eq!(itineraire(p(100.0, 0.0), p(0.0, 20.0), 0.0)[2].y, 84.0);
    }

    #[test]
    fn le_plan_de_depart_a_les_jonctions_de_la_maquette() {
        let d = Document::demi_additionneur();
        let f = tracer(&d, |id| d.element(id).unwrap().pos);
        let de = |nom: &str| f.iter().find(|x| d.nom_signal(x.source) == nom).unwrap();
        assert_eq!(
            de("n1").jonctions,
            vec![p(360.0, 240.0), p(360.0, 315.0), p(360.0, 445.0)]
        );
        assert_eq!(de("a").jonctions, vec![p(153.5, 135.0)]);
        assert_eq!(de("b").jonctions, vec![p(153.5, 345.0)]);
        assert!(de("s").jonctions.is_empty());
        assert_eq!(
            de("n2").fils[0].1,
            vec![
                p(505.0, 150.0),
                p(560.0, 150.0),
                p(560.0, 225.0),
                p(615.0, 225.0)
            ]
        );
        assert!(
            de("n3").etiquette_dessous,
            "n3 ne fait que monter : son étiquette passe dessous"
        );
        assert!(!de("n1").etiquette_dessous);
    }

    #[test]
    fn arrondir_garde_les_extremites_et_reste_pres_des_sommets() {
        let brute = vec![p(0.0, 0.0), p(50.0, 0.0), p(50.0, 50.0), p(100.0, 50.0)];
        let ronde = arrondir(&brute, 14.0);
        assert_eq!(ronde[0], brute[0]);
        assert_eq!(*ronde.last().unwrap(), brute[3]);
        assert!(longueur(&ronde) < longueur(&brute));
        assert!(longueur(&ronde) > longueur(&brute) - 14.0);
        // Un coude trop court pour le rayon demandé ne dépasse jamais le milieu du segment.
        let serree = arrondir(
            &[p(0.0, 0.0), p(10.0, 0.0), p(10.0, 4.0), p(50.0, 4.0)],
            14.0,
        );
        assert!(serree
            .iter()
            .all(|q| q.y >= -0.01 && q.y <= 4.01 && q.x >= -0.01));
    }

    #[test]
    fn portion_et_point_suivent_l_abscisse_curviligne() {
        let l = vec![p(0.0, 0.0), p(10.0, 0.0), p(10.0, 10.0)];
        assert_eq!(longueur(&l), 20.0);
        assert_eq!(
            portion(&l, 5.0, 15.0),
            vec![p(5.0, 0.0), p(10.0, 0.0), p(10.0, 5.0)]
        );
        assert_eq!(portion(&l, 0.0, 20.0), l);
        assert!(portion(&l, 8.0, 8.0).is_empty());
        assert_eq!(point_a(&l, 15.0), Some(p(10.0, 5.0)));
        assert_eq!(point_a(&l, 25.0), None);
        assert_eq!(distance_a(&l, p(5.0, 3.0)), 3.0);
        assert_eq!(distance_a(&l, p(13.0, 14.0)), 5.0);
    }
}
