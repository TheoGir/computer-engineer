//! Les pièces ajoutées pour la 1.0 : bouton poussoir, afficheur 7 segments, description d'une
//! brique et « où est-elle utilisée ».
//!
//! Avec `CE_CAPTURES=/dossier`, le dernier test enregistre aussi des captures d'écran.

use ce_app::app::Application;
use ce_app::apparence::{Apparence, Surveillance};
use ce_app::document::{Element, Point};
use ce_app::quete::{Quete, PARCOURS};
use egui::Vec2;
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

fn valeur(b: &Banc, id: u32) -> u64 {
    b.state()
        .atelier
        .session
        .entrees
        .get(&id)
        .copied()
        .unwrap_or(0)
}

#[test]
fn le_poussoir_vaut_un_tant_qu_on_appuie() {
    let mut b = banc(false);
    let id = {
        let app = b.state_mut();
        let id = app
            .atelier
            .poser(Element::poussoir("p"), Point::nouveau(300.0, 470.0));
        app.atelier.apres_modification(&app.apparence);
        id
    };
    b.run_steps(30);
    let p = b.state().plan.cam.ecran(Point::nouveau(300.0, 470.0));
    b.hover_at(p);
    b.step();
    b.drag_at(p);
    b.run_steps(5);
    assert_eq!(valeur(&b, id), 1, "enfoncé : 1");
    b.drop_at(p);
    b.run_steps(5);
    assert_eq!(valeur(&b, id), 0, "lâché : 0, sans rester basculé");
}

#[test]
fn la_quete_des_sept_segments_montre_un_afficheur() {
    let fiche = PARCOURS
        .iter()
        .find(|f| f.cle == "sept-segments")
        .expect("quête");
    let plan = Quete::de_fiche(fiche).plan_de_depart();
    let seg = plan
        .elements
        .iter()
        .find(|e| e.nom == "seg")
        .expect("lampe seg");
    assert!(seg.est_afficheur());
    assert_eq!(seg.largeur(), 72.0);
}

#[test]
fn une_brique_se_decrit_et_dit_ou_elle_est_posee() {
    let mut b = banc(false);
    let app = b.state_mut();
    let ids: Vec<_> = app
        .atelier
        .doc
        .elements
        .iter()
        .filter(|e| e.nom.starts_with('g'))
        .map(|e| e.id)
        .take(2)
        .collect();
    app.atelier.selectionner_tous(ids);
    assert!(app.atelier.regrouper_selection(&app.apparence).is_some());
    let nom = app.atelier.projet.modeles[0].nom.clone();
    assert_eq!(app.atelier.usages_du_modele(&nom), "plan principal (1)");
    app.atelier.decrire_modele(&nom, "  deux portes  ");
    assert_eq!(app.atelier.projet.modeles[0].description, "deux portes");
    app.atelier.decrire_modele(&nom, "—");
    assert_eq!(app.atelier.projet.modeles[0].description, "");
}

#[test]
fn captures_pieces() {
    let Ok(dossier) = std::env::var("CE_CAPTURES") else {
        eprintln!("CE_CAPTURES n'est pas défini : captures ignorées.");
        return;
    };
    let mut b = banc(true);
    {
        let app = b.state_mut();
        let p = app
            .atelier
            .poser(Element::poussoir("ok"), Point::nouveau(300.0, 520.0));
        let n = app
            .atelier
            .poser(Element::entree("n", 7), Point::nouveau(300.0, 650.0));
        let s = app.atelier.poser(
            Element::afficheur("chiffre", ""),
            Point::nouveau(560.0, 650.0),
        );
        assert!(app.atelier.doc.relier(
            ce_app::document::Source::de(n),
            ce_app::document::Fil {
                puits: s,
                entree: 0
            },
        ));
        app.atelier.apres_modification(&app.apparence);
        // « 3 » : a, b, c, d et g.
        app.atelier.session.regler(&app.atelier.doc, n, 0b100_1111);
        app.atelier.appuyer(p, true);
    }
    b.run_steps(30);
    // Le plan se recadre sur tout ce qui est posé (touche F).
    b.state_mut().atelier.selectionner_tous(Vec::new());
    b.key_press(egui::Key::F);
    b.run_steps(200);
    let image = b.render().expect("rendu");
    std::fs::create_dir_all(&dossier).expect("dossier de sortie");
    image
        .save(format!("{dossier}/p1-poussoir-afficheur.png"))
        .expect("écriture de la capture");
}
