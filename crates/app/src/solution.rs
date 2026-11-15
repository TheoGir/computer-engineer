//! La solution de chaque quête, à regarder : la plus petite construction connue.
//!
//! Pour une quête de brique, la solution est le plan du composant pris dans
//! [`ce_circuit::minimum::bibliotheque_des_solutions`] : le corrigé, remplacé par une
//! construction plus petite quand la recherche en a trouvé une (le nombre premier en 8 NAND au
//! lieu de 115…). Ses interrupteurs et ses lampes portent les noms de la quête, et les briques
//! qu'il utilise sont dans la palette. Pour un ordinateur ou un grand projet, c'est l'atelier
//! de la quête, branché comme le dit la consigne, avec les grandes pièces de la même
//! bibliothèque.
//!
//! Le test `chaque_solution_reussit_sa_quete` vérifie, pour chacune des quêtes, que la solution
//! réussit la vérification et qu'elle a exactement le nombre de portes annoncé comme cible.

use ce_circuit::minimum::bibliotheque_des_solutions;

use crate::brique::Projet;
use crate::document::{Document, Element, Fil, Genre, Id, Point, Source};
use crate::parcours::Parcours;
use crate::quete::{Atelier, Epreuve, Fiche, Quete};

/// Ajoute au projet le composant `nom` de la bibliothèque des solutions, et ceux qu'il
/// utilise, sauf ceux que le projet a déjà.
fn importer(p: &mut Projet, nom: &str) -> Result<(), String> {
    for m in crate::importer::importer(bibliotheque_des_solutions(), nom)? {
        if p.modele(&m.nom).is_none() {
            p.ajouter(m);
        }
    }
    Ok(())
}

/// Le projet qui montre la solution d'une quête. Il n'est pas lié à la quête : le vérifier ne
/// fait rien gagner.
pub fn projet(fiche: &'static Fiche, nom: &str) -> Result<Projet, String> {
    let mut p = match fiche.epreuve {
        Epreuve::Ordinateur { .. } | Epreuve::Systeme { .. } => {
            let mut p = Parcours::default().projet(fiche, nom)?;
            p.quete = None;
            brancher_la_quete(&mut p, fiche)?;
            p
        }
        Epreuve::Combinatoire | Epreuve::Sequentielle => {
            let modeles = crate::importer::importer(bibliotheque_des_solutions(), fiche.corrige)?;
            let racine = modeles
                .iter()
                .find(|m| m.nom == fiche.corrige)
                .ok_or_else(|| tr!("la brique {modele} manque", modele = fiche.corrige))?;
            let mut plan = racine.plan.clone();
            nommer_comme_la_quete(&mut plan, &Quete::de_fiche(fiche));
            let mut p = Projet::nouveau(nom, plan);
            for m in modeles {
                if m.nom != fiche.corrige {
                    p.ajouter(m);
                }
            }
            p
        }
    };
    p.quete = None;
    Ok(p)
}

/// Donne aux interrupteurs et aux lampes du plan du composant les noms qu'ils portent dans la
/// quête (une broche du corrigé peut être renommée dans la quête).
fn nommer_comme_la_quete(plan: &mut Document, q: &Quete) {
    let renommages: Vec<(Genre, String, String)> = q
        .entrees
        .iter()
        .map(|a| (Genre::Entree, a.corrige.clone(), a.nom.clone()))
        .chain(
            q.sorties
                .iter()
                .map(|a| (Genre::Sortie, a.corrige.clone(), a.nom.clone())),
        )
        .filter(|(_, de, vers)| de != vers)
        .collect();
    for e in &mut plan.elements {
        if let Some((_, _, vers)) = renommages
            .iter()
            .find(|(genre, de, _)| *genre == e.genre && *de == e.nom)
        {
            e.nom = vers.clone();
        }
    }
}

/// Pose les grandes pièces qui manquent à l'atelier et branche tout, comme le dit la consigne.
fn brancher_la_quete(p: &mut Projet, fiche: &'static Fiche) -> Result<(), String> {
    let manque = || tr!("l'atelier ne se branche pas").to_owned();
    match fiche.epreuve {
        Epreuve::Systeme {
            atelier: Atelier::Deux { .. },
            ..
        } => brancher_les_deux(p).ok_or_else(manque),
        Epreuve::Systeme {
            atelier: Atelier::GrandeMemoire { .. },
            ..
        } => {
            poser(p, "RAM256", "ram", Point::nouveau(820.0, 520.0))?;
            brancher_l_atelier(p).ok_or_else(manque)
        }
        Epreuve::Ordinateur { complet: true, .. } => {
            poser(p, "PROCESSEUR", "cpu", Point::nouveau(600.0, 260.0))?;
            poser(p, "RAM16", "ram", Point::nouveau(820.0, 520.0))?;
            brancher_l_atelier(p).ok_or_else(manque)
        }
        _ => brancher_l_atelier(p).ok_or_else(manque),
    }
}

