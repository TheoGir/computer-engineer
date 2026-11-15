//! Tests de l'interface, pilotée comme par une souris et un clavier, sans fenêtre ni carte
//! graphique : on envoie de vrais événements à egui et on regarde l'état de l'application.

use ce_app::app::Application;
use ce_app::apparence::{Apparence, Surveillance};
use ce_app::brique::Projet;
use ce_app::document::{Document, Element, Fil, Genre, Id, Point, Source};
use ce_app::etat::{Etat, Selection};
use ce_app::panneaux::{Action, Ouverture};
use ce_app::plan::{Geste, Port, Survol};
use ce_app::sauvegarde::Dossier;
use egui::{Pos2, Vec2};
use egui_kittest::kittest::Queryable;
use egui_kittest::Harness;

type Banc = Harness<'static, Application>;

fn banc_avec(app: Application) -> Banc {
    let mut b = Harness::builder()
        .with_size(Vec2::new(1440.0, 900.0))
        .with_pixels_per_point(1.0)
        .with_step_dt(1.0 / 60.0)
        .build_ui_state(|ui, app: &mut Application| app.image(ui), app);
    // Le temps que les panneaux se mesurent, que le plan se cadre et que le circuit se stabilise.
    b.run_steps(300);
    b
}

fn banc() -> Banc {
    banc_avec(Application::nouvelle(
        Apparence::default(),
        Surveillance::default(),
    ))
}

/// Le nombre de cas de la quête déjà validés.
fn vus(b: &Banc) -> usize {
    b.state().atelier.quete.as_ref().map_or(0, |q| q.vus.len())
}

/// Le centre d'une commande des panneaux, retrouvée par son nom.
fn repere(b: &Banc, nom: &str, rang: usize) -> Pos2 {
    b.state()
        .panneaux
        .repere(nom, rang)
        .unwrap_or_else(|| panic!("la commande {nom} {rang} n'est pas affichée"))
        .center()
}

