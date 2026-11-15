//! Les pièces ajoutées pour la 1.0 : bouton poussoir, afficheur 7 segments, description d'une
//! brique et « où est-elle utilisée ».
//!
//! Avec `CE_CAPTURES=/dossier`, le dernier test enregistre aussi des captures d'écran.

use ce_app::app::Application;
use ce_app::apparence::{Apparence, Surveillance};
use ce_app::document::{Element, Point};
use ce_app::quete::{Quete, PARCOURS};
use egui::Vec2;
use egui_kittest::kittest::Queryable;
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

#[test]
fn la_palette_cherche_parmi_mes_briques() {
    let mut b = banc(false);
    {
        let app = b.state_mut();
        for (nom, description) in [
            ("ADD1", "additionne deux bits"),
            ("MUX", "aiguille"),
            ("RÉGISTRE", "garde un octet"),
            ("COMPTEUR", "compte les fronts"),
            ("NONET", "une porte"),
        ] {
            let mut m = ce_app::brique::modele_du_plan(
                &ce_app::document::Document::demi_additionneur(),
                nom,
            );
            m.description = description.to_owned();
            app.atelier.projet.ajouter(m);
        }
        app.atelier.version += 1;
    }
    b.run_steps(10);
    for i in 0..5 {
        assert!(
            b.state().panneaux.repere("modele", i).is_some(),
            "brique {i}"
        );
    }
    assert!(b.state().panneaux.repere("filtre-briques", 0).is_some());
    // Sans accents ni majuscules, dans le nom ou la description.
    b.state_mut().panneaux.filtre_briques = "registre".to_owned();
    b.run_steps(5);
    let visibles: Vec<usize> = (0..5)
        .filter(|&i| b.state().panneaux.repere("modele", i).is_some())
        .collect();
    assert_eq!(visibles, [2]);
    b.state_mut().panneaux.filtre_briques = "OCTET".to_owned();
    b.run_steps(5);
    assert!(b.state().panneaux.repere("modele", 2).is_some());
    assert!(b.state().panneaux.repere("modele", 0).is_none());
}

#[test]
fn captures_machine_et_palette() {
    let Ok(dossier) = std::env::var("CE_CAPTURES") else {
        eprintln!("CE_CAPTURES n'est pas défini : captures ignorées.");
        return;
    };
    let mut b = banc(true);
    {
        let app = b.state_mut();
        let texte = "        LDI 7\n        STA 2\n        ADDI 5\n        STA 9\nboucle: ADDI 1\n        STA 15\n        JMP boucle\n";
        let projet = ce_app::exemples::ordinateur("Ordinateur", texte).expect("ordinateur");
        app.atelier = ce_app::etat::Atelier::avec(projet, &app.apparence);
        app.atelier.session.avancer_vite(128 * 40);
        let at = &mut app.atelier;
        app.fenetres.ouvrir_l_editeur(at);
    }
    b.run_steps(120);
    let image = b.render().expect("rendu");
    std::fs::create_dir_all(&dossier).expect("dossier de sortie");
    image
        .save(format!("{dossier}/p2-machine.png"))
        .expect("écriture de la capture");
}

#[test]
fn capture_depannage() {
    let Ok(dossier) = std::env::var("CE_CAPTURES") else {
        eprintln!("CE_CAPTURES n'est pas défini : captures ignorées.");
        return;
    };
    let mut b = banc(true);
    {
        let app = b.state_mut();
        let fiche = PARCOURS
            .iter()
            .find(|f| f.cle == "panne-additionneur-complet")
            .expect("quête");
        let apparence = app.apparence.clone();
        assert!(app.atelier.ouvrir_quete(fiche, "Dépannage", &apparence));
    }
    b.run_steps(120);
    let image = b.render().expect("rendu");
    std::fs::create_dir_all(&dossier).expect("dossier de sortie");
    image
        .save(format!("{dossier}/p3-depannage.png"))
        .expect("écriture de la capture");
}

#[test]
fn le_plan_s_exporte_en_image() {
    let racine = std::env::temp_dir().join(format!("ce-photo-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&racine);
    let app = Application::nouvelle(Apparence::default(), Surveillance::default())
        .avec_dossier(ce_app::sauvegarde::Dossier::dans(&racine));
    let mut b = Harness::builder()
        .with_size(Vec2::new(1280.0, 800.0))
        .with_pixels_per_point(1.0)
        .with_step_dt(1.0 / 60.0)
        .wgpu()
        .build_ui_state(|ui, app: &mut Application| app.image(ui), app);
    b.run_steps(120);
    let camera = b.state().plan.cam;
    b.state_mut().panneaux.photo_demandee = true;
    b.run_steps(20);
    assert_eq!(b.state().plan.cam, camera, "la vue revient comme avant");
    let fichier = racine.join("exports").join(format!(
        "{}.png",
        ce_app::export::identifiant(&b.state().atelier.projet.nom)
    ));
    let image = image::open(&fichier).expect("l'image est écrite");
    let (l, h) = (image.width(), image.height());
    assert!(l > 100 && h > 50 && l <= 1280 && h <= 800, "{l} × {h}");
    if let Ok(dossier) = std::env::var("CE_CAPTURES") {
        std::fs::create_dir_all(&dossier).expect("dossier de sortie");
        std::fs::copy(&fichier, format!("{dossier}/p3-photo.png")).expect("copie");
    }
    let _ = std::fs::remove_dir_all(&racine);
}

#[test]
fn le_banc_d_essai_s_ecrit_s_enregistre_et_se_joue() {
    let captures = std::env::var("CE_CAPTURES").ok();
    let mut b = banc(captures.is_some());
    b.key_press_modifiers(egui::Modifiers::COMMAND, egui::Key::T);
    b.run_steps(10);
    let texte = b
        .state()
        .fenetres
        .banc
        .as_ref()
        .expect("banc ouvert")
        .texte
        .clone();
    assert!(texte.contains("a b -> s c"), "{texte}");
    let version = b.state().atelier.version;
    let saisie = b.get_by_role(egui::accesskit::Role::MultilineTextInput);
    saisie.focus();
    b.run_steps(2);
    b.key_press_modifiers(egui::Modifiers::COMMAND, egui::Key::End);
    b.run_steps(2);
    b.get_by_role(egui::accesskit::Role::MultilineTextInput)
        .type_text("1 1 -> 1 0");
    b.run_steps(5);
    // Le banc est enregistré avec le projet, à la première frappe.
    let enregistre = b
        .state()
        .atelier
        .projet
        .bancs
        .get("")
        .cloned()
        .unwrap_or_default();
    assert!(enregistre.ends_with("1 1 -> 1 0"), "{enregistre}");
    assert!(b.state().atelier.version > version);
    b.get_by_label("Lancer le banc").click();
    b.run_steps(10);
    let vue = b.state().fenetres.banc.clone().expect("banc ouvert");
    let (resultat, _) = vue.resultat.expect("banc joué");
    let lignes = resultat.expect("le banc se lit");
    assert_eq!(lignes.len(), 2);
    assert!(lignes[0].juste());
    assert_eq!(
        lignes[1].ecarts,
        ["s : attendu 1, obtenu 0", "c : attendu 0, obtenu 1"]
    );
    if let Some(dossier) = captures {
        let image = b.render().expect("rendu");
        std::fs::create_dir_all(&dossier).expect("dossier de sortie");
        image
            .save(format!("{dossier}/p3-banc.png"))
            .expect("écriture de la capture");
    }
}
