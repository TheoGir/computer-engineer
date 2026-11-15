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
    itineraire_oriente(p1, 1.0, p2, 1.0, (tronc > 0.0).then_some(tronc))
}

/// Itinéraire en angle droit de `p1` à `p2`, quand les éléments peuvent être retournés.
/// `s1` est le sens où part le fil de la sortie (`1` vers la droite, `-1` vers la gauche),
/// `s2` le sens où il doit aller en arrivant sur l'entrée. `tronc` : l'abscisse du tronçon
/// vertical partagé par les fils d'une même source.
pub fn itineraire_oriente(
    p1: Point,
    s1: f32,
    p2: Point,
    s2: f32,
    tronc: Option<f32>,
) -> Vec<Point> {
    let miroir = |p: Point| Point::nouveau(-p.x, p.y);
    if s1 == s2 {
        if s1 > 0.0 {
            return droit(p1, p2, tronc);
        }
        // Les deux retournés : le même itinéraire, vu dans un miroir.
        return droit(miroir(p1), miroir(p2), tronc.map(|t| -t))
            .into_iter()
            .map(miroir)
            .collect();
    }
    // Sens contraires : le fil part, longe à la verticale au-delà des deux bouts, et revient.
    let x = if s1 > 0.0 {
        (p1.x + DEGAGEMENT).max(p2.x + DEGAGEMENT)
    } else {
        (p1.x - DEGAGEMENT).min(p2.x - DEGAGEMENT)
    };
    vec![p1, Point::nouveau(x, p1.y), Point::nouveau(x, p2.y), p2]
}

/// L'itinéraire d'une sortie qui va vers la droite à une entrée qu'on aborde vers la droite.
fn droit(p1: Point, p2: Point, tronc: Option<f32>) -> Vec<Point> {
    if p2.x - p1.x >= AVANCE_MIN {
        if (p2.y - p1.y).abs() <= 0.5 {
            return vec![p1, p2];
        }
        let mx = tronc.unwrap_or((p1.x + p2.x) / 2.0);
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
    /// Pour chaque fil (dans l'ordre de `fils`), les points où un fil d'une autre source le
    /// croise sans s'y relier : le fil y fait un petit pont ([`enjamber`]).
    pub ponts: Vec<Vec<Point>>,
    /// Vrai si l'étiquette du signal se place sous la sortie plutôt qu'au-dessus.
    pub etiquette_dessous: bool,
    /// Position de la sortie.
    pub depart: Point,
}

/// Calcule les fils de toutes les sources. `position` donne la position affichée d'un élément.
pub fn tracer(doc: &Document, position: impl Fn(Id) -> Point) -> Vec<Faisceau> {
    // Une seule passe sur les arrivées, rangées par source.
    let mut par_source: HashMap<Source, Vec<(Fil, Point, f32)>> = HashMap::new();
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
                    e.sens(),
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
            faisceaux.push(faisceau(source, p1, s.sens(), arrivees));
        }
    }
    let nombre: usize = faisceaux
        .iter()
        .flat_map(|f| &f.fils)
        .map(|(_, pts)| pts.len())
        .sum();
    let lisible = nombre <= TRONCONS_MAX;
    if lisible {
        separer_voies(&mut faisceaux, true);
        separer_voies(&mut faisceaux, false);
    }
    for f in &mut faisceaux {
        finir(f);
    }
    if lisible {
        chercher_ponts(&mut faisceaux);
    }
    faisceaux
}

fn faisceau(source: Source, p1: Point, s1: f32, arrivees: Vec<(Fil, Point, f32)>) -> Faisceau {
    // L'arrivée la plus proche dans le sens où part le fil : le tronc commun s'arrête à
    // mi-chemin d'elle.
    let plus_proche = arrivees
        .iter()
        .filter(|(_, p2, s2)| {
            *s2 == s1 && (p2.x - p1.x) * s1 >= AVANCE_MIN && (p2.y - p1.y).abs() > 0.5
        })
        .map(|(_, p2, _)| (p2.x - p1.x) * s1)
        .fold(f32::INFINITY, f32::min);
    let tronc = plus_proche
        .is_finite()
        .then(|| p1.x + s1 * (plus_proche / 2.0).max(22.0));
    let fils: Vec<(Fil, Vec<Point>)> = arrivees
        .into_iter()
        .map(|(f, p2, s2)| (f, itineraire_oriente(p1, s1, p2, s2, tronc)))
        .collect();
    Faisceau {
        source,
        ponts: vec![Vec::new(); fils.len()],
        fils,
        jonctions: Vec::new(),
        etiquette_dessous: false,
        depart: p1,
    }
}

