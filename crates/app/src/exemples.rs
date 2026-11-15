//! Des projets tout faits, à ouvrir pour les regarder tourner et les démonter.

use ce_circuit::ordinateur::{self, Reglages};

use crate::brique::Projet;
use crate::document::{Document, Element, Fil, Id, Point, Source};
use crate::importer::importer;
use crate::quete::Atelier;

/// Nom de la brique qui contient tout l'ordinateur.
pub const ORDINATEUR: &str = "ORDINATEUR";

/// Un projet tout fait, proposé dans la liste des projets.
#[derive(Clone, Copy, Debug)]
pub struct Exemple {
    /// Nom du projet une fois ouvert.
    pub nom: &'static str,
    /// Ce qu'il montre, en quelques mots.
    pub resume: &'static str,
    /// Le programme chargé dans la mémoire de l'ordinateur, en assembleur.
    pub programme: &'static str,
    /// Ce qu'on dit à l'ouverture.
    pub accueil: &'static str,
}

/// Les exemples proposés.
pub const EXEMPLES: [Exemple; 2] = [
    Exemple {
        nom: "Ordinateur miniature",
        resume: "exemple : il multiplie deux chiffres",
        programme: ordinateur::programmes::CALCULATRICE,
        accueil: "L'ordinateur miniature tourne : 3 401 portes NAND. Clique sur le clavier puis tape deux chiffres, il affiche leur produit. Double-clique sur une brique pour l'ouvrir, Ctrl+P pour changer son programme.",
    },
    Exemple {
        nom: "Ordinateur en majuscules",
        resume: "exemple : il réécrit en majuscules",
        programme: ordinateur::programmes::MAJUSCULES,
        accueil: "Le même ordinateur, avec un autre programme : clique sur le clavier puis tape du texte, il le réécrit en majuscules.",
    },
];

/// Note dans la brique de la mémoire morte le programme dont elle est faite.
fn garder_le_programme(projet: &mut Projet, programme: &str) {
    if let Some(i) = projet.index(crate::programme::MEMOIRE_MORTE) {
        projet.modeles[i].programme = Some(programme.to_owned());
    }
}

/// L'ordinateur miniature, programmé avec le texte assembleur `programme`.
///
/// Le projet contient toutes ses briques, jusqu'aux portes NAND, et un plan principal où
/// l'ordinateur est branché sur un bouton de démarrage, un clavier, un écran et deux lampes.
pub fn ordinateur(nom: &str, programme: &str) -> Result<Projet, String> {
    let code = ordinateur::assembler(programme).map_err(|e| e.to_string())?;
    let reglages = Reglages::default();
    let bib = ordinateur::bibliotheque(&code, reglages).map_err(|e| e.to_string())?;
    let mut projet = Projet::nouveau(nom, Document::nouveau());
    for m in importer(&bib, ORDINATEUR)? {
        projet.ajouter(m);
    }
    garder_le_programme(&mut projet, programme);
    let modele = projet
        .modele(ORDINATEUR)
        .ok_or(tr!("la brique de l'ordinateur manque"))?;

    let mut plan = Document::nouveau();
    let mut brique = modele.instance();
    brique.nom = "pc".to_owned();
    let (entrees, sorties) = (brique.broches_e.clone(), brique.broches_s.clone());
    let pc = plan.poser(brique, Point::nouveau(520.0, 300.0));
    let entree = |nom: &str| entrees.iter().position(|b| b.nom == nom);
    let sortie = |nom: &str| sorties.iter().position(|b| b.nom == nom);
    let brancher = |plan: &mut Document, source: Source, puits: Id, entree: Option<usize>| {
        entree.is_some_and(|entree| plan.relier(source, Fil { puits, entree }))
    };

    // Le bouton de démarrage tient `raz` à 1 le temps d'un front montant de l'horloge.
    let raz = plan.poser(
        Element::impulsion("raz", reglages.demi_periode * 5 / 2),
        Point::nouveau(200.0, 250.0),
    );
    let clavier = plan.poser(Element::clavier("clavier"), Point::nouveau(160.0, 370.0));
    let ecran = plan.poser(Element::ecran("ecran"), Point::nouveau(940.0, 200.0));
    let arret = plan.poser(
        Element::sortie("arret", "arrêt", 1),
        Point::nouveau(820.0, 380.0),
    );
    let compteur = plan.poser(
        Element::sortie("pc", "compteur de programme", 8),
        Point::nouveau(820.0, 470.0),
    );
    let mut ok = brancher(&mut plan, Source::de(raz), pc, entree("raz"));
    ok &= brancher(&mut plan, Source::de(clavier), pc, entree("clavier"));
    for (nom, puits, broche) in [
        ("ecran", ecran, 0),
        ("ecran_ecrit", ecran, 1),
        ("arret", arret, 0),
        ("pc", compteur, 0),
    ] {
        ok &= sortie(nom)
            .is_some_and(|j| brancher(&mut plan, Source::nouvelle(pc, j), puits, Some(broche)));
    }
    if !ok {
        return Err(tr!("les broches de l'ordinateur ne sont pas celles attendues").to_owned());
    }
    projet.principal = plan;
    Ok(projet)
}

