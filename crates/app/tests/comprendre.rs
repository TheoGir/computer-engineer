//! Comprendre et partager un circuit, par l'interface : indices de la quête, analyse (Ctrl+E),
//! copier-coller (Ctrl+C, Ctrl+V), aide (lexique, recherche).
//!
//! Avec `CE_CAPTURES=/dossier`, le dernier test enregistre aussi des captures d'écran (il faut
//! alors un pilote graphique, Vulkan logiciel « lavapipe » sous Linux par exemple).

use ce_app::app::Application;
use ce_app::apparence::{Apparence, Surveillance};
use ce_app::fenetres::Rubrique;
use egui::{Key, Modifiers, Pos2, Vec2};
use egui_kittest::Harness;

type Banc = Harness<'static, Application>;

fn banc(wgpu: bool) -> Banc {
    let mut constructeur = Harness::builder()
        .with_size(Vec2::new(1440.0, 900.0))
        .with_pixels_per_point(if wgpu { 2.0 } else { 1.0 })
        .with_step_dt(1.0 / 60.0);
    if wgpu {
        constructeur = constructeur.wgpu();
    }
    let mut b = constructeur.build_ui_state(
        |ui, app: &mut Application| app.image(ui),
        Application::nouvelle(Apparence::default(), Surveillance::default()),
    );
    b.run_steps(300);
    b
}

fn cliquer(b: &mut Banc, p: Pos2) {
    b.hover_at(p);
    b.step();
    b.drag_at(p);
    b.step();
    b.drop_at(p);
    b.run_steps(20);
}

fn id(b: &Banc, nom: &str) -> ce_app::document::Id {
    b.state()
        .atelier
        .doc
        .elements
        .iter()
        .find(|e| e.nom == nom)
        .expect("élément")
        .id
}

fn raccourci(b: &mut Banc, touche: Key) {
    b.key_press_modifiers(Modifiers::COMMAND, touche);
    b.run_steps(10);
}

#[test]
fn les_indices_se_revelent_un_par_un_dans_le_panneau_de_la_quete() {
    let mut b = banc(false);
    let avant = b
        .state()
        .panneaux
        .repere("indice", 0)
        .expect("bouton d'indice");
    cliquer(&mut b, avant.center());
    // Un indice révélé : le bouton descend sous lui, pour le suivant.
    let apres = b
        .state()
        .panneaux
        .repere("indice", 0)
        .expect("indice suivant");
    assert!(apres.top() > avant.top() + 20.0, "{avant:?} → {apres:?}");
    cliquer(&mut b, apres.center());
    let encore = b
        .state()
        .panneaux
        .repere("indice", 0)
        .expect("dernier indice");
    cliquer(&mut b, encore.center());
    // Le demi-additionneur a trois indices : une fois tous montrés, le bouton s'en va.
    assert!(b.state().panneaux.repere("indice", 0).is_none());
}

#[test]
fn ctrl_e_analyse_le_plan_et_ctrl_c_ctrl_v_colle_une_copie() {
    let mut b = banc(false);
    raccourci(&mut b, Key::E);
    let analyse = b
        .state()
        .fenetres
        .analyse
        .as_ref()
        .expect("Ctrl+E ouvre l'analyse");
    let an = analyse
        .resultat
        .as_ref()
        .expect("le demi-additionneur s'analyse");
    assert_eq!(an.lignes.len(), 4);
    raccourci(&mut b, Key::E);
    assert!(b.state().fenetres.analyse.is_none(), "Ctrl+E la referme");

    let n = b.state().atelier.doc.elements.len();
    let (g1, g2) = (id(&b, "g1"), id(&b, "g2"));
    b.state_mut().atelier.selectionner_tous(vec![g1, g2]);
    raccourci(&mut b, Key::C);
    assert!(b.state().atelier.presse_papiers.is_some());
    raccourci(&mut b, Key::V);
    assert_eq!(b.state().atelier.doc.elements.len(), n + 2);
    // Ctrl+V par le presse-papiers du système : le texte d'un Ctrl+C se colle aussi.
    let texte = b
        .state()
        .atelier
        .presse_papiers
        .as_ref()
        .unwrap()
        .en_texte();
    b.event(egui::Event::Paste(texte));
    b.run_steps(10);
    assert_eq!(b.state().atelier.doc.elements.len(), n + 4);
}

#[test]
fn captures_comprendre() {
    let Ok(dossier) = std::env::var("CE_CAPTURES") else {
        eprintln!("CE_CAPTURES n'est pas défini : captures ignorées.");
        return;
    };
    let enregistrer = |b: &mut Banc, nom: &str| {
        let image = b.render().expect("rendu");
        std::fs::create_dir_all(&dossier).expect("dossier de sortie");
        image
            .save(format!("{dossier}/{nom}.png"))
            .expect("écriture de la capture");
    };
    let mut b = banc(true);
    // Les trois indices du demi-additionneur.
    for _ in 0..3 {
        let r = b.state().panneaux.repere("indice", 0).expect("indice");
        cliquer(&mut b, r.center());
    }
    b.run_steps(60);
    enregistrer(&mut b, "i1-indices");

    // L'analyse du plan.
    raccourci(&mut b, Key::E);
    b.run_steps(60);
    enregistrer(&mut b, "i2-analyse");
    raccourci(&mut b, Key::E);

    // Copier-coller : deux portes, collées un peu plus loin.
    let (g1, g2) = (id(&b, "g1"), id(&b, "g2"));
    b.state_mut().atelier.selectionner_tous(vec![g1, g2]);
    raccourci(&mut b, Key::C);
    raccourci(&mut b, Key::V);
    b.run_steps(400);
    enregistrer(&mut b, "i3-copier-coller");
    b.state_mut().atelier.annuler();

    // L'aide : l'algèbre de Boole, le lexique, puis une recherche.
    b.state_mut().fenetres.aide = Some(Rubrique::Algebre);
    b.run_steps(30);
    enregistrer(&mut b, "i4-algebre");
    b.state_mut().fenetres.aide = Some(Rubrique::Lexique);
    b.run_steps(30);
    enregistrer(&mut b, "i5-lexique");
    b.state_mut().fenetres.recherche = "bascule".to_owned();
    b.run_steps(30);
    enregistrer(&mut b, "i6-recherche");
}