/// Pastilles de jonction et place de l'étiquette, une fois les itinéraires fixés.
fn finir(f: &mut Faisceau) {
    let fils = &f.fils;
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
    let y = f.depart.y;
    let monte = fils
        .iter()
        .any(|(_, pts)| pts.iter().any(|p| p.y < y - 1.0));
    let descend = fils
        .iter()
        .any(|(_, pts)| pts.iter().any(|p| p.y > y + 1.0));
    f.jonctions = jonctions;
    f.etiquette_dessous = monte && !descend;
}

/// Écart entre deux voies parallèles, quand des fils de sources différentes passeraient l'un
/// sur l'autre. Un fil fait 6 unités : il reste 6 unités de fond entre les deux.
pub const ECART_VOIES: f32 = 12.0;
/// Deux tronçons parallèles plus proches que cela se confondent à l'écran.
const CONFONDUS: f32 = 3.0;
/// Deux tronçons alignés dont les bouts sont à moins de cela l'un de l'autre semblent se
/// toucher.
const PRESQUE: f32 = 6.0;
/// Longueur minimale qu'un tronçon voisin garde quand on décale une voie.
const VOISIN_MIN: f32 = 8.0;
/// Au-delà de ce nombre de tronçons, ni voies ni ponts : le plan est trop grand pour que ça
/// se lise de toute façon, et le tracé doit rester rapide.
pub const TRONCONS_MAX: usize = 60_000;

/// Un tronçon droit à l'intérieur d'un fil : du sommet `i` au sommet `i + 1`.
#[derive(Clone, Copy, Debug)]
struct Troncon {
    faisceau: usize,
    fil: usize,
    i: usize,
    /// Abscisse d'un tronçon vertical, ordonnée d'un tronçon horizontal.
    pos: f32,
    /// Étendue dans l'autre sens.
    de: f32,
    a: f32,
}

fn le_long(p: Point, vertical: bool) -> f32 {
    if vertical {
        p.y
    } else {
        p.x
    }
}

fn en_travers(p: Point, vertical: bool) -> f32 {
    if vertical {
        p.x
    } else {
        p.y
    }
}

/// Les tronçons verticaux (ou horizontaux) de tous les fils. Avec `interieurs`, seulement ceux
/// qui ne touchent pas une broche : on peut les décaler sans débrancher le fil.
fn troncons(faisceaux: &[Faisceau], vertical: bool, interieurs: bool) -> Vec<Troncon> {
    let mut liste = Vec::new();
    for (k, f) in faisceaux.iter().enumerate() {
        for (n, (_, pts)) in f.fils.iter().enumerate() {
            let fin = pts.len().saturating_sub(1);
            for i in 0..fin {
                if interieurs && (i == 0 || i + 1 == fin) {
                    continue;
                }
                let (p, q) = (pts[i], pts[i + 1]);
                if (en_travers(p, vertical) - en_travers(q, vertical)).abs() > 0.01
                    || (le_long(p, vertical) - le_long(q, vertical)).abs() < 0.5
                {
                    continue;
                }
                let (u, v) = (le_long(p, vertical), le_long(q, vertical));
                liste.push(Troncon {
                    faisceau: k,
                    fil: n,
                    i,
                    pos: en_travers(p, vertical),
                    de: u.min(v),
                    a: u.max(v),
                });
            }
        }
    }
    liste
}

/// Le décalage de la voie `rang` : 0, puis +1, -1, +2, -2… écarts.
fn decalage(rang: usize) -> f32 {
    let n = rang.div_ceil(2) as f32;
    if rang % 2 == 1 {
        n * ECART_VOIES
    } else {
        -n * ECART_VOIES
    }
}