/// Pose une brique de la bibliothèque des solutions sous le nom `nom`, si l'atelier n'a pas
/// déjà un élément de ce nom.
fn poser(p: &mut Projet, modele: &str, nom: &str, pos: Point) -> Result<(), String> {
    if p.principal.nomme(nom).is_some() {
        return Ok(());
    }
    importer(p, modele)?;
    let mut b = p
        .modele(modele)
        .ok_or_else(|| tr!("la brique {modele} manque", modele = modele))?
        .instance();
    b.nom = nom.to_owned();
    p.principal.poser(b, pos);
    Ok(())
}

/// Le rang de l'entrée `nom` d'un élément.
fn entree(plan: &Document, e: Id, nom: &str) -> Option<Fil> {
    let i = plan
        .element(e)?
        .broches_e
        .iter()
        .position(|b| b.nom == nom)?;
    Some(Fil {
        puits: e,
        entree: i,
    })
}

/// La sortie `nom` d'un élément.
fn sortie(plan: &Document, e: Id, nom: &str) -> Option<Source> {
    let j = plan
        .element(e)?
        .broches_s
        .iter()
        .position(|b| b.nom == nom)?;
    Some(Source::nouvelle(e, j))
}

/// Un point à côté d'un élément.
fn pres_de(plan: &Document, e: Id, dx: f32, dy: f32) -> Point {
    let p = plan
        .element(e)
        .map_or(Point::nouveau(300.0, 300.0), |e| e.pos);
    Point::nouveau(p.x + dx, p.y + dy)
}

/// Branche l'atelier d'un ordinateur (processeur `cpu`, mémoires `rom` et `ram`, horloge,
/// bouton `raz`, clavier, écran) comme l'ordinateur de référence : trois portes de plus, un NON
/// pour `nraz`, un NON pour l'horloge inversée et un ET pour l'écriture à l'écran.
pub fn brancher_l_atelier(p: &mut Projet) -> Option<()> {
    for m in ["NON", "ET"] {
        importer(p, m).ok()?;
    }
    let non = p.modele("NON")?.instance();
    let et = p.modele("ET")?.instance();
    let plan = &mut p.principal;
    let id = |plan: &Document, nom: &str| plan.nomme(nom).map(|e| e.id);
    let (cpu, rom, ram) = (id(plan, "cpu")?, id(plan, "rom")?, id(plan, "ram")?);
    let (raz, horloge, clavier, ecran) = (
        id(plan, "raz")?,
        id(plan, "horloge")?,
        id(plan, "clavier")?,
        id(plan, "ecran")?,
    );
    let poser = |plan: &mut Document, mut e: Element, nom: &str, pos: Point| {
        e.nom = nom.to_owned();
        plan.poser(e, pos)
    };
    let pos_nraz = pres_de(plan, raz, 160.0, 0.0);
    let nraz = poser(plan, non.clone(), "nraz", pos_nraz);
    let pos_nclk = pres_de(plan, horloge, 160.0, 0.0);
    let nclk = poser(plan, non, "nclk", pos_nclk);
    let pos_ecrit = pres_de(plan, ecran, -200.0, 0.0);
    let ecrit = poser(plan, et, "ecrit", pos_ecrit);
    let largeur_d = plan.element(rom)?.broches_s.first()?.largeur;
    let bits = plan
        .element(ram)?
        .broches_e
        .iter()
        .find(|b| b.nom == "adresse")?
        .largeur;
    let pos_adr = pres_de(plan, ram, -200.0, 0.0);
    let adr = plan.poser(
        Element::decoupe("adr", largeur_d, &[(0, bits - 1)]),
        pos_adr,
    );
    let liens = [
        (Source::de(horloge), entree(plan, cpu, "clk")?),
        (Source::de(horloge), entree(plan, ram, "clk")?),
        (Source::de(horloge), entree(plan, nclk, "a")?),
        (Source::de(raz), entree(plan, nraz, "a")?),
        (sortie(plan, nraz, "s")?, entree(plan, cpu, "nraz")?),
        (sortie(plan, cpu, "pc")?, entree(plan, rom, "a")?),
        (sortie(plan, rom, "d")?, entree(plan, cpu, "instr")?),
        (
            sortie(plan, rom, "d")?,
            Fil {
                puits: adr,
                entree: 0,
            },
        ),
        (Source::de(adr), entree(plan, ram, "adresse")?),
        (sortie(plan, cpu, "a")?, entree(plan, ram, "d")?),
        (
            sortie(plan, cpu, "a")?,
            Fil {
                puits: ecran,
                entree: 0,
            },
        ),
        (
            sortie(plan, cpu, "ecrit_ram")?,
            entree(plan, ram, "ecrire")?,
        ),
        (sortie(plan, ram, "q")?, entree(plan, cpu, "lu")?),
        (Source::de(clavier), entree(plan, cpu, "clavier")?),
        (sortie(plan, cpu, "sortie")?, entree(plan, ecrit, "a")?),
        (sortie(plan, nclk, "s")?, entree(plan, ecrit, "b")?),
        (
            sortie(plan, ecrit, "s")?,
            Fil {
                puits: ecran,
                entree: 1,
            },
        ),
    ];
    for (source, fil) in liens {
        if !plan.relier(source, fil) {
            return None;
        }
    }
    Some(())
}

