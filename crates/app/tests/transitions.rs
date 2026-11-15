//! Les animations et les transitions, pilotées comme par une souris et un clavier, sans fenêtre.
//!
//! Ce qu'on vérifie ici : elles ne changent rien à la simulation, elles ne retiennent aucun
//! clic, leurs durées viennent du thème, et le réglage « Animations : Réduites » les coupe.

use std::path::{Path, PathBuf};

use ce_app::app::{Application, Ecran};
use ce_app::apparence::{Apparence, Surveillance};
use ce_app::brique::Projet;
use ce_app::document::{Document, Genre, Id, Source};
use ce_app::reglages::Reglages;
use ce_app::sauvegarde::Dossier;
use egui::{Pos2, Vec2};
use egui_kittest::Harness;

type Banc = Harness<'static, Application>;

fn banc_avec(app: Application) -> Banc {
    Harness::builder()
        .with_size(Vec2::new(1440.0, 900.0))
        .with_pixels_per_point(1.0)
        .with_step_dt(1.0 / 60.0)
        .build_ui_state(|ui, app: &mut Application| app.image(ui), app)
}

/// L'atelier sur le demi-additionneur, animations complètes ou réduites.
fn atelier(reduites: bool) -> Banc {
    let mut apparence = Apparence::default();
    apparence.mouvement.sobre = reduites;
    let mut b = banc_avec(Application::nouvelle(apparence, Surveillance::default()));
    b.run_steps(300);
    b
}

