//! Des composants écrits en fichier (`ce-circuit`) aux briques de l'application.
//!
//! Chaque composant devient un modèle, avec un plan disposé automatiquement : les broches
//! d'entrée à gauche, les sorties à droite, et entre les deux une colonne par étage de
//! logique. Les morceaux de bus (`a[0:3]`) deviennent des découpeurs et des jonctions.
//!
//! Ce module ne change rien à ce que calcule le circuit : les tests comparent, sur
//! l'ordinateur miniature, la machine importée à la machine d'origine.

use std::collections::BTreeMap;

use ce_circuit::{Bibliotheque, Composant, Sens};

use crate::brique::Modele;
use crate::document::{Document, Element, Fil, Genre, Id, Point, Source};

/// Écart horizontal entre deux colonnes du plan.
const ENTRE_COLONNES: f32 = 130.0;
/// Écart vertical entre deux éléments d'une colonne.
const ENTRE_LIGNES: f32 = 26.0;

/// Importe le composant `racine` et tout ce qu'il utilise. Les modèles sont rendus dans
/// l'ordre où l'on peut les construire : chacun après ceux qu'il contient.
pub fn importer(bib: &Bibliotheque, racine: &str) -> Result<Vec<Modele>, String> {
    let mut faits: Vec<Modele> = Vec::new();
    let mut en_cours: Vec<String> = Vec::new();
    visiter(bib, racine, &mut faits, &mut en_cours)?;
    Ok(faits)
}

fn visiter(
    bib: &Bibliotheque,
    nom: &str,
    faits: &mut Vec<Modele>,
    en_cours: &mut Vec<String>,
) -> Result<(), String> {
    if faits.iter().any(|m| m.nom == nom) {
        return Ok(());
    }
    if en_cours.iter().any(|n| n == nom) {
        return Err(tr!("le composant « {nom} » se contient lui-même", nom = nom));
    }
    let comp = bib
        .composants
        .get(nom)
        .ok_or_else(|| tr!("le composant « {nom} » n'existe pas", nom = nom))?;
    en_cours.push(nom.to_owned());
    for inst in &comp.instances {
        match inst.genre.as_str() {
            "NAND" | "HORLOGE" => {}
            "TRI" | "ZERO" | "UN" => {
                return Err(tr!("« {nom} » utilise la primitive {genre}, que l'application ne sait pas encore poser", nom = nom, genre = inst.genre));
            }
            autre => visiter(bib, autre, faits, en_cours)?,
        }
    }
    en_cours.pop();
    let plan = plan_de(nom, comp, faits)?;
    faits.push(Modele {
        nom: nom.to_owned(),
        description: comp.description.clone(),
        plan,
        programme: None,
    });
    Ok(())
}

/// Une broche vue depuis l'intérieur du composant : celle du composant lui-même ou celle
/// d'une de ses instances.
struct Prise {
    /// L'élément du plan qui la porte.
    element: Id,
    /// Son rang parmi les entrées (ou les sorties) de cet élément.
    rang: usize,
    largeur: u32,
    /// Vrai si elle pilote le fil : entrée du composant ou sortie d'une instance.
    pilote: bool,
    /// Numéro de son premier bit dans la table commune.
    debut: usize,
}

fn trouver(uf: &mut [usize], mut i: usize) -> usize {
    while uf[i] != i {
        uf[i] = uf[uf[i]];
        i = uf[i];
    }
    i
}

/// Découpe `nom[3]`, `nom[0:3]` ou `nom` en nom et liste de bits.
fn extremite(
    texte: &str,
    largeur_de: impl Fn(&str) -> Option<u32>,
) -> Result<(String, Vec<u32>), String> {
    let faux = || tr!("extrémité « {texte} » illisible", texte = texte);
    let (nom, bits) = match texte.find('[') {
        None => (texte, None),
        Some(i) => {
            let dedans = texte[i + 1..].strip_suffix(']').ok_or_else(faux)?;
            (&texte[..i], Some(dedans))
        }
    };
    let largeur = largeur_de(nom).ok_or_else(|| tr!("broche « {nom} » inconnue", nom = nom))?;
    let bits: Vec<u32> = match bits {
        None => (0..largeur).collect(),
        Some(d) => match d.split_once(':') {
            None => vec![d.trim().parse().map_err(|_| faux())?],
            Some((a, b)) => {
                let a: u32 = a.trim().parse().map_err(|_| faux())?;
                let b: u32 = b.trim().parse().map_err(|_| faux())?;
                if a <= b {
                    (a..=b).collect()
                } else {
                    (b..=a).rev().collect()
                }
            }
        },
    };
    if bits.iter().any(|b| *b >= largeur) {
        return Err(tr!("« {texte} » dépasse la largeur de la broche", texte = texte));
    }
    Ok((nom.to_owned(), bits))
}