/// Relie deux ordinateurs A et B par une boîte aux lettres (un registre de 8 bits dont la
/// charge vient de raz inversé), comme le dit la consigne des grands projets à deux.
pub fn brancher_les_deux(p: &mut Projet) -> Option<()> {
    for m in ["REGISTRE8", "NON"] {
        importer(p, m).ok()?;
    }
    let registre = p.modele("REGISTRE8")?.instance();
    let non = p.modele("NON")?.instance();
    let plan = &mut p.principal;
    let id = |plan: &Document, nom: &str| plan.nomme(nom).map(|e| e.id);
    let (a, b, raz, clavier, ecran) = (
        id(plan, "a")?,
        id(plan, "b")?,
        id(plan, "raz")?,
        id(plan, "clavier")?,
        id(plan, "ecran")?,
    );
    let mut boite_e = registre;
    boite_e.nom = "boite".to_owned();
    let pos_boite = pres_de(plan, a, 360.0, 0.0);
    let boite = plan.poser(boite_e, pos_boite);
    let mut nraz_e = non;
    nraz_e.nom = "nraz".to_owned();
    let pos_nraz = pres_de(plan, raz, 160.0, 0.0);
    let nraz = plan.poser(nraz_e, pos_nraz);
    let liens = [
        (Source::de(raz), entree(plan, a, "raz")?),
        (Source::de(raz), entree(plan, b, "raz")?),
        (Source::de(raz), entree(plan, nraz, "a")?),
        (Source::de(clavier), entree(plan, a, "clavier")?),
        (sortie(plan, a, "ecran")?, entree(plan, boite, "d")?),
        (sortie(plan, a, "ecran_ecrit")?, entree(plan, boite, "clk")?),
        (sortie(plan, nraz, "s")?, entree(plan, boite, "charge")?),
        (sortie(plan, boite, "q")?, entree(plan, b, "clavier")?),
        (
            sortie(plan, b, "ecran")?,
            Fil {
                puits: ecran,
                entree: 0,
            },
        ),
        (
            sortie(plan, b, "ecran_ecrit")?,
            Fil {
                puits: ecran,
                entree: 1,
            },
        ),
    ];
    for (source, fil) in liens {
        if !plan.relier(source, fil) {
            return None;
        }
    }
    Some(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::economie::{cible_de_la_quete, de_la_quete, fournies, AJOUTS};
    use crate::quete::PARCOURS;

    /// Chaque quête a sa solution ; elle réussit la vérification de la quête, et son nombre de
    /// portes est exactement la cible affichée par la construction minimale.
    #[test]
    fn chaque_solution_reussit_sa_quete() {
        let ordinateurs = PARCOURS.iter().filter(|f| f.epreuve.ordinateur()).count();
        assert_eq!(ordinateurs, AJOUTS.len());
        let mut fautes = Vec::new();
        for f in PARCOURS {
            let p = match projet(f, "solution") {
                Ok(p) => p,
                Err(e) => {
                    fautes.push(format!("{} : {e}", f.cle));
                    continue;
                }
            };
            let mut q = Quete::de_fiche(f);
            let v = q.verifier(&p).clone();
            if !v.reussi {
                fautes.push(format!("{} : {}", f.cle, v.texte));
                continue;
            }
            let (cible, _) = cible_de_la_quete(f).unwrap();
            let e = de_la_quete(&q, &p, &p.principal).unwrap();
            if e.tiennes != cible.nand() || e.etoiles() != 3 {
                fautes.push(format!(
                    "{} : {} portes, cible {}",
                    f.cle,
                    e.tiennes,
                    cible.nand()
                ));
            }
            // Un ordinateur se juge sur ce qu'on ajoute à l'atelier : l'atelier fournit bien
            // `fournies` portes, et la solution en ajoute exactement la cible.
            if f.epreuve.ordinateur() {
                let atelier = Parcours::default().projet(f, "atelier").unwrap();
                let depart = atelier.nb_portes(&atelier.principal);
                let ajoutees = p.nb_portes(&p.principal) - depart;
                if depart != fournies(f) || ajoutees != cible.nand() {
                    fautes.push(format!(
                        "{} : l'atelier fournit {depart} portes ({} attendues), la solution en ajoute {ajoutees}",
                        f.cle,
                        fournies(f)
                    ));
                }
            }
        }
        assert!(fautes.is_empty(), "{}", fautes.join("\n"));
    }
}