/// Quand des fils de sources différentes empruntent le même couloir (même abscisse pour des
/// tronçons verticaux, même ordonnée pour des horizontaux) et se chevauchent, on les écarte sur
/// des voies parallèles : sinon deux signaux distincts ont l'air d'un seul fil. Les fils d'une
/// même source restent ensemble (ils partagent leur tronc), et un tronçon qui touche une
/// broche ne bouge jamais.
fn separer_voies(faisceaux: &mut [Faisceau], vertical: bool) {
    let mut liste = troncons(faisceaux, vertical, true);
    if liste.len() < 2 {
        return;
    }
    liste.sort_by(|a, b| a.pos.total_cmp(&b.pos));
    let mut debut = 0;
    while debut < liste.len() {
        let mut fin = debut + 1;
        while fin < liste.len() && liste[fin].pos - liste[fin - 1].pos <= CONFONDUS {
            fin += 1;
        }
        let groupe = &liste[debut..fin];
        debut = fin;
        if groupe.iter().all(|t| t.faisceau == groupe[0].faisceau) {
            continue;
        }
        // L'étendue de chaque source dans ce couloir.
        let mut sources: Vec<(usize, f32, f32)> = Vec::new();
        for t in groupe {
            match sources.iter_mut().find(|s| s.0 == t.faisceau) {
                Some(s) => {
                    s.1 = s.1.min(t.de);
                    s.2 = s.2.max(t.a);
                }
                None => sources.push((t.faisceau, t.de, t.a)),
            }
        }
        let touche = |a: &(usize, f32, f32), b: &(usize, f32, f32)| {
            a.1 < b.2 + PRESQUE && b.1 < a.2 + PRESQUE
        };
        if !sources
            .iter()
            .enumerate()
            .any(|(i, a)| sources[i + 1..].iter().any(|b| touche(a, b)))
        {
            continue;
        }
        sources.sort_by(|a, b| a.1.total_cmp(&b.1).then(a.0.cmp(&b.0)));
        let mut places: Vec<((usize, f32, f32), usize)> = Vec::new();
        for s in sources {
            let prises: Vec<usize> = places
                .iter()
                .filter(|(autre, _)| touche(autre, &s))
                .map(|(_, rang)| *rang)
                .collect();
            let siens: Vec<&Troncon> = groupe.iter().filter(|t| t.faisceau == s.0).collect();
            let possible = |rang: usize| {
                let d = decalage(rang);
                siens.iter().all(|t| {
                    let pts = &faisceaux[t.faisceau].fils[t.fil].1;
                    let ici = en_travers(pts[t.i], vertical) + d;
                    let (avant, apres) = (
                        en_travers(pts[t.i - 1], vertical),
                        en_travers(pts[t.i + 2], vertical),
                    );
                    let garde = |de: f32, vers_avant: f32, vers_apres: f32| {
                        vers_avant.abs() >= 0.5
                            && (vers_apres - de) * vers_avant.signum() >= VOISIN_MIN
                    };
                    garde(avant, t.pos - avant, ici) && garde(ici, apres - t.pos, apres)
                })
            };
            let rang = (0..8)
                .find(|r| !prises.contains(r) && (*r == 0 || possible(*r)))
                .unwrap_or(0);
            if rang > 0 {
                let d = decalage(rang);
                for t in &siens {
                    let pts = &mut faisceaux[t.faisceau].fils[t.fil].1;
                    for q in &mut pts[t.i..=t.i + 1] {
                        if vertical {
                            q.x += d;
                        } else {
                            q.y += d;
                        }
                    }
                }
            }
            places.push((s, rang));
        }
    }
}

/// Les croisements sans connexion : là où un tronçon horizontal d'un fil coupe un tronçon
/// vertical d'un fil d'une autre source, le fil horizontal fait un pont.
fn chercher_ponts(faisceaux: &mut [Faisceau]) {
    const CASE: f32 = 128.0;
    let verticaux = troncons(faisceaux, true, false);
    let horizontaux = troncons(faisceaux, false, false);
    if verticaux.is_empty() || horizontaux.is_empty() {
        return;
    }
    let case = |v: f32| (v / CASE).floor() as i32;
    let mut grille: HashMap<(i32, i32), Vec<usize>> = HashMap::new();
    for (n, v) in verticaux.iter().enumerate() {
        for cy in case(v.de)..=case(v.a) {
            grille.entry((case(v.pos), cy)).or_default().push(n);
        }
    }
    for h in &horizontaux {
        let cy = case(h.pos);
        let ponts = &mut faisceaux[h.faisceau].ponts[h.fil];
        for cx in case(h.de)..=case(h.a) {
            let Some(dans) = grille.get(&(cx, cy)) else {
                continue;
            };
            for &n in dans {
                let v = &verticaux[n];
                if v.faisceau == h.faisceau
                    || v.pos <= h.de + 0.5
                    || v.pos >= h.a - 0.5
                    || h.pos <= v.de + 0.5
                    || h.pos >= v.a - 0.5
                {
                    continue;
                }
                let q = Point::nouveau(v.pos, h.pos);
                if !ponts.iter().any(|p| p.distance(q) < 0.5) {
                    ponts.push(q);
                }
            }
        }
    }
}