/// Un dossier de données neuf, avec un projet à reprendre : le demi-additionneur.
fn dossier(nom: &str) -> PathBuf {
    let racine = std::env::temp_dir().join(format!("ce-transitions-{nom}-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&racine);
    let d = Dossier::dans(&racine);
    let f = d
        .enregistrer(&Projet::nouveau(
            "Additionneur",
            Document::demi_additionneur(),
        ))
        .unwrap();
    d.noter_dernier(&f).unwrap();
    racine
}

/// L'application telle qu'au lancement, sur l'accueil, avec ce dossier et ces réglages.
fn lancer(racine: &Path, reglages: Option<Reglages>) -> Banc {
    let d = Dossier::dans(racine);
    if let Some(r) = reglages {
        d.enregistrer_reglages(&r).unwrap();
    }
    let app = Application::nouvelle(Apparence::default(), Surveillance::default())
        .avec_dossier(d)
        .au_lancement();
    let mut b = banc_avec(app);
    b.run_steps(120);
    b
}

fn repere(b: &Banc, nom: &str, rang: usize) -> Pos2 {
    let app = b.state();
    match app.ecran {
        Ecran::Accueil => app.accueil.repere(nom, rang),
        Ecran::Atelier => app.panneaux.repere(nom, rang),
    }
    .unwrap_or_else(|| panic!("la commande {nom} {rang} n'est pas affichée"))
    .center()
}

/// Un clic : trois images, pas une de plus.
fn cliquer(b: &mut Banc, p: Pos2) {
    b.hover_at(p);
    b.step();
    b.drag_at(p);
    b.step();
    b.drop_at(p);
    b.step();
}

fn double_cliquer(b: &mut Banc, p: Pos2) {
    b.hover_at(p);
    b.step();
    for _ in 0..2 {
        b.drag_at(p);
        b.step();
        b.drop_at(p);
        b.step();
    }
}

fn id(b: &Banc, nom: &str) -> Id {
    b.state()
        .atelier
        .doc
        .elements
        .iter()
        .find(|e| e.nom == nom)
        .unwrap_or_else(|| panic!("pas d'élément {nom}"))
        .id
}

fn centre(b: &Banc, nom: &str) -> Pos2 {
    b.state().plan.centre_ecran(&b.state().atelier, id(b, nom))
}

/// Tout ce que la simulation a produit : l'instant, le journal, et ce qu'on lit sur chaque
/// signal du plan affiché.
fn trace(b: &Banc) -> String {
    let at = &b.state().atelier;
    let lectures: Vec<String> = at
        .doc
        .prises()
        .map(|s| format!("{}={}", at.doc.nom_signal(s), at.lecture(s).texte()))
        .collect();
    format!(
        "t={} lieu={:?}\n{:?}\n{:?}\n{}",
        at.session.t,
        at.chemin,
        at.session.journal,
        at.quete.as_ref().map(|q| (q.vus.len(), q.reussie())),
        lectures.join(" ")
    )
}

/// Le même scénario, joué image par image : interrupteurs, pose, câblage, suppression, brique
/// regroupée puis ouverte et refermée, quête vérifiée. Renvoie la trace à chaque étape.
fn scenario(reduites: bool) -> Vec<String> {
    let mut b = atelier(reduites);
    let mut traces = vec![trace(&b)];
    let mut noter = |b: &mut Banc, images: usize| {
        b.run_steps(images);
        traces.push(trace(b));
    };

    // Les interrupteurs, à la souris.
    let p = centre(&b, "b");
    cliquer(&mut b, p);
    noter(&mut b, 7);
    noter(&mut b, 240);
    let p = centre(&b, "a");
    cliquer(&mut b, p);
    noter(&mut b, 33);
    noter(&mut b, 240);

    // La quête, vérifiée par son bouton : la fête ne touche à rien.
    let p = repere(&b, "verifier", 0);
    cliquer(&mut b, p);
    noter(&mut b, 11);
    noter(&mut b, 200);

    // Une porte posée, câblée, puis supprimée.
    let porte = {
        let app = b.state_mut();
        app.plan.poser_au_centre(
            &app.apparence,
            &mut app.atelier,
            ce_app::document::Element::nand("essai"),
        )
    };
    noter(&mut b, 5);
    let source = Source::de(id(&b, "b"));
    {
        let app = b.state_mut();
        app.atelier.doc.relier(
            source,
            ce_app::document::Fil {
                puits: porte,
                entree: 0,
            },
        );
        app.atelier.apres_modification(&app.apparence);
    }
    noter(&mut b, 13);
    noter(&mut b, 150);
    {
        let app = b.state_mut();
        app.atelier.selectionner(porte);
        app.atelier.supprimer_selection(&app.apparence);
    }
    noter(&mut b, 9);
    noter(&mut b, 240);

    // Les cinq portes deviennent une brique ; on y plonge d'un double-clic, on en remonte.
    let brique = {
        let app = b.state_mut();
        let portes: Vec<Id> = app.atelier.doc.portes().map(|e| e.id).collect();
        app.atelier.selectionner_tous(portes);
        app.atelier
            .regrouper_selection(&app.apparence)
            .expect("les cinq portes se regroupent")
    };
    noter(&mut b, 240);
    let p = b.state().plan.centre_ecran(&b.state().atelier, brique);
    double_cliquer(&mut b, p);
    noter(&mut b, 4);
    assert_eq!(b.state().atelier.chemin.len(), 1, "on est dans la brique");
    noter(&mut b, 120);
    b.key_press(egui::Key::Escape);
    noter(&mut b, 3);
    b.key_press(egui::Key::Escape);
    noter(&mut b, 6);
    assert!(b.state().atelier.chemin.is_empty(), "on est remonté");
    let p = centre(&b, "b");
    cliquer(&mut b, p);
    noter(&mut b, 17);
    noter(&mut b, 240);
    traces
}

#[test]
fn les_animations_ne_changent_rien_a_la_simulation() {
    let completes = scenario(false);
    let reduites = scenario(true);
    assert_eq!(completes.len(), reduites.len());
    for (i, (c, r)) in completes.iter().zip(&reduites).enumerate() {
        assert_eq!(
            c, r,
            "étape {i} : la simulation diffère selon les animations"
        );
    }
    // Le scénario a bien fait tourner la simulation.
    assert!(completes.last().unwrap().contains("t="));
    assert_ne!(completes.first(), completes.last());
}

#[test]
fn un_clic_pendant_le_fondu_d_ecran_est_pris_par_le_nouvel_ecran() {
    let racine = dossier("clic");
    let mut b = lancer(&racine, None);
    assert_eq!(b.state().ecran, Ecran::Accueil);
    let p = repere(&b, "continuer", 0);
    cliquer(&mut b, p);
    b.step();
    assert_eq!(b.state().ecran, Ecran::Atelier);
    assert!(b.state().en_transition(), "le fondu vers l'atelier joue");

    // En plein fondu, l'atelier répond déjà : l'image de l'accueil qui s'efface par-dessus ne
    // retient pas le clic.
    let avant = b.state().atelier.valeur(Source::de(id(&b, "b")));
    let p = centre(&b, "b");
    cliquer(&mut b, p);
    assert!(
        b.state().en_transition(),
        "le clic est tombé pendant le fondu"
    );
    let apres = b.state().atelier.valeur(Source::de(id(&b, "b")));
    assert_ne!(avant, apres, "l'interrupteur a basculé pendant le fondu");

    // Le fondu se termine tout seul, et plus rien ne demande à redessiner pour lui.
    b.run_steps(60);
    assert!(!b.state().en_transition());
    let _ = std::fs::remove_dir_all(&racine);
}

#[test]
fn la_duree_du_fondu_vient_du_theme() {
    let racine = dossier("duree");
    let mut b = lancer(&racine, None);
    b.state_mut().apparence.mouvement.ecran = 2.0;
    let p = repere(&b, "continuer", 0);
    cliquer(&mut b, p);
    b.run_steps(100);
    assert!(
        b.state().en_transition(),
        "à 1,7 s d'un fondu de 2 s, il joue encore"
    );
    b.run_steps(30);
    assert!(!b.state().en_transition(), "à 2,2 s, il est fini");

    // Une durée nulle dans le thème : pas de fondu du tout.
    b.state_mut().apparence.mouvement.ecran = 0.0;
    let p = repere(&b, "marque", 0);
    cliquer(&mut b, p);
    b.step();
    assert_eq!(b.state().ecran, Ecran::Accueil);
    assert!(!b.state().en_transition());
    let _ = std::fs::remove_dir_all(&racine);
}

#[test]
fn le_reglage_animations_reduites_coupe_les_transitions() {
    let racine = dossier("reduites");
    let reglages = Reglages {
        animations_sobres: Some(true),
        ..Reglages::default()
    };
    let mut b = lancer(&racine, Some(reglages));
    assert!(b.state().apparence.mouvement.sobre);

    // Changement de rubrique, puis d'écran : immédiats.
    let p = repere(&b, "menu", 1);
    cliquer(&mut b, p);
    b.step();
    assert!(!b.state().en_transition());
    let p = repere(&b, "menu", 0);
    cliquer(&mut b, p);
    b.step();
    let p = repere(&b, "continuer", 0);
    cliquer(&mut b, p);
    b.step();
    assert_eq!(b.state().ecran, Ecran::Atelier);
    assert!(!b.state().en_transition(), "pas de fondu d'écran");

    // Un panneau fermé libère sa place à l'image suivante, sans glisser.
    b.run_steps(120);
    let libre_avant = b.state().disposition.unwrap().libre;
    b.state_mut().atelier.volets.briques = false;
    b.step();
    let juste_apres = b.state().disposition.unwrap().libre;
    b.run_steps(60);
    let installe = b.state().disposition.unwrap().libre;
    assert_ne!(libre_avant, installe, "le plan a gagné la place du panneau");
    assert_eq!(juste_apres, installe, "sans animation : tout de suite");

    // Entrer dans une brique : pas de plongée.
    let brique = {
        let app = b.state_mut();
        let portes: Vec<Id> = app.atelier.doc.portes().map(|e| e.id).collect();
        app.atelier.selectionner_tous(portes);
        app.atelier
            .regrouper_selection(&app.apparence)
            .expect("les cinq portes se regroupent")
    };
    b.run_steps(60);
    let p = b.state().plan.centre_ecran(&b.state().atelier, brique);
    double_cliquer(&mut b, p);
    b.step();
    assert_eq!(b.state().atelier.chemin.len(), 1);
    assert!(!b.state().plan.en_plongee(), "pas de plongée");
    let _ = std::fs::remove_dir_all(&racine);
}

#[test]
fn on_plonge_dans_une_brique_et_on_en_remonte_sans_rien_bloquer() {
    let mut b = atelier(false);
    let brique = {
        let app = b.state_mut();
        let portes: Vec<Id> = app.atelier.doc.portes().map(|e| e.id).collect();
        app.atelier.selectionner_tous(portes);
        app.atelier
            .regrouper_selection(&app.apparence)
            .expect("les cinq portes se regroupent")
    };
    b.run_steps(120);
    let p = b.state().plan.centre_ecran(&b.state().atelier, brique);
    double_cliquer(&mut b, p);
    b.step();
    assert_eq!(
        b.state().atelier.chemin.len(),
        1,
        "le double-clic ouvre la brique"
    );
    assert!(b.state().plan.en_plongee(), "la plongée joue");

    // Pendant la plongée, le plan de la brique répond déjà : on y sélectionne une porte (les
    // messages de bienvenue, qui couvrent le haut du plan, sont écartés).
    b.state_mut().atelier.messages.clear();
    b.step();
    let (dedans, p) = {
        let app = b.state();
        let libre = app.disposition.unwrap().libre.shrink(30.0);
        app.atelier
            .doc
            .elements
            .iter()
            .filter(|e| e.genre == Genre::Nand)
            .map(|e| (e.id, app.plan.centre_ecran(&app.atelier, e.id)))
            .find(|(_, p)| libre.contains(*p) && !app.plan.obstacles.iter().any(|r| r.contains(*p)))
            .expect("une porte visible dans la brique")
    };
    cliquer(&mut b, p);
    assert!(
        b.state().plan.en_plongee(),
        "le clic est tombé pendant la plongée"
    );
    assert!(b.state().atelier.est_selectionne(dedans));

    b.run_steps(60);
    assert!(!b.state().plan.en_plongee(), "la plongée se termine");

    // Échap lâche la sélection, puis referme la brique : on remonte.
    b.key_press(egui::Key::Escape);
    b.step();
    b.key_press(egui::Key::Escape);
    b.run_steps(2);
    assert!(b.state().atelier.chemin.is_empty());
    assert!(b.state().plan.en_plongee(), "la remontée joue");
    b.run_steps(60);
    assert!(!b.state().plan.en_plongee());
}

#[test]
fn supprimer_et_cabler_s_animent_puis_le_plan_se_repose() {
    let mut b = atelier(false);
    let p = centre(&b, "g5");
    cliquer(&mut b, p);
    assert!(b.state().atelier.est_selectionne(id(&b, "g5")));
    let avant = b.state().atelier.doc.elements.len();
    b.key_press(egui::Key::Delete);
    b.run_steps(2);
    // La porte est partie du document tout de suite ; seule sa silhouette s'efface.
    assert_eq!(b.state().atelier.doc.elements.len(), avant - 1);
    assert!(b.state().plan.en_mouvement, "la disparition s'anime");
    b.key_press_modifiers(egui::Modifiers::COMMAND, egui::Key::Z);
    b.run_steps(2);
    assert_eq!(b.state().atelier.doc.elements.len(), avant);
    assert!(b.state().plan.en_mouvement, "le retour s'anime aussi");

    // Animations réduites : la même suppression ne laisse rien à animer, et une fois le
    // circuit stable plus rien ne demande à redessiner.
    let mut b = atelier(true);
    let p = centre(&b, "g5");
    cliquer(&mut b, p);
    b.key_press(egui::Key::Delete);
    b.run_steps(2);
    assert_eq!(b.state().atelier.doc.elements.len(), avant - 1);
    b.hover_at(Pos2::new(700.0, 10.0));
    b.run_steps(400);
    assert!(!b.state().plan.en_mouvement, "le plan est au repos");
    assert!(!b.state().anims.en_mouvement, "les panneaux aussi");
}