/// L'atelier des quêtes d'ordinateur : la mémoire morte (programmée avec `programme`), un
/// bouton de démarrage, une horloge, un clavier et un écran, et, si `fourni`, le processeur
/// et la mémoire vive. Rien n'est branché : c'est au joueur de le faire. Les briques NON et
/// ET sont dans la palette.
pub fn atelier_de_l_ordinateur(nom: &str, programme: &str, fourni: bool) -> Result<Projet, String> {
    let code = ordinateur::assembler(programme).map_err(|e| e.to_string())?;
    let reglages = Reglages::default();
    let bib = ordinateur::bibliotheque(&code, reglages).map_err(|e| e.to_string())?;
    let mut projet = Projet::nouveau(nom, Document::nouveau());
    let racines: &[&str] = if fourni {
        &["NON", "ET", "PROCESSEUR", "MEMOIRE_MORTE", "RAM16"]
    } else {
        &["NON", "ET", "MEMOIRE_MORTE"]
    };
    for racine in racines {
        for m in importer(&bib, racine)? {
            if projet.modele(&m.nom).is_none() {
                projet.ajouter(m);
            }
        }
    }
    garder_le_programme(&mut projet, programme);
    let mut plan = Document::nouveau();
    for (modele, nom, x, y) in [
        ("PROCESSEUR", "cpu", 560.0, 300.0),
        ("MEMOIRE_MORTE", "rom", 300.0, 520.0),
        ("RAM16", "ram", 820.0, 520.0),
    ] {
        if !fourni && modele != "MEMOIRE_MORTE" {
            continue;
        }
        let mut b = projet
            .modele(modele)
            .ok_or_else(|| tr!("la brique {modele} manque", modele = modele))?
            .instance();
        b.nom = nom.to_owned();
        plan.poser(b, Point::nouveau(x, y));
    }
    plan.poser(
        Element::impulsion("raz", reglages.demi_periode * 5 / 2),
        Point::nouveau(120.0, 160.0),
    );
    plan.poser(
        Element::horloge("horloge", reglages.demi_periode),
        Point::nouveau(120.0, 280.0),
    );
    plan.poser(Element::clavier("clavier"), Point::nouveau(120.0, 400.0));
    plan.poser(Element::ecran("ecran"), Point::nouveau(1080.0, 240.0));
    projet.principal = plan;
    Ok(projet)
}

