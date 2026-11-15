//! La documentation (F2), par l'interface : elle s'ouvre partout, sur la page de l'élément
//! sélectionné, on y navigue par liens et « Retour », et une quête s'ouvre depuis une page.
//!
//! Avec `CE_CAPTURES=/dossier`, le dernier test enregistre aussi des captures d'écran (il faut
//! alors un pilote graphique, Vulkan logiciel « lavapipe » sous Linux par exemple).

use ce_app::app::{Application, Ecran};
use ce_app::apparence::{Apparence, Mode, Surveillance};
use ce_app::documentation::{self as doc, Page};
use ce_app::fenetre_doc::Documentation;
use ce_app::langue::{self, Langue};
use ce_app::panneaux::Ouverture;
use egui::{Key, Vec2};
use egui_kittest::kittest::Queryable;
use egui_kittest::Harness;

type Banc = Harness<'static, Application>;

fn banc(wgpu: bool, apparence: Apparence) -> Banc {
    let mut constructeur = Harness::builder()
        .with_size(Vec2::new(1440.0, 900.0))
        .with_pixels_per_point(if wgpu { 2.0 } else { 1.0 })
        .with_step_dt(1.0 / 60.0);
    if wgpu {
        constructeur = constructeur.wgpu();
    }
    let mut b = constructeur.build_ui_state(
        |ui, app: &mut Application| app.image(ui),
        Application::nouvelle(apparence, Surveillance::default()),
    );
    b.state_mut().ecran = Ecran::Accueil;
    b.run_steps(120);
    b
}

fn page(b: &Banc) -> Option<Page> {
    b.state()
        .fenetres
        .documentation
        .as_ref()
        .map(|d| d.page.clone())
}

#[test]
fn f2_ouvre_et_ferme_la_documentation_depuis_l_accueil() {
    langue::choisir_pour_ce_fil(Some(Langue::Francais));
    let mut b = banc(false, Apparence::default());
    assert_eq!(b.state().ecran, Ecran::Accueil);
    b.key_press(Key::F2);
    b.run_steps(10);
    assert_eq!(page(&b), Some(Page::Sommaire));
    b.key_press(Key::F2);
    b.run_steps(10);
    assert_eq!(page(&b), None);
    langue::choisir_pour_ce_fil(None);
}

#[test]
fn l_entree_du_menu_de_l_accueil_ouvre_la_documentation() {
    langue::choisir_pour_ce_fil(Some(Langue::Francais));
    let mut b = banc(false, Apparence::default());
    let r = b
        .state()
        .accueil
        .repere("documentation", 0)
        .expect("entrée du menu");
    b.hover_at(r.center());
    b.step();
    b.drag_at(r.center());
    b.step();
    b.drop_at(r.center());
    b.run_steps(10);
    assert_eq!(page(&b), Some(Page::Sommaire));
    langue::choisir_pour_ce_fil(None);
}

#[test]
fn f2_dans_l_atelier_ouvre_la_page_de_l_element_selectionne() {
    langue::choisir_pour_ce_fil(Some(Langue::Francais));
    let mut b = banc(false, Apparence::default());
    b.state_mut().ouvrir(Ouverture::Quete("et".to_owned()));
    b.run_steps(30);
    assert_eq!(b.state().ecran, Ecran::Atelier);
    // Une NAND posée : sa page de pièce de base.
    let nand = b
        .state()
        .atelier
        .doc
        .elements
        .iter()
        .find(|e| e.genre == ce_app::document::Genre::Nand)
        .map(|e| e.id);
    if let Some(id) = nand {
        b.state_mut().atelier.selection = Some(ce_app::etat::Selection::Elements(vec![id]));
        b.key_press(Key::F2);
        b.run_steps(10);
        assert_eq!(page(&b), Some(Page::Notion("nand")));
    }
    // Un interrupteur : sa page, la même fenêtre change de page.
    let entree = b
        .state()
        .atelier
        .doc
        .elements
        .iter()
        .find(|e| e.genre == ce_app::document::Genre::Entree)
        .map(|e| e.id)
        .expect("un interrupteur");
    b.state_mut().atelier.selection = Some(ce_app::etat::Selection::Elements(vec![entree]));
    b.key_press(Key::F2);
    b.run_steps(10);
    assert_eq!(page(&b), Some(Page::Notion("interrupteur")));
    langue::choisir_pour_ce_fil(None);
}

#[test]
fn on_navigue_par_liens_et_on_revient_en_arriere() {
    langue::choisir_pour_ce_fil(Some(Langue::Francais));
    let mut b = banc(false, Apparence::default());
    b.state_mut().fenetres.documentation =
        Some(Documentation::sur(Page::Brique("OU_X".to_owned())));
    b.run_steps(10);
    // Le lien « Utilisée par » vers l'équivalence.
    let titre = doc::titre(&Page::Brique("EQUIV".to_owned()));
    b.get_all_by_label(&titre)
        .next()
        .expect("lien")
        .scroll_to_me();
    b.run_steps(10);
    b.get_all_by_label(&titre).next().expect("lien").click();
    b.run_steps(10);
    assert_eq!(page(&b), Some(Page::Brique("EQUIV".to_owned())));
    b.get_by_label("‹").click();
    b.run_steps(10);
    assert_eq!(page(&b), Some(Page::Brique("OU_X".to_owned())));
    b.get_by_label("›").click();
    b.run_steps(10);
    assert_eq!(page(&b), Some(Page::Brique("EQUIV".to_owned())));
    langue::choisir_pour_ce_fil(None);
}