/// Fait défiler le panneau de la quête jusqu'à montrer une commande, puis renvoie son centre.
fn montrer_dans_la_quete(b: &mut Banc, nom: &str, rang: usize) -> Pos2 {
    for _ in 0..40 {
        if b.state().panneaux.repere(nom, rang).is_some() {
            break;
        }
        let zone = b.state().disposition.unwrap().quete;
        molette(b, zone.center(), -2.0);
    }
    repere(b, nom, rang)
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

fn majuscule(b: &mut Banc, enfoncee: bool) {
    b.event(egui::Event::ModifiersChanged(if enfoncee {
        egui::Modifiers::SHIFT
    } else {
        egui::Modifiers::NONE
    }));
    b.step();
}

fn taper(b: &mut Banc, texte: &str) {
    for c in texte.chars() {
        b.event(egui::Event::Text(c.to_string()));
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
    let app = b.state();
    app.plan.centre_ecran(&app.atelier, id(b, nom))
}

fn port(b: &Banc, port: Port) -> Pos2 {
    let app = b.state();
    app.plan.port_ecran(&app.atelier, port)
}

fn position(b: &Banc, nom: &str) -> Point {
    b.state().atelier.doc.element(id(b, nom)).unwrap().pos
}

fn cliquer(b: &mut Banc, p: Pos2) {
    b.hover_at(p);
    b.step();
    b.drag_at(p);
    b.step();
    b.drop_at(p);
    b.step();
}

/// Appuie en `de`, glisse jusqu'à `vers` en plusieurs images, sans relâcher.
fn tirer(b: &mut Banc, de: Pos2, vers: Pos2) {
    b.hover_at(de);
    b.step();
    b.drag_at(de);
    b.step();
    for i in 1..=10 {
        b.hover_at(de + (vers - de) * (i as f32 / 10.0));
        b.step();
    }
}

fn glisser(b: &mut Banc, de: Pos2, vers: Pos2) {
    tirer(b, de, vers);
    b.drop_at(vers);
    b.step();
}

fn molette(b: &mut Banc, p: Pos2, crans: f32) {
    b.hover_at(p);
    b.step();
    b.event(egui::Event::MouseWheel {
        unit: egui::MouseWheelUnit::Point,
        delta: Vec2::new(0.0, crans * 50.0),
        phase: egui::TouchPhase::Move,
        source: egui::MouseWheelSource::Unknown,
        modifiers: egui::Modifiers::NONE,
    });
    b.run_steps(90);
}

/// Un point du fond du plan où il n'y a ni élément, ni fil, ni panneau : sous l'interrupteur b,
/// à gauche de la porte g5.
fn fond_libre(b: &Banc) -> Pos2 {
    let app = b.state();
    let p = app.plan.cam.ecran(Point::nouveau(170.0, 470.0));
    assert!(app.disposition.unwrap().libre.contains(p));
    assert!(
        !app.plan.obstacles.iter().any(|r| r.contains(p)),
        "le point d'essai est sous un panneau"
    );
    p
}

#[test]
fn au_lancement_le_circuit_est_cadre_et_la_simulation_arrive_au_repos() {
    let b = banc();
    let app = b.state();
    assert_eq!(app.atelier.session.t, 3);
    assert_eq!(app.atelier.etat(), Etat::Stable);
    assert_eq!(app.atelier.quete.as_ref().unwrap().vus.len(), 1);
    let d = app.disposition.unwrap();
    for e in &app.atelier.doc.elements {
        let p = app.plan.centre_ecran(&app.atelier, e.id);
        assert!(
            d.libre.shrink(20.0).contains(p),
            "{} est hors de la zone libre : {p:?} dans {:?}",
            e.nom,
            d.libre
        );
    }
    assert!(
        app.plan.cam.k > 0.6,
        "le circuit est affiché assez grand : zoom {}",
        app.plan.cam.k
    );
}

#[test]
fn cliquer_sur_un_interrupteur_le_bascule_et_la_quete_avance() {
    let mut b = banc();
    assert!(b.state().atelier.invite);
    let p = centre(&b, "b");
    cliquer(&mut b, p);
    let ib = id(&b, "b");
    assert_eq!(b.state().atelier.session.entrees[&ib], 1);
    assert!(!b.state().atelier.invite);
    assert_eq!(
        position(&b, "b"),
        Point::nouveau(61.0, 345.0),
        "un clic ne déplace pas l'interrupteur"
    );
    b.run_steps(300);
    assert_eq!(b.state().atelier.etat(), Etat::Stable);
    assert_eq!(vus(&b), 2);
}

#[test]
fn glisser_une_porte_la_deplace_les_fils_suivent_et_ctrl_z_annule() {
    let mut b = banc();
    let k = b.state().plan.cam.k;
    let depart = centre(&b, "g4");
    let avant = position(&b, "g4");
    tirer(&mut b, depart, depart + Vec2::new(90.0, -30.0));
    assert!(matches!(
        b.state().plan.geste,
        Geste::Element { bouge: true, .. }
    ));
    b.drop_at(depart + Vec2::new(90.0, -30.0));
    b.run_steps(60);
    let apres = position(&b, "g4");
    assert!(
        (apres.x - (avant.x + 90.0 / k)).abs() <= 2.5,
        "x : {} → {}",
        avant.x,
        apres.x
    );
    assert!(
        (apres.y - (avant.y - 30.0 / k)).abs() <= 2.5,
        "y : {} → {}",
        avant.y,
        apres.y
    );
    assert_eq!(apres.x % 5.0, 0.0, "la porte se range sur la grille");
    assert_eq!(
        b.state().atelier.selection,
        Some(Selection::Elements(vec![id(&b, "g4")]))
    );
    // La position affichée a rejoint la position réelle.
    let affichee = b.state().plan.position(&b.state().atelier, id(&b, "g4"));
    assert!(affichee.distance(apres) < 0.1);
    assert_eq!(
        vus(&b),
        1,
        "déplacer ne change pas le câblage : la quête garde ses cas"
    );
    b.key_press_modifiers(egui::Modifiers::COMMAND, egui::Key::Z);
    b.run_steps(60);
    assert_eq!(position(&b, "g4"), avant);
    b.key_press_modifiers(egui::Modifiers::COMMAND, egui::Key::Y);
    b.run_steps(5);
    assert_eq!(position(&b, "g4"), apres);
}

#[test]
fn une_porte_presque_alignee_se_redresse_toute_seule() {
    let mut b = banc();
    let k = b.state().plan.cam.k;
    let depart = centre(&b, "s");
    // On descend la lampe de 30 unités, puis on la remonte à 3 unités de l'alignement avec g4.
    glisser(&mut b, depart, depart + Vec2::new(40.0, 30.0 * k));
    let milieu = centre(&b, "s");
    b.run_steps(30);
    glisser(&mut b, milieu, milieu + Vec2::new(0.0, -27.0 * k));
    b.run_steps(30);
    assert_eq!(
        position(&b, "s").y,
        240.0,
        "la lampe s'aligne sur la sortie de g4"
    );
}

#[test]
fn on_debranche_un_fil_puis_on_le_retire_a_la_souris() {
    let mut b = banc();
    let (g2, g4) = (id(&b, "g2"), id(&b, "g4"));
    let fil = Fil {
        puits: g4,
        entree: 0,
    };
    assert_eq!(b.state().atelier.doc.source_de(fil), Some(Source::de(g2)));
    // Attraper le fil par son arrivée et le lâcher dans le vide : il est débranché.
    let arrivee = port(&b, Port::Entree(fil));
    let vide = fond_libre(&b);
    glisser(&mut b, arrivee + Vec2::new(-3.0, 0.0), vide);
    assert_eq!(b.state().atelier.doc.source_de(fil), None);
    assert_eq!(vus(&b), 0, "le câblage a changé : la quête repart de zéro");
    // Tirer un fil de la sortie de g2 jusqu'à proximité de l'entrée : il s'accroche tout seul.
    let sortie = port(&b, Port::Sortie(Source::de(g2)));
    tirer(
        &mut b,
        sortie + Vec2::new(3.0, 0.0),
        arrivee + Vec2::new(-14.0, 9.0),
    );
    assert_eq!(
        b.state().plan.geste,
        Geste::Fil {
            depuis: Port::Sortie(Source::de(g2)),
            aimant: Some(Port::Entree(fil))
        }
    );
    b.drop_at(arrivee + Vec2::new(-14.0, 9.0));
    b.step();
    assert_eq!(b.state().atelier.doc.source_de(fil), Some(Source::de(g2)));
    assert_eq!(b.state().atelier.selection, Some(Selection::Fil(fil)));
    // Deux actions annulables : le débranchement et le rebranchement.
    b.key_press_modifiers(egui::Modifiers::COMMAND, egui::Key::Z);
    b.step();
    assert_eq!(b.state().atelier.doc.source_de(fil), None);
    b.key_press_modifiers(egui::Modifiers::COMMAND, egui::Key::Z);
    b.step();
    assert_eq!(b.state().atelier.doc.source_de(fil), Some(Source::de(g2)));
}

#[test]
fn un_fil_se_tire_aussi_depuis_une_entree_libre_vers_une_sortie() {
    let mut b = banc();
    let (a, g5) = (id(&b, "a"), id(&b, "g5"));
    let fil = Fil {
        puits: g5,
        entree: 1,
    };
    let arrivee = port(&b, Port::Entree(fil));
    let vide = fond_libre(&b);
    glisser(&mut b, arrivee, vide);
    assert_eq!(b.state().atelier.doc.source_de(fil), None);
    let sortie_a = port(&b, Port::Sortie(Source::de(a)));
    glisser(&mut b, arrivee, sortie_a + Vec2::new(10.0, 6.0));
    assert_eq!(b.state().atelier.doc.source_de(fil), Some(Source::de(a)));
}

#[test]
fn cliquer_un_fil_le_selectionne_et_suppr_le_debranche() {
    let mut b = banc();
    let (g5, c) = (id(&b, "g5"), id(&b, "c"));
    let fil = Fil {
        puits: c,
        entree: 0,
    };
    let milieu = port(&b, Port::Sortie(Source::de(g5))).lerp(port(&b, Port::Entree(fil)), 0.5);
    cliquer(&mut b, milieu);
    assert_eq!(b.state().atelier.selection, Some(Selection::Fil(fil)));
    b.key_press(egui::Key::Delete);
    b.step();
    assert_eq!(b.state().atelier.doc.source_de(fil), None);
    assert_eq!(b.state().atelier.selection, None);
    // Cliquer dans le vide désélectionne.
    let p = centre(&b, "g1");
    cliquer(&mut b, p);
    assert!(b.state().atelier.selection.is_some());
    let vide = fond_libre(&b);
    cliquer(&mut b, vide);
    assert_eq!(b.state().atelier.selection, None);
}

#[test]
fn la_molette_zoome_autour_du_pointeur() {
    let mut b = banc();
    let p = centre(&b, "g3") + Vec2::new(30.0, 50.0);
    let (avant, monde) = (b.state().plan.cam.k, b.state().plan.cam.monde(p));
    molette(&mut b, p, 3.0);
    let cam = b.state().plan.cam;
    assert!(cam.k > avant * 1.3, "zoom {avant} → {}", cam.k);
    assert!(
        cam.ecran(monde).distance(p) < 0.5,
        "le point sous le pointeur n'a pas bougé"
    );
    molette(&mut b, p, -40.0);
    assert_eq!(
        b.state().plan.cam.k,
        Apparence::default().manipulation.zoom_min
    );
    // F recadre.
    b.key_press(egui::Key::F);
    b.run_steps(120);
    assert!((b.state().plan.cam.k - avant).abs() < 0.01);
}

#[test]
fn glisser_le_fond_deplace_le_plan_puis_il_glisse_encore_un_peu() {
    let mut b = banc();
    let depart = fond_libre(&b);
    let avant = b.state().plan.cam;
    tirer(&mut b, depart, depart + Vec2::new(120.0, 40.0));
    let pendant = b.state().plan.cam;
    assert!(
        (pendant.x - avant.x - 120.0).abs() < 6.0,
        "le plan suit la souris sans retard : {} → {}",
        avant.x,
        pendant.x
    );
    b.drop_at(depart + Vec2::new(120.0, 40.0));
    b.run_steps(120);
    let apres = b.state().plan.cam;
    assert!(
        apres.x > pendant.x + 5.0,
        "le plan continue sur sa lancée : {} → {}",
        pendant.x,
        apres.x
    );
    assert_eq!(apres.k, avant.k);
    assert_eq!(
        b.state().atelier.doc,
        Document::demi_additionneur(),
        "déplacer le plan ne touche pas au circuit"
    );
}

#[test]
fn la_brique_nand_se_pose_par_clic_ou_en_la_tirant_sur_le_plan() {
    let mut b = banc();
    let d = b.state().disposition.unwrap();
    let brique = repere(&b, "nand", 0);
    cliquer(&mut b, brique);
    assert_eq!(
        b.state().atelier.doc.portes().count(),
        6,
        "un clic sur la brique pose une porte"
    );
    assert_eq!(
        b.state().atelier.nom_selection().as_deref(),
        Some("g6 · NAND")
    );
    // La tirer jusqu'au plan : elle suit le pointeur et se pose où on la lâche.
    let cible = fond_libre(&b) + Vec2::new(30.0, -40.0);
    glisser(&mut b, brique, cible);
    b.run_steps(30);
    assert_eq!(b.state().atelier.doc.portes().count(), 7);
    assert!(
        centre(&b, "g7").distance(cible) < 6.0,
        "la porte est là où on l'a lâchée : {:?} / {cible:?}",
        centre(&b, "g7")
    );
    // La lâcher sur un panneau : elle retourne dans la palette.
    glisser(&mut b, brique, d.quete.center());
    assert_eq!(b.state().atelier.doc.portes().count(), 7);
    // Deux poses à annuler.
    for _ in 0..2 {
        b.key_press_modifiers(egui::Modifiers::COMMAND, egui::Key::Z);
        b.step();
    }
    assert_eq!(b.state().atelier.doc.portes().count(), 5);
}

#[test]
fn les_panneaux_arretent_les_clics_et_se_cachent() {
    let mut b = banc();
    let d = b.state().disposition.unwrap();
    // Zoomer très fort amène le circuit sous les panneaux : cliquer sur un panneau ne touche pas au plan.
    let p = centre(&b, "g1");
    cliquer(&mut b, p);
    let choisi = b.state().atelier.selection.clone();
    assert!(choisi.is_some());
    cliquer(&mut b, d.infos.center());
    assert_eq!(
        b.state().atelier.selection,
        choisi,
        "un clic sur un panneau ne désélectionne rien"
    );
    let zoom = b.state().plan.cam.k;
    molette(&mut b, d.infos.center(), 3.0);
    assert_eq!(
        b.state().plan.cam.k,
        zoom,
        "la molette au-dessus d'un panneau ne zoome pas le plan"
    );
    // Le premier bouton de la barre des panneaux cache les briques : le plan gagne la place.
    cliquer(
        &mut b,
        Pos2::new(d.volets.left() + 40.0, d.volets.center().y),
    );
    assert!(!b.state().atelier.volets.briques);
    b.run_steps(60);
    let apres = b.state().disposition.unwrap();
    assert!(apres.libre.left() < d.libre.left() - 100.0);
    assert!(!apres.ecran.intersects(apres.briques));
}

#[test]
fn le_clavier_pilote_la_lecture_la_suppression_et_le_deplacement() {
    let mut b = banc();
    assert!(b.state().atelier.lecture);
    b.key_press(egui::Key::Space);
    b.step();
    assert!(!b.state().atelier.lecture);
    let p = centre(&b, "g5");
    cliquer(&mut b, p);
    let avant = position(&b, "g5");
    b.key_press(egui::Key::ArrowRight);
    b.step();
    b.key_press(egui::Key::ArrowDown);
    b.step();
    assert_eq!(
        position(&b, "g5"),
        Point::nouveau(avant.x + 10.0, avant.y + 10.0)
    );
    b.key_press(egui::Key::Delete);
    b.step();
    assert_eq!(b.state().atelier.doc.portes().count(), 4);
    // Un interrupteur ne se supprime pas.
    let p = centre(&b, "a");
    tirer(&mut b, p, p + Vec2::new(0.0, -12.0));
    b.drop_at(p + Vec2::new(0.0, -12.0));
    b.step();
    b.key_press(egui::Key::Delete);
    b.step();
    assert!(b.state().atelier.doc.elements.iter().any(|e| e.nom == "a"));
}

#[test]
fn cliquer_dans_le_chronogramme_remonte_le_temps() {
    let mut b = banc();
    let d = b.state().disposition.unwrap();
    let cellule = (d.temps.width() - 24.0 - 96.0 - 24.0 - 250.0 - 24.0) / 16.0;
    let instant_1 = Pos2::new(
        d.temps.left() + 24.0 + 96.0 + cellule * 1.5,
        d.temps.top() + 56.0 + 20.0 + 30.0,
    );
    cliquer(&mut b, instant_1);
    assert_eq!(b.state().atelier.etat(), Etat::Passe(1));
    let g4 = id(&b, "g4");
    assert_eq!(
        ce_app::simulation::symbole(b.state().atelier.valeur(Source::de(g4))),
        'X',
        "à t = 1 la somme est encore inconnue"
    );
    // Lecture ramène au présent.
    b.key_press(egui::Key::Space);
    b.step();
    assert_eq!(b.state().atelier.etat(), Etat::Stable);
}

#[test]
fn la_quete_se_reussit_en_cliquant_les_lignes_de_la_table() {
    let mut b = banc();
    for ligne in 0..4 {
        let p = repere(&b, "cas", ligne);
        cliquer(&mut b, p);
        b.run_steps(400);
        assert_eq!(b.state().atelier.etat(), Etat::Stable);
    }
    assert!(
        b.state().atelier.quete.as_ref().unwrap().reussie(),
        "cas validés : {}",
        vus(&b)
    );
    // Le panneau de la quête, ouvert, dit les félicitations : aucun message ne les répète
    // par-dessus le circuit. La fête est lancée, et la suite est proposée sans défiler.
    let at = &b.state().atelier;
    assert!(at.fete.is_some(), "la réussite se fête");
    assert!(!at.messages.iter().any(|m| m.texte.contains("quatre cas")));
    let suivante = b
        .state()
        .panneaux
        .repere("quete-suivante", 0)
        .expect("« Quête suivante » affichée sans défiler");
    let panneau = b.state().disposition.unwrap().quete;
    assert!(
        panneau.contains_rect(suivante),
        "« Quête suivante » entier dans le panneau : {suivante:?} dans {panneau:?}"
    );

    // Panneau fermé, les félicitations passent par un message.
    let mut b = banc();
    b.state_mut().atelier.volets.quete = false;
    b.run_steps(400);
    for nom in ["b", "a", "b"] {
        let interrupteur = id(&b, nom);
        b.state_mut().atelier.basculer(interrupteur);
        b.run_steps(400);
    }
    let at = &b.state().atelier;
    assert!(
        at.quete.as_ref().unwrap().reussie(),
        "cas validés : {}",
        vus(&b)
    );
    assert!(at.messages.iter().any(|m| m.texte.contains("quatre cas")));
}

#[test]
fn un_cadre_selectionne_les_portes_ctrl_g_en_fait_une_brique_qui_marche_et_s_ouvre() {
    let mut b = banc();
    // Le cadre : Maj enfoncée, on tire sur le fond du plan autour des cinq portes.
    let portes: Vec<Id> = b.state().atelier.doc.portes().map(|e| e.id).collect();
    let centres: Vec<Pos2> = (1..=5).map(|i| centre(&b, &format!("g{i}"))).collect();
    let boite = egui::Rect::from_points(&centres).expand2(Vec2::new(46.0, 52.0));
    {
        let app = b.state();
        let depart = app.plan.cam.monde(boite.left_top());
        assert_eq!(
            app.plan.viser(&app.apparence, &app.atelier, depart),
            Survol::Rien,
            "le cadre part du fond du plan"
        );
    }
    majuscule(&mut b, true);
    tirer(&mut b, boite.left_top(), boite.right_bottom());
    assert!(matches!(b.state().plan.geste, Geste::Cadre { .. }));
    b.drop_at(boite.right_bottom());
    b.step();
    majuscule(&mut b, false);
    let choisis = b.state().atelier.elements_selectionnes().to_vec();
    assert!(
        portes.iter().all(|p| choisis.contains(p)),
        "les cinq portes sont dans le cadre : {choisis:?}"
    );

    // Ctrl+G : les portes deviennent une brique, rangée dans la palette.
    b.key_press_modifiers(egui::Modifiers::COMMAND, egui::Key::G);
    b.run_steps(400);
    {
        let at = &b.state().atelier;
        assert_eq!(at.doc.portes().count(), 0);
        assert_eq!(at.projet.modeles.len(), 1);
        assert_eq!(at.projet.nb_portes(&at.doc), 5);
        assert_eq!(at.session.nb_portes(), 5);
        assert_eq!(at.session.erreur(), None);
        assert_eq!(at.etat(), Etat::Stable);
    }
    // Le circuit se comporte comme avant : la quête valide toujours les cas.
    assert_eq!(vus(&b), 1);
    let p = centre(&b, "b");
    cliquer(&mut b, p);
    b.run_steps(400);
    assert_eq!(vus(&b), 2);

    // La brique s'ouvre d'un double-clic : on voit ses cinq portes ; Échap la referme.
    let brique = b
        .state()
        .atelier
        .doc
        .elements
        .iter()
        .find(|e| e.genre == Genre::Brique)
        .unwrap()
        .nom
        .clone();
    let p = centre(&b, &brique);
    double_cliquer(&mut b, p);
    b.run_steps(60);
    assert_eq!(b.state().atelier.chemin.len(), 1);
    assert_eq!(b.state().atelier.doc.portes().count(), 5);
    assert!(
        b.state().panneaux.repere("ariane", 1).is_some(),
        "le fil d'Ariane montre la brique ouverte"
    );
    b.key_press(egui::Key::Escape);
    b.step();
    b.key_press(egui::Key::Escape);
    b.run_steps(30);
    assert!(b.state().atelier.chemin.is_empty());
    assert_eq!(b.state().atelier.doc.portes().count(), 0);

    // La palette pose une deuxième fois la même brique.
    let modele = repere(&b, "modele", 0);
    cliquer(&mut b, modele);
    b.run_steps(30);
    let nom = b.state().atelier.projet.modeles[0].nom.clone();
    assert_eq!(b.state().atelier.projet.nb_utilisations(&nom), 2);
    assert_eq!(b.state().atelier.session.nb_portes(), 10);
}

#[test]
fn le_plan_s_enregistre_comme_brique_s_ouvre_depuis_la_palette_et_le_fil_d_ariane_ramene() {
    let mut b = banc();
    let bouton = repere(&b, "enregistrer-brique", 0);
    cliquer(&mut b, bouton);
    b.run_steps(30);
    assert_eq!(b.state().atelier.projet.modeles.len(), 1);
    let (e, s) = b.state().atelier.projet.modeles[0].plan.interface();
    assert_eq!(
        (e.len(), s.len()),
        (2, 2),
        "les interrupteurs et les lampes sont devenus les broches"
    );
    let ouvrir = repere(&b, "ouvrir-modele", 0);
    cliquer(&mut b, ouvrir);
    b.run_steps(60);
    assert_eq!(b.state().atelier.chemin.len(), 1);
    // Dans la brique, elle ne peut pas se poser elle-même.
    let modele = repere(&b, "modele", 0);
    cliquer(&mut b, modele);
    b.step();
    assert!(!b
        .state()
        .atelier
        .doc
        .elements
        .iter()
        .any(|e| e.genre == Genre::Brique));
    let racine = repere(&b, "ariane", 0);
    cliquer(&mut b, racine);
    b.run_steps(60);
    assert!(b.state().atelier.chemin.is_empty());
    assert!(
        b.state().atelier.quete.is_some(),
        "de retour sur le plan principal, la quête est toujours là"
    );
}

#[test]
fn maj_clic_ajoute_a_la_selection_et_tout_se_deplace_se_duplique_ensemble() {
    let mut b = banc();
    let (p1, p2) = (centre(&b, "g1"), centre(&b, "g2"));
    cliquer(&mut b, p1);
    majuscule(&mut b, true);
    cliquer(&mut b, p2);
    majuscule(&mut b, false);
    let (g1, g2) = (id(&b, "g1"), id(&b, "g2"));
    assert_eq!(
        b.state().atelier.selection,
        Some(Selection::Elements(vec![g1, g2]))
    );
    assert_eq!(
        b.state().atelier.nom_selection().as_deref(),
        Some("2 éléments")
    );
    let (a1, a2) = (position(&b, "g1"), position(&b, "g2"));
    b.key_press(egui::Key::ArrowRight);
    b.step();
    assert_eq!(position(&b, "g1"), Point::nouveau(a1.x + 10.0, a1.y));
    assert_eq!(position(&b, "g2"), Point::nouveau(a2.x + 10.0, a2.y));
    // Glisser un des deux déplace les deux.
    let k = b.state().plan.cam.k;
    let depart = centre(&b, "g1");
    glisser(&mut b, depart, depart + Vec2::new(0.0, -40.0));
    b.run_steps(30);
    let (b1, b2) = (position(&b, "g1"), position(&b, "g2"));
    assert!(
        (b1.y - (a1.y - 40.0 / k)).abs() <= 2.5,
        "{} → {}",
        a1.y,
        b1.y
    );
    assert!(((b2.y - a2.y) - (b1.y - a1.y)).abs() < 0.01);
    // Le bouton « Dupliquer » de la barre de sélection copie les deux portes.
    let dupliquer = repere(&b, "action", Action::Dupliquer as usize);
    cliquer(&mut b, dupliquer);
    b.step();
    assert_eq!(b.state().atelier.doc.portes().count(), 7);
    assert_eq!(b.state().atelier.elements_selectionnes().len(), 2);
    // Ctrl+A sélectionne tout, Échap lâche tout.
    b.key_press_modifiers(egui::Modifiers::COMMAND, egui::Key::A);
    b.step();
    assert_eq!(
        b.state().atelier.elements_selectionnes().len(),
        b.state().atelier.doc.elements.len()
    );
    b.key_press(egui::Key::Escape);
    b.step();
    assert_eq!(b.state().atelier.selection, None);
}

#[test]
fn un_element_se_renomme_dans_les_infos_sans_declencher_les_raccourcis() {
    let mut b = banc();
    let p = centre(&b, "g3");
    cliquer(&mut b, p);
    let g3 = id(&b, "g3");
    let champ = repere(&b, "champ", 0);
    cliquer(&mut b, champ);
    b.run_steps(3);
    // Le texte tapé remplace l'ancien nom ; l'espace et « f » vont au champ, pas à l'application.
    let (lecture, zoom) = (b.state().atelier.lecture, b.state().plan.cam.k);
    taper(&mut b, "ou f");
    assert_eq!(b.state().atelier.lecture, lecture);
    assert_eq!(b.state().plan.cam.k, zoom);
    b.key_press(egui::Key::Enter);
    b.run_steps(3);
    assert_eq!(b.state().atelier.doc.element(g3).unwrap().nom, "ou f");
    assert_eq!(b.state().atelier.doc.portes().count(), 5);
    // Échap abandonne la saisie.
    let champ = repere(&b, "champ", 0);
    cliquer(&mut b, champ);
    b.run_steps(3);
    taper(&mut b, "autre");
    b.key_press(egui::Key::Escape);
    b.run_steps(3);
    assert_eq!(b.state().atelier.doc.element(g3).unwrap().nom, "ou f");
    // Les interrupteurs et les lampes de la quête gardent leur nom.
    let p = centre(&b, "a");
    tirer(&mut b, p, p + Vec2::new(0.0, -12.0));
    b.drop_at(p + Vec2::new(0.0, -12.0));
    b.run_steps(3);
    let champ = repere(&b, "champ", 0);
    cliquer(&mut b, champ);
    b.run_steps(3);
    taper(&mut b, "x");
    b.key_press(egui::Key::Enter);
    b.run_steps(3);
    assert!(b.state().atelier.doc.nomme("a").is_some());
    assert!(b
        .state()
        .atelier
        .messages
        .iter()
        .any(|m| m.texte.contains("La quête a besoin de ce nom")));
}

/// L'ordinateur miniature ouvert depuis la liste des projets, avec ses 3 401 portes.
fn banc_ordinateur() -> Banc {
    let mut b = banc();
    let exemple = repere(&b, "projet", 1);
    cliquer(&mut b, exemple);
    b.run_steps(120);
    assert_eq!(b.state().atelier.projet.nom, "Ordinateur miniature");
    b
}

#[test]
fn l_ordinateur_miniature_s_ouvre_calcule_au_clavier_et_se_visite_brique_par_brique() {
    let mut b = banc_ordinateur();
    {
        let at = &b.state().atelier;
        assert_eq!(at.session.nb_portes(), 3401);
        assert_eq!(at.session.erreur(), None);
        assert!(at.session.a_une_horloge());
        assert!(at.quete.is_none());
        assert!(at.lecture, "il tourne dès l'ouverture");
    }
    // On clique sur le clavier du plan puis on tape « 67 » : l'écran affiche le produit.
    let clavier = centre(&b, "clavier");
    cliquer(&mut b, clavier);
    assert_eq!(
        b.state().atelier.clavier_selectionne(),
        Some(id(&b, "clavier"))
    );
    taper(&mut b, "67");
    let ecran = id(&b, "ecran");
    let mut images = 0;
    while b.state().atelier.session.ecran(ecran) != "6*7=42" && images < 6000 {
        b.step();
        images += 1;
    }
    assert_eq!(
        b.state().atelier.session.ecran(ecran),
        "6*7=42",
        "après {images} images"
    );
    assert!(
        b.state().atelier.lecture,
        "l'espace tapé ne met pas en pause"
    );

    // On entre dans l'ordinateur, puis dans son processeur : chaque niveau est un vrai plan.
    let pc = centre(&b, "pc");
    double_cliquer(&mut b, pc);
    b.run_steps(60);
    assert_eq!(b.state().atelier.chemin, ["ORDINATEUR"]);
    assert_eq!(
        b.state().atelier.etapes_du_chemin(),
        ["pc · ORDINATEUR"],
        "le fil d'Ariane nomme l'exemplaire ouvert"
    );
    {
        // C'est l'ordinateur posé qu'on regarde : il continue de tourner pendant qu'on est dedans.
        let at = &b.state().atelier;
        assert!(at.session.exemplaire());
        assert!(at.lecture);
    }
    let t = b.state().atelier.session.t;
    b.run_steps(30);
    assert!(b.state().atelier.session.t > t, "le temps avance dedans");
    let dedans = b.state().atelier.doc.elements.len();
    assert!(dedans > 5, "le plan de l'ordinateur a {dedans} éléments");
    let p = {
        let app = b.state();
        let libre = app.disposition.unwrap().libre.shrink(30.0);
        app.atelier
            .doc
            .elements
            .iter()
            .filter(|e| e.genre == Genre::Brique)
            .map(|e| app.plan.centre_ecran(&app.atelier, e.id))
            .find(|p| libre.contains(*p) && !app.plan.obstacles.iter().any(|r| r.contains(*p)))
            .expect("l'ordinateur est fait de briques, visibles sur le plan")
    };
    double_cliquer(&mut b, p);
    b.run_steps(60);
    assert_eq!(b.state().atelier.chemin.len(), 2);
    assert!(b.state().panneaux.repere("ariane", 2).is_some());
    {
        // Les signaux de cette brique sont ceux de l'ordinateur qui tourne, pas des X.
        let at = &b.state().atelier;
        assert!(at.session.exemplaire());
        let lectures: Vec<_> = at.doc.prises().map(|s| at.lecture(s)).collect();
        let nettes = lectures.iter().filter(|l| l.flous == 0).count();
        assert!(
            nettes * 2 > lectures.len(),
            "{nettes} signaux déterminés sur {}",
            lectures.len()
        );
    }
    // Retour au plan principal par le fil d'Ariane : l'ordinateur n'a pas perdu son écran.
    let racine = repere(&b, "ariane", 0);
    cliquer(&mut b, racine);
    b.run_steps(60);
    assert!(b.state().atelier.chemin.is_empty());
    assert_eq!(b.state().atelier.session.ecran(ecran), "6*7=42");
}

#[test]
fn le_projet_s_enregistre_tout_seul_et_se_rouvre_au_lancement_suivant() {
    let racine = std::env::temp_dir().join(format!("ce-interface-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&racine);
    let nouvelle = || {
        Application::nouvelle(Apparence::default(), Surveillance::default())
            .avec_dossier(Dossier::dans(&racine))
    };
    let mut b = banc_avec(nouvelle());
    assert!(
        Dossier::dans(&racine).lister().is_empty(),
        "rien n'a changé : rien n'est écrit"
    );
    let nand = repere(&b, "nand", 0);
    cliquer(&mut b, nand);
    b.run_steps(30);
    assert!(
        Dossier::dans(&racine).lister().is_empty(),
        "on attend la fin du geste"
    );
    b.run_steps(90);
    assert_eq!(Dossier::dans(&racine).lister(), ["Premier circuit"]);
    // Ouvrir l'exemple n'écrit rien tant qu'on n'y touche pas : il reste dans les exemples.
    let exemple = repere(&b, "projet", 1);
    cliquer(&mut b, exemple);
    b.run_steps(180);
    assert_eq!(Dossier::dans(&racine).lister(), ["Premier circuit"]);
    assert_eq!(b.state().panneaux.projets, ["Premier circuit"]);
    // Dès qu'on le modifie, il s'enregistre à son tour, sans toucher au premier projet.
    let nand = repere(&b, "nand", 0);
    cliquer(&mut b, nand);
    b.run_steps(180);
    assert_eq!(
        Dossier::dans(&racine).lister(),
        ["Ordinateur miniature", "Premier circuit"]
    );
    assert_eq!(b.state().panneaux.projets, ["Premier circuit"]);
    drop(b);

    // Au lancement suivant, on retrouve le dernier projet ouvert.
    let b = banc_avec(nouvelle());
    assert_eq!(b.state().atelier.projet.nom, "Ordinateur miniature");
    assert_eq!(b.state().atelier.session.nb_portes(), 3402);
    // Et le premier circuit a gardé sa sixième porte.
    let premier = Dossier::dans(&racine).ouvrir("Premier circuit").unwrap();
    assert_eq!(premier.principal.portes().count(), 6);
    assert!(premier.quete.is_some());
    let _ = std::fs::remove_dir_all(&racine);
}

/// Un grand plan à plat : `n` portes en nappe, chacune lisant deux portes de la rangée d'avant.
fn nappe(colonnes: usize, rangees: usize) -> Projet {
    let mut plan = Document::nouveau();
    let horloge = plan.poser(Element::horloge("h", 4), Point::nouveau(0.0, 0.0));
    let mut avant: Vec<Id> = vec![horloge; colonnes];
    for r in 0..rangees {
        let mut ligne = Vec::with_capacity(colonnes);
        for c in 0..colonnes {
            let g = plan.poser(
                Element::nand(""),
                Point::nouveau(160.0 + r as f32 * 130.0, c as f32 * 70.0),
            );
            for (entree, source) in [avant[c], avant[(c + 1) % colonnes]]
                .into_iter()
                .enumerate()
            {
                assert!(plan.relier(Source::de(source), Fil { puits: g, entree }));
            }
            ligne.push(g);
        }
        avant = ligne;
    }
    Projet::nouveau("Nappe", plan)
}

#[test]
fn un_plan_de_deux_mille_portes_s_affiche_se_simule_et_se_manipule() {
    let mut b = banc();
    {
        let app = b.state_mut();
        let a = app.apparence.clone();
        app.atelier.ouvrir(nappe(40, 50), &a);
    }
    b.run_steps(200);
    {
        let app = b.state();
        assert_eq!(app.atelier.session.nb_portes(), 2000);
        assert_eq!(app.atelier.session.erreur(), None);
        assert!(app.atelier.session.t > 0, "l'horloge fait avancer le temps");
        let (elements, _) = app.plan.dessines;
        assert!(elements > 0);
    }
    // Zoomer au milieu du plan : seule la partie visible est dessinée.
    let milieu = b.state().disposition.unwrap().libre.center();
    molette(&mut b, milieu, 30.0);
    let (elements, _) = b.state().plan.dessines;
    assert!(
        elements < 400,
        "au zoom maximum, {elements} éléments dessinés sur 2001"
    );
    // On attrape une porte et on la déplace comme sur un petit plan.
    let (cible, nom) = {
        let app = b.state();
        let libre = app.disposition.unwrap().libre.shrink(60.0);
        app.atelier
            .doc
            .portes()
            .map(|e| (app.plan.centre_ecran(&app.atelier, e.id), e.nom.clone()))
            .find(|(p, _)| libre.contains(*p) && !app.plan.obstacles.iter().any(|r| r.contains(*p)))
            .expect("une porte est visible")
    };
    let avant = position(&b, &nom);
    glisser(&mut b, cible, cible + Vec2::new(0.0, 60.0));
    b.run_steps(20);
    assert!(position(&b, &nom).y > avant.y);
}

/// Mesure le temps de calcul d'une image (gestes, simulation, mise en page, dessin, sans le
/// rendu par la carte graphique) sur un grand plan à plat, puis sur l'ordinateur miniature.
/// À lancer à la demande :
/// `cargo test --release -p ce-app --test interface -- --ignored --nocapture cadence`.
#[test]
#[ignore = "mesure, pas un contrôle"]
fn cadence_des_grands_circuits() {
    let mesurer = |b: &mut Banc, quoi: &str| {
        let t0 = b.state().atelier.session.t;
        let debut = std::time::Instant::now();
        let images = 300;
        b.run_steps(images);
        let par_image = debut.elapsed().as_secs_f64() * 1000.0 / images as f64;
        let (elements, fils) = b.state().plan.dessines;
        println!(
            "{quoi} : {par_image:.2} ms par image ({:.0} images/s possibles), {elements} éléments et {fils} fils dessinés, {} pas simulés par image",
            1000.0 / par_image,
            (b.state().atelier.session.t - t0) / images as u64
        );
    };
    for (colonnes, rangees) in [(40, 50), (100, 100)] {
        let mut b = banc();
        let a = b.state().apparence.clone();
        b.state_mut().atelier.ouvrir(nappe(colonnes, rangees), &a);
        b.run_steps(120);
        mesurer(
            &mut b,
            &format!("nappe de {} portes, vue entière", colonnes * rangees),
        );
        let milieu = b.state().disposition.unwrap().libre.center();
        molette(&mut b, milieu, 30.0);
        mesurer(
            &mut b,
            &format!("nappe de {} portes, zoom maximum", colonnes * rangees),
        );
        b.key_press(egui::Key::Space);
        b.step();
        mesurer(
            &mut b,
            &format!(
                "nappe de {} portes, zoom maximum, en pause",
                colonnes * rangees
            ),
        );
        b.key_press(egui::Key::F);
        b.run_steps(120);
        mesurer(
            &mut b,
            &format!(
                "nappe de {} portes, vue entière, en pause",
                colonnes * rangees
            ),
        );
    }
    let mut b = banc_ordinateur();
    mesurer(&mut b, "ordinateur miniature, 32 768 pas/s demandés");
    let pc = centre(&b, "pc");
    double_cliquer(&mut b, pc);
    b.run_steps(60);
    mesurer(&mut b, "ordinateur miniature, plan de l'ordinateur ouvert");
    // À la vitesse « max », la simulation prend tout le temps de calcul accordé à chaque image.
    let a = b.state().apparence.clone();
    while b.state().atelier.rapide(&a) != Some(0.0) {
        b.state_mut().atelier.changer_vitesse(&a);
    }
    mesurer(&mut b, "ordinateur miniature, vitesse max");
}

/// Mesure le temps de calcul d'une image (gestes, simulation, mise en page, dessin, sans le
/// rendu par la carte graphique). À lancer à la demande :
/// `cargo test -p ce-app --profile ci --test interface -- --ignored --nocapture cadence`.
#[test]
#[ignore = "mesure, pas un contrôle"]
fn cadence() {
    let mut b = banc();
    let p = centre(&b, "b");
    cliquer(&mut b, p);
    let debut = std::time::Instant::now();
    let images = 600;
    b.run_steps(images);
    let par_image = debut.elapsed().as_secs_f64() * 1000.0 / images as f64;
    println!(
        "{par_image:.3} ms par image, soit {:.0} images par seconde possibles",
        1000.0 / par_image
    );
}

#[test]
fn verifier_la_quete_donne_sa_brique_et_ouvre_la_suivante() {
    let mut b = banc();
    // Le demi-additionneur de l'atelier est juste : Vérifier le valide d'un coup.
    let verifier = repere(&b, "verifier", 0);
    cliquer(&mut b, verifier);
    b.run_steps(30);
    {
        let at = &b.state().atelier;
        let q = at.quete.as_ref().unwrap();
        assert!(q.reussie());
        assert!(q.verdict.as_ref().unwrap().reussi);
        assert!(at.parcours.faite("demi-additionneur"));
        assert!(at.parcours.briques.iter().any(|m| m.nom == "DEMI_ADD"));
    }
    // La quête suivante s'ouvre dans un projet neuf, avec la brique gagnée dans la palette.
    let suivante = repere(&b, "quete-suivante", 0);
    cliquer(&mut b, suivante);
    b.run_steps(60);
    {
        let at = &b.state().atelier;
        assert_eq!(at.quete.as_ref().unwrap().cle, "additionneur-complet");
        assert!(at.projet.modele("DEMI_ADD").is_some());
        assert!(
            at.parcours.faite("demi-additionneur"),
            "l'avancement est gardé"
        );
        let noms: Vec<&str> = at.doc.elements.iter().map(|e| e.nom.as_str()).collect();
        for nom in ["a", "b", "re", "s", "rs"] {
            assert!(noms.contains(&nom), "il manque {nom} dans {noms:?}");
        }
        // Rien n'est construit : la vérification le dit.
    }
    let verifier = montrer_dans_la_quete(&mut b, "verifier", 0);
    cliquer(&mut b, verifier);
    b.run_steps(10);
    assert!(
        !b.state()
            .atelier
            .quete
            .as_ref()
            .unwrap()
            .verdict
            .as_ref()
            .unwrap()
            .reussi
    );
    // Une autre quête se choisit dans la liste du parcours.
    let non = montrer_dans_la_quete(&mut b, "quete-du-parcours", 0);
    cliquer(&mut b, non);
    b.run_steps(60);
    assert_eq!(b.state().atelier.quete.as_ref().unwrap().cle, "non");
}

#[test]
fn la_barre_du_haut_tient_dans_toutes_les_fenetres_et_tailles_de_texte() {
    use ce_app::reglages::Reglages;
    for texte in [1.0_f32, 1.15, 1.3] {
        for (l, h) in [
            (1040.0, 660.0),
            (1280.0, 720.0),
            (1366.0, 768.0),
            (1440.0, 900.0),
            (1920.0, 1080.0),
        ] {
            let reglages = Reglages {
                taille_texte: texte,
                ..Reglages::default()
            };
            let apparence = reglages.appliquer(&Apparence::default());
            let app = Application::nouvelle(apparence, Surveillance::default());
            let mut b = Harness::builder()
                .with_size(Vec2::new(l, h))
                .with_pixels_per_point(1.0)
                .with_step_dt(1.0 / 60.0)
                .build_ui_state(|ui, app: &mut Application| app.image(ui), app);
            // Le passé affiché : la carte de lecture à sa plus grande largeur.
            b.state_mut().atelier.vue_t = Some(0);
            b.run_steps(40);
            let app = b.state();
            let d = app.disposition.unwrap();
            let c = app.panneaux.contenus;
            let cas = format!("{l}×{h}, texte ×{texte}");
            assert!(
                d.marque.right() < d.transport.left() && d.transport.right() < d.volets.left(),
                "{cas} : la barre du haut se chevauche : {:?} {:?} {:?}",
                d.marque,
                d.transport,
                d.volets
            );
            // Chaque carte a la place de son contenu, tel qu'il est dessiné.
            let transport = if d.etroit {
                c.transport_reduit
            } else {
                c.transport
            };
            let volets = if d.compact { c.volets_icones } else { c.volets };
            assert!(
                (d.transport.width() - transport).abs() < 1.0
                    && (d.volets.width() - volets).abs() < 1.0,
                "{cas} : une carte n'a pas la largeur de son contenu"
            );
            // Les libellés sont là dès que la place le permet : toujours dans une grande
            // fenêtre, et dans la fenêtre habituelle avec le texte normal.
            let f = &app.apparence.formes;
            let rangee = 2.0 * (f.marge + f.gouttiere) + c.marque + c.transport + c.volets;
            assert_eq!(d.compact, l < rangee, "{cas}");
            if l >= 1920.0 || (l >= 1440.0 && texte == 1.0) {
                assert!(!d.compact && !d.etroit, "{cas}");
            }
            // Le pire cas : on regarde le passé d'un circuit qui tourne depuis longtemps. Même
            // quand la rangée réduite ne tient plus, aucune carte ne passe sous sa voisine.
            b.state_mut().atelier.vue_t = Some(123_456_789);
            b.run_steps(10);
            let app = b.state();
            let d = app.disposition.unwrap();
            let g = app.apparence.formes.gouttiere;
            assert!(
                d.marque.right() + g <= d.transport.left() + 0.5
                    && d.transport.right() + g <= d.volets.left() + 0.5,
                "{cas}, temps à neuf chiffres : la barre du haut se chevauche : {:?} {:?} {:?}",
                d.marque,
                d.transport,
                d.volets
            );
            assert!(
                d.transport.width() >= 300.0,
                "{cas} : carte de lecture écrasée"
            );
        }
    }
}

#[test]
fn la_barre_du_haut_ne_saute_pas_quand_le_temps_defile() {
    // L'ordinateur miniature a une horloge : son temps prend vite quatre chiffres, puis cinq.
    let mut b = banc_ordinateur();
    let barre = |b: &Banc| {
        let d = b.state().disposition.unwrap();
        (d.transport, d.volets, d.compact, repere(b, "jouer", 0))
    };
    let lance = barre(&b);
    assert!(b.state().atelier.session.t > 1000);
    assert!(
        !lance.2,
        "fenêtre habituelle : les libellés des panneaux restent"
    );
    b.run_steps(120);
    assert!(b.state().atelier.session.t > 100_000);
    assert_eq!(barre(&b), lance, "le temps défile, la barre ne bouge pas");
    // Pause, retour à zéro : « Lecture » est toujours au même endroit.
    cliquer(&mut b, lance.3);
    assert!(!b.state().atelier.lecture);
    let zero = repere(&b, "zero", 0);
    cliquer(&mut b, zero);
    b.run_steps(10);
    assert_eq!(b.state().atelier.session.t, 0);
    assert_eq!(
        barre(&b),
        lance,
        "revenir à zéro ne déplace pas la carte de lecture"
    );
    cliquer(&mut b, lance.3);
    assert!(
        b.state().atelier.lecture,
        "le clic tombe encore sur Lecture"
    );

    // Après un très long temps, la carte a pu grandir : elle ne rétrécit pas ensuite, donc rien
    // ne revient en arrière sous la souris.
    cliquer(&mut b, lance.3);
    b.state_mut().atelier.vue_t = Some(123_456_789);
    b.run_steps(10);
    let long = barre(&b);
    b.state_mut().atelier.vue_t = None;
    b.run_steps(10);
    assert_eq!(barre(&b), long);
    // Un autre projet repart des largeurs de départ.
    b.state_mut().ouvrir(Ouverture::Nouveau);
    b.run_steps(30);
    assert!(!barre(&b).2);
}

/// Le test d'acceptation de bout en bout : dans l'ordinateur miniature ouvert, Ctrl+P ouvre
/// l'éditeur de programme ; un nouveau programme, chargé d'un clic, refabrique la mémoire morte
/// en NAND, et l'ordinateur le fait tourner.
#[test]
fn on_ecrit_un_programme_dans_l_application_et_l_ordinateur_l_execute() {
    let mut b = banc_ordinateur();
    b.key_press_modifiers(egui::Modifiers::COMMAND, egui::Key::P);
    b.run_steps(5);
    let editeur = b
        .state()
        .fenetres
        .editeur
        .clone()
        .expect("l'éditeur s'ouvre");
    assert_eq!(editeur.modele, "MEMOIRE_MORTE");
    assert!(
        editeur.texte.contains("OUT"),
        "il montre le programme actuel"
    );
    // Une erreur s'affiche tout de suite et empêche de charger.
    {
        let e = b.state_mut().fenetres.editeur.as_mut().unwrap();
        e.texte = "LDI 'H'\nSAUTE ailleurs\n".to_owned();
        e.essayer();
    }
    b.run_steps(3);
    assert!(b.query_by_label_contains("ligne 2").is_some());
    {
        let e = b.state_mut().fenetres.editeur.as_mut().unwrap();
        e.texte =
            "        LDI 'H'\n        OUT\n        LDI 'I'\n        OUT\nfin:    HLT\n".to_owned();
        e.essayer();
    }
    b.run_steps(3);
    b.get_by_label("Charger dans la mémoire").click();
    b.run_steps(3);
    let ecran = id(&b, "ecran");
    let mut images = 0;
    while b.state().atelier.session.ecran(ecran) != "HI" && images < 3000 {
        b.step();
        images += 1;
    }
    assert_eq!(
        b.state().atelier.session.ecran(ecran),
        "HI",
        "après {images} images"
    );
    assert!(b
        .state()
        .atelier
        .projet
        .modele("MEMOIRE_MORTE")
        .and_then(|m| m.programme.as_deref())
        .is_some_and(|t| t.contains("'I'")));
    // F1 ouvre l'aide, F1 la referme.
    b.key_press(egui::Key::F1);
    b.run_steps(3);
    assert!(b.state().fenetres.aide.is_some());
    b.key_press(egui::Key::F1);
    b.run_steps(3);
    assert!(b.state().fenetres.aide.is_none());
}