fn plan_de(nom: &str, comp: &Composant, faits: &[Modele]) -> Result<Document, String> {
    let ici = |e: String| tr!("dans « {nom} » : {e}", nom = nom, e = e);
    let mut plan = Document::nouveau();
    let mut prises: BTreeMap<String, Prise> = BTreeMap::new();
    let mut nb_bits = 0usize;
    let mut ajouter = |prises: &mut BTreeMap<String, Prise>,
                       cle: String,
                       element: Id,
                       rang: usize,
                       largeur: u32,
                       pilote: bool| {
        prises.insert(
            cle,
            Prise {
                element,
                rang,
                largeur,
                pilote,
                debut: nb_bits,
            },
        );
        nb_bits += largeur as usize;
    };

    // 1. Les broches du composant et ses instances, posées n'importe où pour l'instant.
    let mut entrees: Vec<Id> = Vec::new();
    let mut sorties: Vec<Id> = Vec::new();
    for b in &comp.broches {
        match b.sens {
            Sens::Entree => {
                let id = plan.poser(Element::entree(&b.nom, b.largeur), Point::default());
                ajouter(&mut prises, b.nom.clone(), id, 0, b.largeur, true);
                entrees.push(id);
            }
            Sens::Sortie => {
                let id = plan.poser(Element::sortie(&b.nom, "", b.largeur), Point::default());
                ajouter(&mut prises, b.nom.clone(), id, 0, b.largeur, false);
                sorties.push(id);
            }
        }
    }
    for inst in &comp.instances {
        match inst.genre.as_str() {
            "NAND" => {
                let id = plan.poser(Element::nand(&inst.nom), Point::default());
                ajouter(&mut prises, format!("{}.a", inst.nom), id, 0, 1, false);
                ajouter(&mut prises, format!("{}.b", inst.nom), id, 1, 1, false);
                ajouter(&mut prises, format!("{}.s", inst.nom), id, 0, 1, true);
            }
            "HORLOGE" => {
                let demi = inst.demi_periode.unwrap_or(1);
                let id = plan.poser(Element::horloge(&inst.nom, demi), Point::default());
                ajouter(&mut prises, format!("{}.s", inst.nom), id, 0, 1, true);
            }
            genre => {
                let modele = faits
                    .iter()
                    .find(|m| m.nom == genre)
                    .ok_or_else(|| ici(tr!("le composant « {genre} » manque", genre = genre)))?;
                let mut element = modele.instance();
                element.nom = inst.nom.clone();
                let (e, s) = (element.broches_e.clone(), element.broches_s.clone());
                let id = plan.poser(element, Point::default());
                for (i, b) in e.iter().enumerate() {
                    ajouter(
                        &mut prises,
                        format!("{}.{}", inst.nom, b.nom),
                        id,
                        i,
                        b.largeur,
                        false,
                    );
                }
                for (j, b) in s.iter().enumerate() {
                    ajouter(
                        &mut prises,
                        format!("{}.{}", inst.nom, b.nom),
                        id,
                        j,
                        b.largeur,
                        true,
                    );
                }
            }
        }
    }

    // 2. Les fils relient des bits : on réunit tous les bits d'un même fil.
    let mut uf: Vec<usize> = (0..nb_bits).collect();
    for fil in &comp.fils {
        let mut premier: Option<Vec<usize>> = None;
        for point in fil {
            let (cle, bits) =
                extremite(point, |n| prises.get(n).map(|p| p.largeur)).map_err(ici)?;
            let debut = prises[&cle].debut;
            let bits: Vec<usize> = bits.iter().map(|b| debut + *b as usize).collect();
            match &premier {
                None => premier = Some(bits),
                Some(p) => {
                    if p.len() != bits.len() {
                        return Err(ici(tr!("le fil [{v}] relie des largeurs différentes", v = fil.join(", "))));
                    }
                    for (a, b) in p.iter().zip(&bits) {
                        let (ra, rb) = (trouver(&mut uf, *a), trouver(&mut uf, *b));
                        uf[ra] = rb;
                    }
                }
            }
        }
    }
    // Pour chaque groupe de bits réunis : le bit qui le pilote.
    let mut pilote_de: BTreeMap<usize, (&Prise, u32)> = BTreeMap::new();
    for p in prises.values().filter(|p| p.pilote) {
        for k in 0..p.largeur {
            let r = trouver(&mut uf, p.debut + k as usize);
            pilote_de.entry(r).or_insert((p, k));
        }
    }

    // 3. Pour chaque broche qui lit : de quels morceaux de quelles sources elle est faite.
    struct Morceau {
        /// La source et son premier bit ; `None` : ces bits ne sont pilotés par rien.
        source: Option<(Source, u32, u32)>,
        largeur: u32,
    }
    let mut lectures: Vec<(Fil, Vec<Morceau>)> = Vec::new();
    for p in prises.values().filter(|p| !p.pilote) {
        let mut morceaux: Vec<Morceau> = Vec::new();
        for k in 0..p.largeur {
            let r = trouver(&mut uf, p.debut + k as usize);
            let pilote = pilote_de
                .get(&r)
                .map(|(d, bit)| (Source::nouvelle(d.element, d.rang), *bit, d.largeur));
            let suite = match (morceaux.last(), pilote) {
                (Some(m), Some((s, bit, _))) => m
                    .source
                    .is_some_and(|(ms, mb, _)| ms == s && mb + m.largeur == bit),
                (Some(m), None) => m.source.is_none(),
                (None, _) => false,
            };
            if suite {
                morceaux.last_mut().expect("il y a un morceau").largeur += 1;
            } else {
                morceaux.push(Morceau {
                    source: pilote,
                    largeur: 1,
                });
            }
        }
        if morceaux.iter().any(|m| m.source.is_some()) {
            lectures.push((
                Fil {
                    puits: p.element,
                    entree: p.rang,
                },
                morceaux,
            ));
        }
    }

    // 4. Un découpeur par source dont on ne prend qu'une partie.
    let entier = |m: &Morceau| {
        m.source
            .is_some_and(|(_, bit, l)| bit == 0 && m.largeur == l)
    };
    let mut plages: BTreeMap<Source, Vec<(u32, u32)>> = BTreeMap::new();
    for (_, morceaux) in &lectures {
        for m in morceaux.iter().filter(|m| !entier(m)) {
            if let Some((s, bit, _)) = m.source {
                let plage = (bit, bit + m.largeur - 1);
                let liste = plages.entry(s).or_default();
                if !liste.contains(&plage) {
                    liste.push(plage);
                }
            }
        }
    }
    let mut decoupeurs: BTreeMap<Source, Id> = BTreeMap::new();
    for (source, liste) in &mut plages {
        liste.sort_unstable();
        let largeur = plan.largeur_source(*source).unwrap_or(1);
        let id = plan.poser(Element::decoupe("", largeur, liste), Point::default());
        plan.relier(
            *source,
            Fil {
                puits: id,
                entree: 0,
            },
        );
        decoupeurs.insert(*source, id);
    }
    let depart = |m: &Morceau| -> Option<Source> {
        let (s, bit, _) = m.source?;
        if entier(m) {
            return Some(s);
        }
        let rang = plages[&s]
            .iter()
            .position(|p| *p == (bit, bit + m.largeur - 1))?;
        Some(Source::nouvelle(decoupeurs[&s], rang))
    };

    // 5. Les fils eux-mêmes, avec une jonction quand une broche est faite de plusieurs morceaux.
    for (arrivee, morceaux) in &lectures {
        if let [m] = morceaux.as_slice() {
            if let Some(s) = depart(m) {
                plan.relier(s, *arrivee);
            }
            continue;
        }
        let largeurs: Vec<u32> = morceaux.iter().map(|m| m.largeur).collect();
        let jonction = plan.poser(Element::jonction("", &largeurs), Point::default());
        for (i, m) in morceaux.iter().enumerate() {
            if let Some(s) = depart(m) {
                plan.relier(
                    s,
                    Fil {
                        puits: jonction,
                        entree: i,
                    },
                );
            }
        }
        plan.relier(Source::de(jonction), *arrivee);
    }

    disposer(&mut plan, &entrees, &sorties);
    Ok(plan)
}