/// Une ligne (déjà arrondie) qui enjambe chaque point de `ponts` par un demi-cercle de rayon
/// `r` tourné vers le haut. Un pont trop près d'un coude ou d'un autre pont est omis.
/// Renvoie aussi, pour chaque pont tracé, la plage de ses sommets dans la ligne : on le
/// redessine par-dessus les autres fils.
pub fn enjamber(
    ligne: &[Point],
    ponts: &[Point],
    r: f32,
) -> (Vec<Point>, Vec<std::ops::Range<usize>>) {
    let mut arcs = Vec::new();
    if ponts.is_empty() || ligne.len() < 2 || r <= 0.0 {
        return (ligne.to_vec(), arcs);
    }
    const PAS: usize = 10;
    let mut sortie = vec![ligne[0]];
    for w in ligne.windows(2) {
        let (p, q) = (w[0], w[1]);
        let dx = q.x - p.x;
        if (q.y - p.y).abs() < 0.01 && dx.abs() > 2.0 * r {
            let sens = dx.signum();
            let mut ici: Vec<f32> = ponts
                .iter()
                .filter(|c| {
                    (c.y - p.y).abs() < 0.5 && (c.x - p.x) * sens > r && (q.x - c.x) * sens > r
                })
                .map(|c| c.x)
                .collect();
            ici.sort_by(|a, b| ((a - p.x) * sens).total_cmp(&((b - p.x) * sens)));
            let mut libre: Option<f32> = None;
            for x in ici {
                if libre.is_some_and(|l| (x - sens * r - l) * sens < 0.0) {
                    continue;
                }
                arcs.push(sortie.len()..sortie.len() + PAS + 1);
                for k in 0..=PAS {
                    let phi = std::f32::consts::PI * k as f32 / PAS as f32;
                    sortie.push(Point::nouveau(
                        x - sens * r * phi.cos(),
                        p.y - r * phi.sin(),
                    ));
                }
                libre = Some(x + sens * r);
            }
        }
        sortie.push(q);
    }
    (sortie, arcs)
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

    /// Chaque tronçon est horizontal ou vertical, et le fil part et arrive dans le bon sens.
    fn verifier(l: &[Point], s1: f32, s2: f32) {
        for w in l.windows(2) {
            assert!(
                (w[0].x - w[1].x).abs() < 1e-3 || (w[0].y - w[1].y).abs() < 1e-3,
                "{l:?}"
            );
        }
        let n = l.len();
        assert!((l[1].x - l[0].x) * s1 > 0.0, "départ : {l:?}");
        assert!((l[n - 1].x - l[n - 2].x) * s2 > 0.0, "arrivée : {l:?}");
    }

    #[test]
    fn les_elements_retournes_ont_des_fils_dans_le_bon_sens() {
        for (a, b) in [
            (p(0.0, 0.0), p(200.0, 50.0)),
            (p(200.0, 0.0), p(0.0, 50.0)),
            (p(0.0, 0.0), p(10.0, 0.0)),
            (p(0.0, 0.0), p(-300.0, -80.0)),
        ] {
            for (s1, s2) in [(1.0, 1.0), (-1.0, -1.0), (1.0, -1.0), (-1.0, 1.0)] {
                let l = itineraire_oriente(a, s1, b, s2, None);
                assert_eq!((l[0], l[l.len() - 1]), (a, b));
                if l.len() > 2 {
                    verifier(&l, s1, s2);
                }
            }
        }
        // Deux éléments retournés : l'image dans un miroir des deux éléments à l'endroit.
        let droit = itineraire(p(-200.0, 0.0), p(0.0, 60.0), 0.0);
        let retourne = itineraire_oriente(p(200.0, 0.0), -1.0, p(0.0, 60.0), -1.0, None);
        let miroir: Vec<Point> = droit.iter().map(|q| p(-q.x, q.y)).collect();
        assert_eq!(retourne, miroir);
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

    /// Quatre interrupteurs empilés lus dans le désordre par une colonne de portes : leurs
    /// troncs tombent tous sur la même abscisse.
    fn colonne_melangee() -> Document {
        use crate::document::Element;
        let mut d = Document::nouveau();
        let e: Vec<Id> = (0..4)
            .map(|i| {
                d.poser(
                    Element::entree(&format!("e{i}"), 1),
                    p(61.0, 120.0 + 90.0 * i as f32),
                )
            })
            .collect();
        let g: Vec<Id> = (0..4)
            .map(|i| d.poser(Element::nand(""), p(330.0, 120.0 + 90.0 * i as f32)))
            .collect();
        for (puits, entree, source) in [
            (0, 0, 0),
            (0, 1, 2),
            (1, 0, 1),
            (1, 1, 3),
            (2, 0, 0),
            (2, 1, 3),
            (3, 0, 2),
            (3, 1, 1),
        ] {
            assert!(d.relier(
                Source::de(e[source]),
                Fil {
                    puits: g[puits],
                    entree
                }
            ));
        }
        d
    }

    /// Les tronçons de deux sources différentes ne se recouvrent jamais.
    fn sans_recouvrement(f: &[Faisceau]) {
        for vertical in [true, false] {
            let t = troncons(f, vertical, false);
            for (n, a) in t.iter().enumerate() {
                for b in &t[n + 1..] {
                    if a.faisceau != b.faisceau && (a.pos - b.pos).abs() <= CONFONDUS {
                        assert!(
                            a.a <= b.de + 0.5 || b.a <= a.de + 0.5,
                            "deux signaux l'un sur l'autre : {a:?} et {b:?}"
                        );
                    }
                }
            }
        }
    }

    #[test]
    fn des_signaux_differents_ne_partagent_pas_un_couloir() {
        let d = colonne_melangee();
        let f = tracer(&d, |id| d.element(id).unwrap().pos);
        sans_recouvrement(&f);
        // Les fils restent en angle droit, partent de la sortie et arrivent sur l'entrée.
        for fx in f.iter().filter(|x| !x.fils.is_empty()) {
            for (fil, pts) in &fx.fils {
                verifier(pts, 1.0, 1.0);
                assert_eq!(pts[0], fx.depart);
                let e = d.element(fil.puits).unwrap();
                assert_eq!(*pts.last().unwrap(), e.entree_en(e.pos, fil.entree));
            }
            // Chaque source garde son tronc commun : une pastille où ses deux fils se séparent.
            assert_eq!(fx.fils.len(), 2);
            assert_eq!(fx.jonctions.len(), 1, "{:?}", fx.fils);
        }
        // Les fils se croisent sans se relier : chaque croisement porte un pont.
        let ponts: usize = f.iter().flat_map(|x| &x.ponts).map(Vec::len).sum();
        assert!(ponts >= 4, "{ponts} ponts");
        for fx in &f {
            for ((_, pts), ponts) in fx.fils.iter().zip(&fx.ponts) {
                for q in ponts {
                    assert!(distance_a(pts, *q) < 0.01, "le pont est sur son fil");
                }
            }
        }
    }

    #[test]
    fn les_fils_d_une_seule_source_ne_bougent_pas() {
        // Le demi-additionneur n'a aucun couloir partagé : ses tracés sont ceux d'avant.
        let d = Document::demi_additionneur();
        let f = tracer(&d, |id| d.element(id).unwrap().pos);
        sans_recouvrement(&f);
    }

    #[test]
    fn enjamber_fait_un_demi_cercle_au_dessus_du_croisement() {
        let l = vec![p(0.0, 0.0), p(100.0, 0.0)];
        let (pont, arcs) = enjamber(&l, &[p(50.0, 0.0)], 9.0);
        assert_eq!(arcs.len(), 1);
        assert_eq!(pont[0], l[0]);
        assert_eq!(*pont.last().unwrap(), l[1]);
        let haut = pont.iter().map(|q| q.y).fold(f32::INFINITY, f32::min);
        assert!((haut + 9.0).abs() < 0.01, "le pont monte de son rayon");
        // Le sommet du pont est juste au-dessus du croisement.
        let sommet = pont.iter().find(|q| (q.y + 9.0).abs() < 0.01).unwrap();
        assert!((sommet.x - 50.0).abs() < 0.01);
        assert!((longueur(&pont) - (100.0 - 18.0 + std::f32::consts::PI * 9.0)).abs() < 0.2);
        // Dans l'autre sens aussi, et rien près d'un bout ni sur un tronçon vertical.
        let (retour, arcs) = enjamber(&[p(100.0, 0.0), p(0.0, 0.0)], &[p(50.0, 0.0)], 9.0);
        assert_eq!(arcs.len(), 1);
        assert!(retour.iter().all(|q| q.y <= 1e-4));
        assert!(enjamber(&l, &[p(4.0, 0.0)], 9.0).1.is_empty());
        assert!(
            enjamber(&[p(0.0, 0.0), p(0.0, 100.0)], &[p(0.0, 50.0)], 9.0)
                .1
                .is_empty()
        );
        // Deux croisements trop serrés : un seul pont.
        assert_eq!(enjamber(&l, &[p(50.0, 0.0), p(55.0, 0.0)], 9.0).1.len(), 1);
    }
}