#[test]
fn une_quete_s_ouvre_depuis_sa_page() {
    langue::choisir_pour_ce_fil(Some(Langue::Francais));
    let mut b = banc(false, Apparence::default());
    b.state_mut().fenetres.documentation = Some(Documentation::sur(Page::Brique("MUX".to_owned())));
    b.run_steps(10);
    b.get_by_label("Ouvrir la quête « Le multiplexeur »")
        .scroll_to_me();
    b.run_steps(10);
    b.get_by_label("Ouvrir la quête « Le multiplexeur »")
        .click();
    b.run_steps(30);
    assert_eq!(b.state().ecran, Ecran::Atelier);
    assert_eq!(
        b.state().atelier.quete.as_ref().map(|q| q.cle),
        Some("multiplexeur")
    );
    langue::choisir_pour_ce_fil(None);
}

#[test]
fn la_recherche_du_sommaire_mene_a_une_page() {
    langue::choisir_pour_ce_fil(Some(Langue::Francais));
    let mut b = banc(false, Apparence::default());
    b.key_press(Key::F2);
    b.run_steps(5);
    b.state_mut()
        .fenetres
        .documentation
        .as_mut()
        .unwrap()
        .recherche = "bascule d".to_owned();
    b.run_steps(5);
    b.get_all_by_label("La bascule D")
        .next()
        .expect("trouvée")
        .click();
    b.run_steps(10);
    assert_eq!(page(&b), Some(Page::Brique("BASCULE_D".to_owned())));
    langue::choisir_pour_ce_fil(None);
}

#[test]
fn chaque_page_s_affiche_sans_paniquer_dans_les_deux_langues() {
    for l in [Langue::Francais, Langue::Anglais] {
        langue::choisir_pour_ce_fil(Some(l));
        let mut b = banc(false, Apparence::default());
        for p in std::iter::once(Page::Sommaire).chain(doc::toutes_les_pages()) {
            b.state_mut().fenetres.documentation = Some(Documentation::sur(p));
            b.run_steps(2);
        }
        langue::choisir_pour_ce_fil(None);
    }
}

/// Captures de la documentation, avec `CE_CAPTURES=/dossier`.
#[test]
fn captures_documentation() {
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
    langue::choisir_pour_ce_fil(Some(Langue::Francais));
    let mut b = banc(true, Apparence::default());
    b.key_press(Key::F2);
    b.run_steps(20);
    enregistrer(&mut b, "d1-sommaire");
    for (nom, p) in [
        ("d2-porte-nand", Page::Notion("nand")),
        ("d3-ou-exclusif", Page::Brique("OU_X".to_owned())),
        ("d4-additionneur-8-bits", Page::Brique("ADD8".to_owned())),
        ("d5-bascule-d", Page::Brique("BASCULE_D".to_owned())),
        ("d6-memoire-16-octets", Page::Brique("RAM16".to_owned())),
        ("d7-signaux", Page::Notion("signaux")),
        ("d8-ordinateur", Page::Notion("ordinateur")),
    ] {
        b.state_mut()
            .fenetres
            .documentation
            .as_mut()
            .unwrap()
            .aller(p);
        b.run_steps(10);
        enregistrer(&mut b, nom);
        // Plus bas dans la page d'une brique : son comportement.
        for titre in ["Sa table de vérité", "Son chronogramme"] {
            if let Some(n) = b.query_by_label(titre) {
                n.scroll_to_me();
                b.run_steps(10);
                enregistrer(&mut b, &format!("{nom}-suite"));
            }
        }
    }
    // Plus bas dans la page de la bascule : la construction et les quêtes.
    b.state_mut()
        .fenetres
        .documentation
        .as_mut()
        .unwrap()
        .aller(Page::Brique("COMPTEUR4".to_owned()));
    b.run_steps(10);
    b.get_by_label("Comment la construire").scroll_to_me();
    b.run_steps(10);
    enregistrer(&mut b, "d9-compteur-construction");
    // La recherche.
    b.state_mut()
        .fenetres
        .documentation
        .as_mut()
        .unwrap()
        .recherche = "retenue".to_owned();
    b.run_steps(10);
    enregistrer(&mut b, "d10-recherche");
    // Dans l'atelier, à côté du circuit, en thème clair.
    let clair = Apparence {
        mode: Mode::Clair,
        ..Apparence::default()
    };
    let mut b = banc(true, clair);
    b.state_mut()
        .ouvrir(Ouverture::Quete("multiplexeur".to_owned()));
    b.run_steps(60);
    b.state_mut().fenetres.documentation = Some(Documentation::sur(Page::Brique("MUX".to_owned())));
    b.run_steps(20);
    enregistrer(&mut b, "d11-atelier-clair");
    // En anglais.
    langue::choisir_pour_ce_fil(Some(Langue::Anglais));
    let mut b = banc(true, Apparence::default());
    b.state_mut().fenetres.documentation =
        Some(Documentation::sur(Page::Brique("BASCULE_D".to_owned())));
    b.run_steps(20);
    enregistrer(&mut b, "d12-anglais-bascule-d");
    b.state_mut()
        .fenetres
        .documentation
        .as_mut()
        .unwrap()
        .aller(Page::Notion("signaux"));
    b.run_steps(10);
    enregistrer(&mut b, "d13-anglais-signaux");
    langue::choisir_pour_ce_fil(None);
}