/// Dispose un plan en colonnes : broches d'entrée et horloges à gauche, puis chaque élément
/// une colonne après la dernière de ses sources, les broches de sortie tout à droite.
pub fn disposer(plan: &mut Document, entrees: &[Id], sorties: &[Id]) {
    let n = plan.elements.len();
    if n == 0 {
        return;
    }
    let rang: BTreeMap<Id, usize> = plan
        .elements
        .iter()
        .enumerate()
        .map(|(i, e)| (e.id, i))
        .collect();
    let amont: Vec<Vec<usize>> = plan
        .elements
        .iter()
        .map(|e| {
            e.entrees
                .iter()
                .flatten()
                .filter_map(|s| rang.get(&s.id).copied())
                .collect()
        })
        .collect();

    // Étage de chaque élément : le plus long chemin depuis la gauche, boucles coupées.
    let mut etage = vec![0usize; n];
    let mut etat = vec![0u8; n];
    for depart in 0..n {
        if etat[depart] != 0 {
            continue;
        }
        let mut pile = vec![(depart, 0usize)];
        etat[depart] = 1;
        while let Some(&mut (i, ref mut k)) = pile.last_mut() {
            if let Some(&j) = amont[i].get(*k) {
                *k += 1;
                if etat[j] == 0 {
                    etat[j] = 1;
                    pile.push((j, 0));
                }
            } else {
                let source = matches!(plan.elements[i].genre, Genre::Entree | Genre::Horloge);
                etage[i] = if source {
                    0
                } else {
                    // Un élément encore « en cours » est en aval dans une boucle : on l'ignore.
                    1 + amont[i]
                        .iter()
                        .filter(|j| etat[**j] == 2)
                        .map(|j| etage[*j])
                        .max()
                        .unwrap_or(0)
                };
                etat[i] = 2;
                pile.pop();
            }
        }
    }
    let dernier = etage.iter().copied().max().unwrap_or(0) + 1;
    for (i, e) in plan.elements.iter().enumerate() {
        if e.genre == Genre::Sortie {
            etage[i] = dernier;
        }
    }

    // Colonne par colonne : chaque élément à la hauteur moyenne de ses sources, sans se
    // chevaucher ; les broches gardent l'ordre de leur déclaration.
    let mut y = vec![0f32; n];
    let mut x_colonne = 80.0f32;
    let mut largeur_avant = 0.0f32;
    for col in 0..=dernier {
        let mut membres: Vec<usize> = (0..n).filter(|i| etage[*i] == col).collect();
        if membres.is_empty() {
            continue;
        }
        let ordre_declare = |i: usize| {
            let id = plan.elements[i].id;
            entrees
                .iter()
                .chain(sorties)
                .position(|b| *b == id)
                .unwrap_or(usize::MAX)
        };
        let souhait = |i: usize| -> f32 {
            let a: Vec<f32> = plan.elements[i]
                .entrees
                .iter()
                .flatten()
                .filter_map(|s| {
                    let j = *rang.get(&s.id)?;
                    (etage[j] < col).then(|| y[j] + plan.elements[j].decalage_sortie(s.sortie))
                })
                .collect();
            if a.is_empty() {
                f32::NAN
            } else {
                a.iter().sum::<f32>() / a.len() as f32
            }
        };
        let souhaits: BTreeMap<usize, f32> = membres.iter().map(|i| (*i, souhait(*i))).collect();
        membres.sort_by(|a, b| {
            ordre_declare(*a).cmp(&ordre_declare(*b)).then(
                souhaits[a]
                    .partial_cmp(&souhaits[b])
                    .unwrap_or(std::cmp::Ordering::Equal)
                    .then(a.cmp(b)),
            )
        });
        let largeur = membres
            .iter()
            .map(|i| plan.elements[*i].largeur())
            .fold(0.0, f32::max);
        if col > 0 || largeur_avant > 0.0 {
            x_colonne += largeur_avant / 2.0 + ENTRE_COLONNES + largeur / 2.0;
        }
        largeur_avant = largeur;
        let mut bas = f32::NEG_INFINITY;
        for i in membres {
            let h = plan.elements[i].hauteur() + 14.0;
            let voulu = if souhaits[&i].is_nan() {
                120.0
            } else {
                souhaits[&i]
            };
            let centre = voulu.max(bas + ENTRE_LIGNES + h / 2.0);
            y[i] = (centre / 5.0).round() * 5.0;
            bas = y[i] + h / 2.0;
            plan.elements[i].pos = Point::nouveau((x_colonne / 5.0).round() * 5.0, y[i]);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::brique::Projet;
    use crate::simulation::Session;
    use ce_circuit::reference;

    fn projet(bib: &Bibliotheque, racine: &str) -> Projet {
        let mut p = Projet::nouveau("essai", Document::nouveau());
        for m in importer(bib, racine).unwrap() {
            p.ajouter(m);
        }
        p
    }

    #[test]
    fn l_additionneur_8_bits_importe_additionne() {
        let bib = reference::bibliotheque();
        let mut p = projet(&bib, "ADD8");
        let add8 = p.modele("ADD8").unwrap().clone();
        // Sur le plan principal : la brique, deux interrupteurs de 8 bits, deux lampes.
        let mut plan = Document::nouveau();
        let brique = plan.poser(add8.instance(), Point::nouveau(400.0, 200.0));
        let b = plan.element(brique).unwrap().clone();
        let mut entree = BTreeMap::new();
        for (i, broche) in b.broches_e.iter().enumerate() {
            let id = plan.poser(
                Element::entree(&broche.nom, broche.largeur),
                Point::nouveau(100.0, 100.0 * i as f32),
            );
            assert!(plan.relier(
                Source::de(id),
                Fil {
                    puits: brique,
                    entree: i
                }
            ));
            entree.insert(broche.nom.clone(), id);
        }
        let mut lampe = BTreeMap::new();
        for (j, broche) in b.broches_s.iter().enumerate() {
            let id = plan.poser(
                Element::sortie(&broche.nom, "", broche.largeur),
                Point::nouveau(700.0, 100.0 * j as f32),
            );
            assert!(plan.relier(
                Source::nouvelle(brique, j),
                Fil {
                    puits: id,
                    entree: 0
                }
            ));
            lampe.insert(broche.nom.clone(), id);
        }
        p.principal = plan.clone();
        let noms: Vec<&String> = entree.keys().collect();
        assert_eq!(noms.len(), 3, "a, b et la retenue entrante : {noms:?}");
        let mut s = Session::nouvelle(&plan, &p, &[]);
        assert_eq!(s.erreur(), None);
        let grande = |e: &BTreeMap<String, Id>, l: u32| {
            e.iter()
                .find(|(_, id)| plan.largeur_source(Source::de(**id)) == Some(l))
                .map(|(n, id)| (n.clone(), *id))
        };
        let retenue = grande(&entree, 1).unwrap().1;
        let mots: Vec<Id> = entree
            .values()
            .copied()
            .filter(|id| *id != retenue)
            .collect();
        for (a, b, r) in [
            (0u64, 0u64, 0u64),
            (1, 1, 0),
            (200, 100, 1),
            (255, 255, 1),
            (37, 5, 0),
        ] {
            s.regler(&plan, mots[0], a);
            s.regler(&plan, mots[1], b);
            s.regler(&plan, retenue, r);
            for _ in 0..200 {
                if s.stable() {
                    break;
                }
                s.pas(&plan);
            }
            assert!(s.stable());
            let total = a + b + r;
            let lue = |l: u32| {
                let id = lampe
                    .values()
                    .copied()
                    .find(|id| {
                        plan.largeur_entree(Fil {
                            puits: *id,
                            entree: 0,
                        }) == Some(l)
                    })
                    .unwrap();
                s.lecture_lue(
                    &plan,
                    Fil {
                        puits: id,
                        entree: 0,
                    },
                )
                .nombre()
            };
            assert_eq!(lue(8), Some(total & 255), "{a} + {b} + {r}");
            assert_eq!(lue(1), Some(total >> 8), "retenue de {a} + {b} + {r}");
        }
        assert_eq!(s.nb_portes(), p.nb_portes(&plan) as usize);
    }

    #[test]
    fn les_primitives_non_prises_en_charge_sont_refusees_clairement() {
        let bib = reference::bibliotheque();
        let e = importer(&bib, "SELECTEUR_BUS").unwrap_err();
        assert!(e.contains("TRI"), "{e}");
        assert!(importer(&bib, "RIEN").is_err());
    }

    #[test]
    fn la_disposition_ne_superpose_rien() {
        let bib = reference::bibliotheque();
        for m in importer(&bib, "REGISTRE8").unwrap() {
            let e = &m.plan.elements;
            for (i, a) in e.iter().enumerate() {
                for b in &e[i + 1..] {
                    let (ag, ah, ad, ab) = a.corps(a.pos);
                    let (bg, bh, bd, bb) = b.corps(b.pos);
                    let separes = ad <= bg || bd <= ag || ab <= bh || bb <= ah;
                    assert!(
                        separes,
                        "dans {} : {} et {} se chevauchent",
                        m.nom, a.nom, b.nom
                    );
                }
            }
            // Les broches gardent l'ordre de leur déclaration.
            let (entrees, _) = m.plan.interface();
            let declarees: Vec<String> = bib.composants[&m.nom]
                .broches
                .iter()
                .filter(|b| b.sens == Sens::Entree)
                .map(|b| b.nom.clone())
                .collect();
            let noms: Vec<String> = entrees.iter().map(|b| b.nom.clone()).collect();
            assert_eq!(noms, declarees, "dans {}", m.nom);
        }
    }
}