/// L'atelier d'un grand projet. Pour deux ordinateurs : `a` (son programme lit le clavier)
/// et `b` (son programme écrit à l'écran), chacun avec sa propre mémoire morte, un bouton de
/// démarrage, un clavier et un écran, rien de branché. Pour la grande mémoire : le processeur,
/// la mémoire morte, une horloge, un clavier et un écran ; la mémoire vive de 256 octets
/// vient des briques gagnées.
pub fn atelier_du_grand_projet(nom: &str, atelier: Atelier) -> Result<Projet, String> {
    let reglages = Reglages::default();
    let assembler = |programme: &str| ordinateur::assembler(programme).map_err(|e| e.to_string());
    let mut projet = Projet::nouveau(nom, Document::nouveau());
    let importer_de = |projet: &mut Projet, bib: &ce_circuit::Bibliotheque, racines: &[&str]| {
        for racine in racines {
            for m in importer(bib, racine)? {
                if projet.modele(&m.nom).is_none() {
                    projet.ajouter(m);
                }
            }
        }
        Ok::<(), String>(())
    };
    let mut plan = Document::nouveau();
    let poser = |projet: &Projet, plan: &mut Document, modele: &str, nom: &str, x, y| {
        let mut b = projet
            .modele(modele)
            .ok_or_else(|| tr!("la brique {modele} manque", modele = modele))?
            .instance();
        b.nom = nom.to_owned();
        plan.poser(b, Point::nouveau(x, y));
        Ok::<(), String>(())
    };
    match atelier {
        Atelier::Deux { envoi, reception } => {
            for (programme, lettre) in [(envoi, "A"), (reception, "B")] {
                let bib = ordinateur::materiel::bibliotheque_nommee(
                    &assembler(programme)?,
                    reglages,
                    lettre,
                )
                .map_err(|e| e.to_string())?;
                importer_de(
                    &mut projet,
                    &bib,
                    &["NON", "ET", &format!("ORDINATEUR_{lettre}")],
                )?;
            }
            poser(&projet, &mut plan, "ORDINATEUR_A", "a", 420.0, 300.0)?;
            poser(&projet, &mut plan, "ORDINATEUR_B", "b", 940.0, 300.0)?;
            plan.poser(
                Element::impulsion("raz", reglages.demi_periode * 5 / 2),
                Point::nouveau(120.0, 200.0),
            );
            plan.poser(Element::clavier("clavier"), Point::nouveau(120.0, 380.0));
            plan.poser(Element::ecran("ecran"), Point::nouveau(1300.0, 260.0));
        }
        Atelier::GrandeMemoire { programme } => {
            let grande = Reglages {
                bits_ram: 8,
                ..reglages
            };
            let bib = ordinateur::bibliotheque(&assembler(programme)?, grande)
                .map_err(|e| e.to_string())?;
            importer_de(
                &mut projet,
                &bib,
                &["NON", "ET", "PROCESSEUR", "MEMOIRE_MORTE"],
            )?;
            poser(&projet, &mut plan, "PROCESSEUR", "cpu", 560.0, 300.0)?;
            poser(&projet, &mut plan, "MEMOIRE_MORTE", "rom", 300.0, 520.0)?;
            plan.poser(
                Element::impulsion("raz", reglages.demi_periode * 5 / 2),
                Point::nouveau(120.0, 160.0),
            );
            plan.poser(
                Element::horloge("horloge", reglages.demi_periode),
                Point::nouveau(120.0, 280.0),
            );
            plan.poser(Element::clavier("clavier"), Point::nouveau(120.0, 400.0));
            plan.poser(Element::ecran("ecran"), Point::nouveau(1080.0, 240.0));
        }
    }
    projet.principal = plan;
    Ok(projet)
}

#[cfg(test)]
pub(crate) mod tests {
    use super::*;
    use crate::document::Genre;
    use crate::simulation::Session;
    use ce_circuit::ordinateur::{programmes, Machine};

    fn element(p: &Projet, genre: Genre) -> Id {
        p.principal
            .elements
            .iter()
            .find(|e| e.genre == genre)
            .unwrap()
            .id
    }

    #[test]
    fn l_ordinateur_importe_calcule_six_fois_sept_avec_son_clavier_et_son_ecran() {
        let p = ordinateur("essai", programmes::CALCULATRICE).unwrap();
        let (clavier, ecran) = (element(&p, Genre::Clavier), element(&p, Genre::Ecran));
        let mut s = Session::nouvelle(&p.principal, &p, &[]);
        assert_eq!(s.erreur(), None);
        // Autant de portes que la machine d'origine : l'import n'a rien ajouté ni perdu.
        let code = ordinateur::assembler(programmes::CALCULATRICE).unwrap();
        let machine = Machine::nouvelle(&code, Reglages::default()).unwrap();
        assert_eq!(s.nb_portes(), machine.nb_portes());
        assert_eq!(p.nb_portes(&p.principal), machine.nb_portes() as u64);
        assert!(p.profondeur(&p.principal) >= 5);
        s.taper(clavier, b'6');
        s.taper(clavier, b'7');
        s.avancer_vite(128 * 400);
        assert_eq!(s.ecran(ecran), "6*7=42");
        assert!(
            !s.stable(),
            "une horloge bat : le circuit n'est jamais au repos"
        );
        // La remise à zéro efface l'écran et rejoue l'impulsion de démarrage.
        s.remettre_a_zero(&p.principal, &p);
        assert_eq!(s.ecran(ecran), "");
        s.taper(clavier, b'9');
        s.taper(clavier, b'9');
        s.avancer_vite(128 * 600);
        assert_eq!(s.ecran(ecran), "9*9=81");
    }

    #[test]
    fn pas_a_pas_ou_d_un_coup_l_ecran_affiche_la_meme_chose() {
        let p = ordinateur("essai", &programmes::afficher("NAND")).unwrap();
        let ecran = element(&p, Genre::Ecran);
        let mut vite = Session::nouvelle(&p.principal, &p, &[]);
        vite.avancer_vite(128 * 14);
        let mut lent = Session::nouvelle(&p.principal, &p, &[]);
        for _ in 0..128 * 14 {
            lent.pas(&p.principal);
        }
        assert_eq!(vite.ecran(ecran), "NAND");
        assert_eq!(lent.ecran(ecran), "NAND");
        let lampe = |s: &Session, nom: &str| {
            let id = p.principal.nomme(nom).unwrap().id;
            s.lecture_lue(
                &p.principal,
                Fil {
                    puits: id,
                    entree: 0,
                },
            )
        };
        assert_eq!(lampe(&vite, "pc"), lampe(&lent, "pc"));
        assert_eq!(lampe(&vite, "arret").nombre(), Some(1));
    }

    /// Branche l'atelier de la dernière quête comme l'ordinateur de référence.
    pub(crate) fn brancher_l_atelier(p: &mut Projet) {
        crate::solution::brancher_l_atelier(p).expect("l'atelier se branche");
    }

    #[test]
    fn l_atelier_de_l_ordinateur_une_fois_branche_calcule() {
        let mut p =
            atelier_de_l_ordinateur("essai", ordinateur::programmes::CALCULATRICE, true).unwrap();
        let mut s = Session::nouvelle(&p.principal, &p, &[]);
        assert_eq!(s.erreur(), None);
        s.avancer_vite(128 * 10);
        assert_eq!(s.ecran(element(&p, Genre::Ecran)), "", "rien n'est branché");
        brancher_l_atelier(&mut p);
        let mut s = Session::nouvelle(&p.principal, &p, &[]);
        assert_eq!(s.erreur(), None);
        assert_eq!(
            s.nb_portes(),
            3401,
            "autant de portes que l'ordinateur d'exemple"
        );
        s.taper(element(&p, Genre::Clavier), b'6');
        s.taper(element(&p, Genre::Clavier), b'7');
        s.avancer_vite(128 * 400);
        assert_eq!(s.ecran(element(&p, Genre::Ecran)), "6*7=42");
    }
}
