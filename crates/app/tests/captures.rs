//! Captures d'écran de l'application, rendues sans fenêtre.
//!
//! Elles ne tournent que si la variable `CE_CAPTURES` désigne un dossier de sortie, car elles
//! demandent un pilote graphique (Vulkan logiciel « lavapipe » sous Linux, par exemple) :
//!
//! ```sh
//! CE_CAPTURES=/tmp/captures cargo test -p ce-app --test captures
//! ```
//!
//! Avec `CE_FILM=/tmp/film.mp4` (et `ffmpeg` installé), le test `film` enregistre en plus une
//! démonstration à 60 images par seconde : chaque image est calculée puis rendue, le film
//! montre donc exactement ce que l'application dessine, sans dépendre de la vitesse de la
//! machine qui l'enregistre.

use ce_app::app::Application;
use ce_app::apparence::{Apparence, Mode, Surveillance};
use ce_app::document::Source;
use egui_kittest::Harness;

fn banc(largeur: f32, hauteur: f32, apparence: Apparence) -> Harness<'static, Application> {
    Harness::builder()
        .with_size(egui::Vec2::new(largeur, hauteur))
        .with_pixels_per_point(2.0)
        .with_step_dt(1.0 / 60.0)
        .wgpu()
        .build_ui_state(
            |ui, app: &mut Application| app.image(ui),
            Application::nouvelle(apparence, Surveillance::default()),
        )
}

fn enregistrer(banc: &mut Harness<'static, Application>, dossier: &str, nom: &str) {
    let image = banc.render().expect("rendu");
    std::fs::create_dir_all(dossier).expect("dossier de sortie");
    image
        .save(format!("{dossier}/{nom}.png"))
        .expect("écriture de la capture");
}

fn id(b: &Harness<'static, Application>, nom: &str) -> ce_app::document::Id {
    b.state()
        .atelier
        .doc
        .elements
        .iter()
        .find(|e| e.nom == nom)
        .expect("élément")
        .id
}

fn tirer(b: &mut Harness<'static, Application>, de: egui::Pos2, vers: egui::Pos2) {
    b.hover_at(de);
    b.step();
    b.drag_at(de);
    b.step();
    for i in 1..=12 {
        b.hover_at(de + (vers - de) * (i as f32 / 12.0));
        b.step();
    }
}

#[test]
fn captures() {
    use ce_app::document::Fil;
    use ce_app::plan::Port;
    use egui::Vec2;

    let Ok(dossier) = std::env::var("CE_CAPTURES") else {
        eprintln!("CE_CAPTURES n'est pas défini : captures ignorées.");
        return;
    };
    let mut b = banc(1440.0, 900.0, Apparence::default());
    b.run_steps(400);
    enregistrer(&mut b, &dossier, "1-sombre");

    // Le courant se propage : on bascule b et on regarde quelques images plus tard.
    let p = b.state().plan.centre_ecran(&b.state().atelier, id(&b, "b"));
    b.hover_at(p);
    b.step();
    b.drag_at(p);
    b.step();
    b.drop_at(p);
    b.run_steps(52);
    enregistrer(&mut b, &dossier, "2-propagation");
    b.run_steps(300);

    // On soulève une porte et on la déplace : les fils suivent.
    let g4 = b
        .state()
        .plan
        .centre_ecran(&b.state().atelier, id(&b, "g4"));
    tirer(&mut b, g4, g4 + Vec2::new(70.0, 80.0));
    enregistrer(&mut b, &dossier, "3-deplacement");
    b.drop_at(g4 + Vec2::new(70.0, 80.0));
    b.run_steps(60);

    // On tire un fil : il s'accroche au point le plus proche.
    let sortie = b
        .state()
        .plan
        .port_ecran(&b.state().atelier, Port::Sortie(Source::de(id(&b, "g3"))));
    let entree = b.state().plan.port_ecran(
        &b.state().atelier,
        Port::Entree(Fil {
            puits: id(&b, "g5"),
            entree: 1,
        }),
    );
    tirer(
        &mut b,
        sortie + Vec2::new(3.0, 0.0),
        entree + Vec2::new(-22.0, 14.0),
    );
    b.run_steps(6);
    enregistrer(&mut b, &dossier, "4-cablage");
    b.drop_at(entree + Vec2::new(-22.0, 14.0));
    b.run_steps(40);
    enregistrer(&mut b, &dossier, "5-recable");

    // Panneaux repliés : le plan prend toute la place.
    {
        let v = &mut b.state_mut().atelier.volets;
        v.briques = false;
        v.quete = false;
        v.infos = false;
        v.temps = false;
    }
    b.key_press(egui::Key::F);
    b.run_steps(200);
    enregistrer(&mut b, &dossier, "6-panneaux-replies");

    let mut clair = banc(
        1440.0,
        900.0,
        Apparence {
            mode: Mode::Clair,
            ..Apparence::default()
        },
    );
    clair.run_steps(400);
    enregistrer(&mut clair, &dossier, "7-clair");

    let mut petit = banc(1100.0, 700.0, Apparence::default());
    petit.run_steps(400);
    enregistrer(&mut petit, &dossier, "8-petite-fenetre");
}

/// Captures des briques et de l'ordinateur miniature.
#[test]
fn captures_briques() {
    use ce_app::document::Genre;
    use egui::{Pos2, Rect, Vec2};

    let Ok(dossier) = std::env::var("CE_CAPTURES") else {
        eprintln!("CE_CAPTURES n'est pas défini : captures ignorées.");
        return;
    };
    let cliquer = |b: &mut Harness<'static, Application>, p: Pos2| {
        b.hover_at(p);
        b.step();
        b.drag_at(p);
        b.step();
        b.drop_at(p);
        b.step();
    };
    let double = |b: &mut Harness<'static, Application>, p: Pos2| {
        b.hover_at(p);
        b.step();
        for _ in 0..2 {
            b.drag_at(p);
            b.step();
            b.drop_at(p);
            b.step();
        }
    };
    let centre = |b: &Harness<'static, Application>, nom: &str| {
        b.state().plan.centre_ecran(&b.state().atelier, id(b, nom))
    };
    let mut b = banc(1440.0, 900.0, Apparence::default());
    b.run_steps(400);

    // Un cadre tiré avec Maj autour des cinq portes.
    let centres: Vec<Pos2> = (1..=5).map(|i| centre(&b, &format!("g{i}"))).collect();
    let boite = Rect::from_points(&centres).expand2(Vec2::new(46.0, 52.0));
    b.event(egui::Event::ModifiersChanged(egui::Modifiers::SHIFT));
    b.step();
    tirer(&mut b, boite.left_top(), boite.right_bottom());
    b.run_steps(4);
    enregistrer(&mut b, &dossier, "b1-cadre-de-selection");
    b.drop_at(boite.right_bottom());
    b.step();
    b.event(egui::Event::ModifiersChanged(egui::Modifiers::NONE));
    b.run_steps(30);
    enregistrer(&mut b, &dossier, "b2-selection-multiple");

    // Ctrl+G : les cinq portes deviennent une brique.
    b.key_press_modifiers(egui::Modifiers::COMMAND, egui::Key::G);
    b.run_steps(240);
    enregistrer(&mut b, &dossier, "b3-brique-regroupee");

    // On l'ouvre d'un double-clic.
    let brique = b
        .state()
        .atelier
        .doc
        .elements
        .iter()
        .find(|e| e.genre == Genre::Brique)
        .expect("brique")
        .nom
        .clone();
    let p = centre(&b, &brique);
    double(&mut b, p);
    b.run_steps(240);
    enregistrer(&mut b, &dossier, "b4-dans-la-brique");

    // L'ordinateur miniature, depuis la liste des projets.
    b.key_press(egui::Key::Escape);
    b.step();
    b.key_press(egui::Key::Escape);
    b.run_steps(30);
    let exemple = b
        .state()
        .panneaux
        .repere("projet", 1)
        .expect("exemple affiché")
        .center();
    cliquer(&mut b, exemple);
    b.run_steps(60);
    let clavier = centre(&b, "clavier");
    cliquer(&mut b, clavier);
    for c in "67".chars() {
        b.event(egui::Event::Text(c.to_string()));
        b.step();
    }
    let ecran = id(&b, "ecran");
    let mut images = 0;
    while b.state().atelier.session.ecran(ecran) != "6*7=42" && images < 6000 {
        b.step();
        images += 1;
    }
    b.run_steps(30);
    enregistrer(&mut b, &dossier, "b5-ordinateur-6x7");

    // Dans l'ordinateur, puis dans une de ses briques.
    let pc = centre(&b, "pc");
    double(&mut b, pc);
    b.run_steps(240);
    enregistrer(&mut b, &dossier, "b6-dans-l-ordinateur");
    let p = {
        let app = b.state();
        let libre = app.disposition.unwrap().libre.shrink(30.0);
        app.atelier
            .doc
            .elements
            .iter()
            .filter(|e| e.genre == Genre::Brique && e.modele.contains("CPU"))
            .chain(
                app.atelier
                    .doc
                    .elements
                    .iter()
                    .filter(|e| e.genre == Genre::Brique),
            )
            .map(|e| app.plan.centre_ecran(&app.atelier, e.id))
            .find(|p| libre.contains(*p) && !app.plan.obstacles.iter().any(|r| r.contains(*p)))
            .expect("une brique visible")
    };
    double(&mut b, p);
    b.run_steps(240);
    enregistrer(&mut b, &dossier, "b7-un-niveau-plus-bas");
}

/// Un dossier de données de démonstration : quatre projets d'âges différents (le dernier ouvert
/// est une quête en cours) et neuf quêtes réussies.
fn dossier_de_demonstration(racine: &std::path::Path) -> ce_app::sauvegarde::Dossier {
    use std::time::{Duration, SystemTime};

    use ce_app::brique::Projet;
    use ce_app::document::Document;
    use ce_app::parcours::Parcours;
    use ce_app::quete::PARCOURS;
    use ce_app::sauvegarde::Dossier;

    let d = Dossier::dans(racine);
    // Quatre projets, d'âges différents, et neuf quêtes réussies.
    let maintenant = SystemTime::now();
    let projets: [(&str, Option<&str>, Document, u64); 4] = [
        (
            "Mon demi-additionneur",
            Some("demi-additionneur"),
            Document::demi_additionneur(),
            300,
        ),
        ("Ma porte NON", Some("non"), Document::nouveau(), 7_200),
        (
            "Essais de bascules",
            None,
            Document::demi_additionneur(),
            3 * 86_400,
        ),
        ("Brouillon", None, Document::nouveau(), 40 * 86_400),
    ];
    for (nom, quete, plan, age) in projets {
        let mut p = Projet::nouveau(nom, plan);
        p.quete = quete.map(str::to_owned);
        let f = d.enregistrer(&p).expect("projet");
        let fichier = std::fs::File::options()
            .write(true)
            .open(racine.join("projets").join(format!("{f}.json")))
            .expect("fichier");
        fichier
            .set_modified(maintenant - Duration::from_secs(age))
            .expect("date");
    }
    d.noter_dernier("Mon demi-additionneur").expect("dernier");
    let mut parcours = Parcours::default();
    for f in PARCOURS.iter().take(9) {
        parcours.faites.insert(f.cle.to_owned());
    }
    d.enregistrer_parcours(&parcours).expect("parcours");
    d
}

/// Un banc qui envoie chaque image rendue à ffmpeg.
struct Film {
    banc: Harness<'static, Application>,
    ffmpeg: std::process::Child,
    pointeur: egui::Pos2,
}

impl Film {
    /// Un film de 1440 × 900 à 60 images par seconde, écrit dans `sortie`.
    fn nouveau(app: Application, sortie: &str) -> Film {
        let banc = Harness::builder()
            .with_size(egui::Vec2::new(1440.0, 900.0))
            .with_pixels_per_point(1.0)
            .with_step_dt(1.0 / 60.0)
            .wgpu()
            .build_ui_state(|ui, app: &mut Application| app.image(ui), app);
        let ffmpeg = std::process::Command::new("ffmpeg")
            .args([
                "-y",
                "-loglevel",
                "error",
                "-f",
                "rawvideo",
                "-pix_fmt",
                "rgba",
                "-s",
                "1440x900",
                "-r",
                "60",
                "-i",
                "-",
            ])
            .args([
                "-c:v",
                "libx264",
                "-preset",
                "medium",
                "-crf",
                "20",
                "-pix_fmt",
                "yuv420p",
                "-movflags",
                "+faststart",
                sortie,
            ])
            .stdin(std::process::Stdio::piped())
            .spawn()
            .expect("ffmpeg doit être installé");
        Film {
            banc,
            ffmpeg,
            pointeur: egui::Pos2::new(700.0, 520.0),
        }
    }

    /// Ferme le film et attend que ffmpeg ait fini de l'écrire.
    fn finir(mut self) {
        drop(self.ffmpeg.stdin.take());
        assert!(self.ffmpeg.wait().expect("fin de ffmpeg").success());
    }

    /// Le centre d'une commande de l'écran affiché, retrouvée par son nom.
    fn repere(&self, nom: &str, rang: usize) -> egui::Pos2 {
        let app = self.banc.state();
        match app.ecran {
            ce_app::app::Ecran::Accueil => app.accueil.repere(nom, rang),
            ce_app::app::Ecran::Atelier => app.panneaux.repere(nom, rang),
        }
        .unwrap_or_else(|| panic!("la commande {nom} {rang} n'est pas affichée"))
        .center()
    }

    fn double_cliquer(&mut self, p: egui::Pos2, secondes: f32) {
        self.aller(p, secondes);
        for _ in 0..2 {
            self.appuyer();
            self.lacher();
        }
    }

    fn image(&mut self) {
        use std::io::Write;
        self.banc.step();
        let image = self.banc.render().expect("rendu");
        self.ffmpeg
            .stdin
            .as_mut()
            .expect("entrée de ffmpeg")
            .write_all(image.as_raw())
            .expect("envoi de l'image");
    }

    fn attendre(&mut self, secondes: f32) {
        for _ in 0..(secondes * 60.0) as usize {
            self.image();
        }
    }

    /// Déplace le pointeur jusqu'à `vers`, en douceur.
    fn aller(&mut self, vers: egui::Pos2, secondes: f32) {
        let de = self.pointeur;
        let n = (secondes * 60.0).max(1.0) as usize;
        for i in 1..=n {
            let t = i as f32 / n as f32;
            let doux = t * t * (3.0 - 2.0 * t);
            self.pointeur = de + (vers - de) * doux;
            self.banc.hover_at(self.pointeur);
            self.image();
        }
    }

    fn appuyer(&mut self) {
        self.banc.drag_at(self.pointeur);
        self.image();
    }

    fn lacher(&mut self) {
        self.banc.event(egui::Event::PointerButton {
            pos: self.pointeur,
            button: egui::PointerButton::Primary,
            pressed: false,
            modifiers: egui::Modifiers::NONE,
        });
        self.image();
    }

    fn cliquer(&mut self, p: egui::Pos2, secondes: f32) {
        self.aller(p, secondes);
        self.appuyer();
        self.attendre(0.08);
        self.lacher();
    }

    fn molette(&mut self, crans: f32, secondes: f32) {
        let n = (secondes * 60.0) as usize;
        for _ in 0..n {
            self.banc.event(egui::Event::MouseWheel {
                unit: egui::MouseWheelUnit::Point,
                delta: egui::Vec2::new(0.0, crans * 50.0 / n as f32),
                phase: egui::TouchPhase::Move,
                source: egui::MouseWheelSource::Unknown,
                modifiers: egui::Modifiers::NONE,
            });
            self.image();
        }
    }

    fn centre(&self, nom: &str) -> egui::Pos2 {
        self.banc
            .state()
            .plan
            .centre_ecran(&self.banc.state().atelier, id(&self.banc, nom))
    }

    fn port(&self, port: ce_app::plan::Port) -> egui::Pos2 {
        self.banc
            .state()
            .plan
            .port_ecran(&self.banc.state().atelier, port)
    }
}

#[test]
fn film() {
    use ce_app::document::{Fil, Point};
    use ce_app::plan::Port;
    use egui::{Pos2, Vec2};

    let Ok(sortie) = std::env::var("CE_FILM") else {
        eprintln!("CE_FILM n'est pas défini : film ignoré.");
        return;
    };
    let mut f = Film::nouveau(
        Application::nouvelle(Apparence::default(), Surveillance::default()),
        &sortie,
    );
    f.banc.run_steps(3);
    f.banc.hover_at(f.pointeur);

    // 1. Mise sous tension : le courant se propage jusqu'aux lampes.
    f.attendre(3.2);
    // 2. On bascule b : la nouvelle valeur parcourt les fils, les portes basculent tour à tour.
    let b = f.centre("b");
    f.cliquer(b, 0.7);
    f.attendre(2.6);
    // 3. On fait le tour des cas par la table de la quête.
    let d = f.banc.state().disposition.expect("disposition");
    let bas = d.quete.top() + f.banc.state().panneaux.contenus.quete - 16.0;
    f.cliquer(Pos2::new(d.quete.center().x, bas - 38.0 * 3.0 + 17.0), 0.7);
    f.attendre(2.4);
    f.cliquer(Pos2::new(d.quete.center().x, bas - 38.0 * 4.0 + 17.0), 0.4);
    f.attendre(2.4);
    // 4. On déplace une porte : elle se soulève, les fils suivent, elle s'aligne toute seule.
    let g4 = f.centre("g4");
    f.aller(g4, 0.7);
    f.appuyer();
    f.aller(g4 + Vec2::new(60.0, -90.0), 0.6);
    f.aller(g4 + Vec2::new(110.0, 70.0), 0.7);
    f.aller(g4 + Vec2::new(40.0, 3.0), 0.7);
    f.lacher();
    f.attendre(0.5);
    // 5. On recâble : on attrape le fil à son arrivée sur g5 et on le branche sur g3.
    let arrivee = f.port(Port::Entree(Fil {
        puits: id(&f.banc, "g5"),
        entree: 1,
    }));
    f.aller(arrivee, 0.7);
    f.appuyer();
    f.aller(arrivee + Vec2::new(-120.0, 60.0), 0.6);
    f.lacher();
    f.attendre(0.4);
    let sortie_g3 = f.port(Port::Sortie(Source::de(id(&f.banc, "g3"))));
    f.aller(sortie_g3 + Vec2::new(3.0, 0.0), 0.6);
    f.appuyer();
    f.aller(arrivee + Vec2::new(40.0, -50.0), 0.5);
    f.aller(arrivee + Vec2::new(-16.0, 10.0), 0.6);
    f.attendre(0.3);
    f.lacher();
    f.attendre(1.8);
    // 6. Annuler deux fois : le circuit d'origine revient.
    for _ in 0..2 {
        f.banc
            .key_press_modifiers(egui::Modifiers::COMMAND, egui::Key::Z);
        f.attendre(0.6);
    }
    f.attendre(1.2);
    // 7. Une nouvelle porte, tirée depuis la palette.
    let brique = Pos2::new(d.briques.center().x, d.briques.top() + 150.0);
    f.aller(brique, 0.7);
    f.appuyer();
    let cible = f.banc.state().plan.cam.ecran(Point::nouveau(250.0, 440.0));
    f.aller(cible, 0.9);
    f.lacher();
    f.attendre(0.6);
    // 8. Zoom à la molette autour du pointeur, déplacement du plan avec glissade, recadrage.
    let g3 = f.centre("g3");
    f.aller(g3 + Vec2::new(40.0, 46.0), 0.6);
    f.molette(5.0, 0.5);
    f.attendre(0.6);
    f.appuyer();
    let ici = f.pointeur;
    f.aller(ici + Vec2::new(-260.0, -90.0), 0.35);
    f.lacher();
    f.attendre(1.0);
    f.banc.key_press(egui::Key::F);
    f.attendre(1.0);
    // 9. Les panneaux se replient, le plan prend la place ; puis le mode clair.
    for i in [3.0, 2.0, 1.0, 0.0] {
        let bouton = Pos2::new(
            d.volets.left() + 8.0 + 40.0 + i * 102.0,
            d.volets.center().y,
        );
        f.cliquer(bouton, 0.35);
    }
    f.banc.key_press(egui::Key::F);
    f.attendre(1.2);
    f.cliquer(Pos2::new(d.volets.right() - 30.0, d.volets.center().y), 0.5);
    f.attendre(1.5);
    f.finir();
}

/// Le film des transitions : lancement, rubriques de l'accueil, thème, passage à l'atelier,
/// câblage et suppression, quête réussie, plongée dans les briques de l'ordinateur. Il ne
/// tourne que si `CE_FILM_TRANSITIONS` désigne le fichier à écrire.
#[test]
fn film_transitions() {
    use ce_app::document::{Fil, Genre, Point};
    use ce_app::plan::Port;
    use egui::{Pos2, Vec2};

    let Ok(sortie) = std::env::var("CE_FILM_TRANSITIONS") else {
        eprintln!("CE_FILM_TRANSITIONS n'est pas défini : film ignoré.");
        return;
    };
    let racine = std::env::temp_dir().join(format!("ce-film-transitions-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&racine);
    let dossier = dossier_de_demonstration(&racine);
    let app = Application::nouvelle(Apparence::default(), Surveillance::default())
        .avec_dossier(dossier)
        .au_lancement();
    let mut f = Film::nouveau(app, &sortie);
    f.banc.hover_at(f.pointeur);

    // 1. Lancement : l'accueil monte du fond.
    f.attendre(1.4);
    // 2. Les rubriques : la pastille du menu glisse, la page arrive pendant que l'autre s'efface.
    let p = f.repere("menu", 1);
    f.cliquer(p, 0.6);
    f.attendre(0.9);
    let p = f.repere("chapitre", 1);
    f.cliquer(p, 0.5);
    f.attendre(0.7);
    let p = f.repere("quete", ce_app::accueil::chapitres()[1].1[1]);
    f.cliquer(p, 0.5);
    f.attendre(0.8);
    let p = f.repere("menu", 2);
    f.cliquer(p, 0.5);
    f.attendre(0.9);
    let p = f.repere("menu", 3);
    f.cliquer(p, 0.5);
    f.attendre(0.8);
    // 3. Le thème : clair, puis de nouveau sombre, en fondu.
    let p = f.repere("reglage-theme", 1);
    f.cliquer(p, 0.6);
    f.attendre(1.1);
    let p = f.repere("reglage-theme", 0);
    f.cliquer(p, 0.4);
    f.attendre(1.0);
    let p = f.repere("menu", 0);
    f.cliquer(p, 0.5);
    f.attendre(0.9);
    // 4. Continuer : on s'enfonce dans l'atelier.
    let p = f.repere("continuer", 0);
    f.cliquer(p, 0.6);
    f.attendre(2.2);
    // 5. Une porte tirée de la palette, un fil qui se trace, puis tout s'efface à la suppression.
    let d = f.banc.state().disposition.expect("disposition");
    let brique = Pos2::new(d.briques.center().x, d.briques.top() + 150.0);
    f.aller(brique, 0.7);
    f.appuyer();
    let cible = f.banc.state().plan.cam.ecran(Point::nouveau(250.0, 440.0));
    f.aller(cible, 0.8);
    f.lacher();
    f.attendre(0.7);
    let porte = f
        .banc
        .state()
        .atelier
        .elements_selectionnes()
        .first()
        .copied()
        .expect("la porte posée est sélectionnée");
    let depart = f.port(Port::Sortie(Source::de(id(&f.banc, "b"))));
    f.aller(depart + Vec2::new(3.0, 0.0), 0.6);
    f.appuyer();
    let arrivee = f.port(Port::Entree(Fil {
        puits: porte,
        entree: 0,
    }));
    f.aller(arrivee + Vec2::new(-10.0, 4.0), 0.7);
    f.lacher();
    f.attendre(1.2);
    let centre = f
        .banc
        .state()
        .plan
        .centre_ecran(&f.banc.state().atelier, porte);
    f.cliquer(centre, 0.5);
    f.attendre(0.4);
    f.banc.key_press(egui::Key::Delete);
    f.attendre(1.2);
    // 6. Les quatre cas de la quête : au dernier, le plan fête la réussite.
    for ligne in 0..4 {
        let p = f.repere("cas", ligne);
        f.cliquer(p, 0.5);
        f.attendre(1.5);
    }
    f.attendre(2.4);
    // 7. La quête suivante s'ouvre en fondu.
    let p = f.repere("quete-suivante", 0);
    f.cliquer(p, 0.7);
    f.attendre(1.8);
    // 8. Retour à l'accueil, puis l'ordinateur miniature.
    let p = f.repere("marque", 0);
    f.cliquer(p, 0.6);
    f.attendre(1.0);
    let p = f.repere("menu", 2);
    f.cliquer(p, 0.5);
    f.attendre(0.8);
    let p = f.repere("exemple", 0);
    f.cliquer(p, 0.6);
    f.attendre(2.2);
    // 9. On plonge dans le processeur, puis un niveau plus bas, et on remonte.
    let pc = f.centre("pc");
    f.double_cliquer(pc, 0.7);
    f.attendre(1.8);
    let plus_bas = {
        let app = f.banc.state();
        let libre = app.disposition.unwrap().libre.shrink(30.0);
        app.atelier
            .doc
            .elements
            .iter()
            .filter(|e| e.genre == Genre::Brique)
            .map(|e| app.plan.centre_ecran(&app.atelier, e.id))
            .find(|p| libre.contains(*p) && !app.plan.obstacles.iter().any(|r| r.contains(*p)))
            .expect("une brique visible")
    };
    f.double_cliquer(plus_bas, 0.7);
    f.attendre(1.8);
    f.banc.key_press(egui::Key::Escape);
    f.attendre(1.6);
    f.banc.key_press(egui::Key::Escape);
    f.attendre(1.8);
    f.finir();
    let _ = std::fs::remove_dir_all(&racine);
}

/// Captures prises au milieu des transitions : ce qu'on voit pendant qu'elles jouent.
#[test]
fn captures_transitions() {
    use ce_app::document::Fil;
    use egui::{Pos2, Vec2};

    let Ok(sortie) = std::env::var("CE_CAPTURES") else {
        eprintln!("CE_CAPTURES n'est pas défini : captures ignorées.");
        return;
    };
    let racine =
        std::env::temp_dir().join(format!("ce-captures-transitions-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&racine);
    let dossier = dossier_de_demonstration(&racine);
    let app = Application::nouvelle(Apparence::default(), Surveillance::default())
        .avec_dossier(dossier)
        .au_lancement();
    let mut b = Harness::builder()
        .with_size(Vec2::new(1440.0, 900.0))
        .with_pixels_per_point(2.0)
        .with_step_dt(1.0 / 60.0)
        .wgpu()
        .build_ui_state(|ui, app: &mut Application| app.image(ui), app);
    let repere = |b: &Harness<'static, Application>, nom: &str, rang: usize| {
        let app = b.state();
        match app.ecran {
            ce_app::app::Ecran::Accueil => app.accueil.repere(nom, rang),
            ce_app::app::Ecran::Atelier => app.panneaux.repere(nom, rang),
        }
        .unwrap_or_else(|| panic!("{nom} {rang}"))
        .center()
    };
    // Un clic, puis `images` images de transition.
    let cliquer = |b: &mut Harness<'static, Application>, p: Pos2, images: usize| {
        b.hover_at(p);
        b.step();
        b.drag_at(p);
        b.step();
        b.drop_at(p);
        b.run_steps(1 + images);
    };

    b.run_steps(5);
    enregistrer(&mut b, &sortie, "t1-lancement");
    b.run_steps(120);
    let p = repere(&b, "menu", 1);
    cliquer(&mut b, p, 5);
    enregistrer(&mut b, &sortie, "t2-rubrique");
    b.run_steps(60);
    let p = repere(&b, "menu", 0);
    cliquer(&mut b, p, 60);
    let p = repere(&b, "continuer", 0);
    cliquer(&mut b, p, 8);
    enregistrer(&mut b, &sortie, "t3-vers-l-atelier");
    b.run_steps(300);

    // Une porte posée, un fil qui se trace, puis la suppression.
    let porte = {
        let app = b.state_mut();
        let porte = ce_app::document::Element::nand("essai");
        app.plan
            .poser_au_centre(&app.apparence, &mut app.atelier, porte)
    };
    b.run_steps(7);
    enregistrer(&mut b, &sortie, "t4-pose");
    b.run_steps(60);
    let source = Source::de(id(&b, "b"));
    {
        let app = b.state_mut();
        app.atelier.doc.relier(
            source,
            Fil {
                puits: porte,
                entree: 0,
            },
        );
        app.atelier.apres_modification(&app.apparence);
    }
    b.run_steps(9);
    enregistrer(&mut b, &sortie, "t5-fil-qui-se-trace");
    b.run_steps(60);
    {
        let app = b.state_mut();
        app.atelier.selectionner(porte);
        app.atelier.supprimer_selection(&app.apparence);
    }
    b.run_steps(6);
    enregistrer(&mut b, &sortie, "t6-suppression");
    b.run_steps(400);

    // La quête réussie : la fête sur le plan, la suite proposée dans le panneau.
    let p = repere(&b, "verifier", 0);
    cliquer(&mut b, p, 26);
    enregistrer(&mut b, &sortie, "t7-fete");
    b.run_steps(200);

    // Un autre projet, ouvert depuis la liste : les deux plans se fondent.
    let p = repere(&b, "projet", 2);
    cliquer(&mut b, p, 9);
    enregistrer(&mut b, &sortie, "t8-autre-projet");
    b.run_steps(120);

    // La plongée dans une brique de l'ordinateur, et la remontée.
    b.state_mut()
        .ouvrir(ce_app::panneaux::Ouverture::Exemple(0));
    b.run_steps(200);
    let pc = b
        .state()
        .plan
        .centre_ecran(&b.state().atelier, id(&b, "pc"));
    b.hover_at(pc);
    b.step();
    for _ in 0..2 {
        b.drag_at(pc);
        b.step();
        b.drop_at(pc);
        b.step();
    }
    b.run_steps(9);
    enregistrer(&mut b, &sortie, "t9-plongee");
    b.run_steps(200);
    b.key_press(egui::Key::Escape);
    b.run_steps(10);
    enregistrer(&mut b, &sortie, "t10-remontee");
    b.run_steps(120);
    let d = b.state().disposition.expect("disposition");
    cliquer(
        &mut b,
        Pos2::new(d.volets.right() - 30.0, d.volets.center().y),
        9,
    );
    enregistrer(&mut b, &sortie, "t11-theme");
    let _ = std::fs::remove_dir_all(&racine);
}

/// Captures du parcours des quêtes.
#[test]
fn captures_quetes() {
    use egui::Pos2;

    let Ok(dossier) = std::env::var("CE_CAPTURES") else {
        eprintln!("CE_CAPTURES n'est pas défini : captures ignorées.");
        return;
    };
    let cliquer = |b: &mut Harness<'static, Application>, p: Pos2| {
        b.hover_at(p);
        b.step();
        b.drag_at(p);
        b.step();
        b.drop_at(p);
        b.step();
    };
    let molette = |b: &mut Harness<'static, Application>, p: Pos2, crans: f32| {
        b.hover_at(p);
        b.step();
        b.event(egui::Event::MouseWheel {
            unit: egui::MouseWheelUnit::Point,
            delta: egui::Vec2::new(0.0, crans * 50.0),
            phase: egui::TouchPhase::Move,
            source: egui::MouseWheelSource::Unknown,
            modifiers: egui::Modifiers::NONE,
        });
        b.run_steps(90);
    };
    let mut b = banc(1440.0, 900.0, Apparence::default());
    b.run_steps(400);

    // Le demi-additionneur est juste : Vérifier le valide et donne sa brique.
    let verifier = b.state().panneaux.repere("verifier", 0).expect("Vérifier");
    cliquer(&mut b, verifier.center());
    b.run_steps(120);
    enregistrer(&mut b, &dossier, "q1-quete-reussie");

    // Plus bas, le parcours par chapitres.
    let panneau = b.state().disposition.expect("disposition").quete.center();
    molette(&mut b, panneau, -8.0);
    enregistrer(&mut b, &dossier, "q2-parcours");

    // La quête suivante s'ouvre avec la brique gagnée dans la palette.
    molette(&mut b, panneau, 40.0);
    let suivante = b
        .state()
        .panneaux
        .repere("quete-suivante", 0)
        .expect("quête suivante");
    cliquer(&mut b, suivante.center());
    b.run_steps(240);
    enregistrer(&mut b, &dossier, "q3-quete-suivante");
}

/// Capture du message qui propose une nouvelle version.
#[test]
fn captures_mise_a_jour() {
    use ce_app::etat::Proposition;

    let Ok(dossier) = std::env::var("CE_CAPTURES") else {
        eprintln!("CE_CAPTURES n'est pas défini : captures ignorées.");
        return;
    };
    let mut b = banc(1440.0, 900.0, Apparence::default());
    b.run_steps(400);
    b.state_mut().atelier.messages.clear();
    b.state_mut().atelier.proposer(
        "La version 0.2.0 de Computer Engineer est disponible : de nouvelles quêtes et un moteur plus rapide.",
        Proposition::MettreAJour,
    );
    b.run_steps(60);
    enregistrer(&mut b, &dossier, "m1-nouvelle-version");
    b.state_mut().atelier.messages.clear();
    b.state_mut().atelier.proposer(
        "La version 0.2.0 est installée et vérifiée. Elle démarre au prochain lancement, ou tout de suite.",
        Proposition::Redemarrer,
    );
    b.run_steps(60);
    enregistrer(&mut b, &dossier, "m2-redemarrer");
}

/// Captures de l'accueil : reprendre, parcours, projets, paramètres, clair, premier lancement,
/// petite fenêtre, et le chemin du retour depuis l'atelier.
#[test]
fn captures_accueil() {
    use ce_app::sauvegarde::Dossier;

    let Ok(sortie) = std::env::var("CE_CAPTURES") else {
        eprintln!("CE_CAPTURES n'est pas défini : captures ignorées.");
        return;
    };
    let racine = std::env::temp_dir().join(format!("ce-captures-accueil-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&racine);
    let d = dossier_de_demonstration(&racine);

    let lancer = |largeur: f32, hauteur: f32, apparence: Apparence| {
        let app = Application::nouvelle(apparence, Surveillance::default())
            .avec_dossier(Dossier::dans(&racine))
            .au_lancement();
        let mut b = Harness::builder()
            .with_size(egui::Vec2::new(largeur, hauteur))
            .with_pixels_per_point(2.0)
            .with_step_dt(1.0 / 60.0)
            .wgpu()
            .build_ui_state(|ui, app: &mut Application| app.image(ui), app);
        b.run_steps(120);
        b
    };
    let cliquer = |b: &mut Harness<'static, Application>, nom: &str, rang: usize| {
        let app = b.state();
        let p = match app.ecran {
            ce_app::app::Ecran::Accueil => app.accueil.repere(nom, rang),
            ce_app::app::Ecran::Atelier => app.panneaux.repere(nom, rang),
        }
        .unwrap_or_else(|| panic!("{nom} {rang}"))
        .center();
        b.hover_at(p);
        b.step();
        b.drag_at(p);
        b.step();
        b.drop_at(p);
        b.run_steps(40);
    };

    let mut b = lancer(1440.0, 900.0, Apparence::default());
    enregistrer(&mut b, &sortie, "a1-accueil");
    cliquer(&mut b, "menu", 1);
    cliquer(&mut b, "chapitre", 1);
    let quete = ce_app::accueil::chapitres()[1].1[1];
    cliquer(&mut b, "quete", quete);
    b.hover_at(egui::Pos2::new(10.0, 10.0));
    b.run_steps(30);
    enregistrer(&mut b, &sortie, "a2-parcours");
    cliquer(&mut b, "menu", 2);
    b.hover_at(egui::Pos2::new(10.0, 10.0));
    b.run_steps(30);
    enregistrer(&mut b, &sortie, "a3-projets");
    cliquer(&mut b, "supprimer", 3);
    b.hover_at(egui::Pos2::new(10.0, 10.0));
    b.run_steps(30);
    enregistrer(&mut b, &sortie, "a4-corbeille");
    cliquer(&mut b, "confirmer", 0);
    b.hover_at(egui::Pos2::new(10.0, 10.0));
    b.run_steps(30);
    enregistrer(&mut b, &sortie, "a5-annuler-la-corbeille");
    cliquer(&mut b, "restaurer", 0);
    cliquer(&mut b, "menu", 3);
    b.hover_at(egui::Pos2::new(10.0, 10.0));
    b.run_steps(30);
    enregistrer(&mut b, &sortie, "a6-parametres");
    // Retour au projet, puis à l'accueil par le nom de l'application.
    cliquer(&mut b, "menu", 0);
    cliquer(&mut b, "continuer", 0);
    b.run_steps(200);
    let marque = b.state().panneaux.repere("marque", 0).expect("marque");
    b.hover_at(marque.center());
    b.run_steps(30);
    enregistrer(&mut b, &sortie, "a7-atelier-retour-accueil");
    drop(b);

    let clair = Apparence {
        mode: ce_app::apparence::Mode::Clair,
        ..Apparence::default()
    };
    let mut b = lancer(1440.0, 900.0, clair);
    enregistrer(&mut b, &sortie, "a8-accueil-clair");
    drop(b);
    let mut b = lancer(1040.0, 660.0, Apparence::default());
    enregistrer(&mut b, &sortie, "a9-petite-fenetre");
    drop(b);

    // Texte « très grand » : rien ne doit déborder.
    let reglages = ce_app::reglages::Reglages {
        taille_texte: 1.3,
        ..ce_app::reglages::Reglages::default()
    };
    d.enregistrer_reglages(&reglages).expect("réglages");
    let mut b = lancer(1440.0, 900.0, Apparence::default());
    enregistrer(&mut b, &sortie, "a10-texte-tres-grand");
    cliquer(&mut b, "menu", 2);
    b.hover_at(egui::Pos2::new(10.0, 10.0));
    b.run_steps(30);
    enregistrer(&mut b, &sortie, "a11-texte-tres-grand-projets");
    // La barre du haut de l'atelier, texte très grand : les boutons perdent leurs libellés.
    cliquer(&mut b, "menu", 0);
    cliquer(&mut b, "continuer", 0);
    b.state_mut().atelier.vue_t = Some(0);
    b.run_steps(200);
    enregistrer(&mut b, &sortie, "a13-atelier-texte-tres-grand");
    drop(b);
    // Fenêtre la plus petite, texte très grand : le nom et la vitesse se réduisent à leur icône.
    let mut b = lancer(1040.0, 660.0, Apparence::default());
    cliquer(&mut b, "continuer", 0);
    b.state_mut().atelier.vue_t = Some(0);
    b.run_steps(200);
    enregistrer(
        &mut b,
        &sortie,
        "a14-atelier-petite-fenetre-texte-tres-grand",
    );
    // Le pire cas : le passé d'un circuit qui tourne depuis longtemps. L'état s'abrège.
    b.state_mut().atelier.vue_t = Some(12_345_670);
    b.run_steps(30);
    enregistrer(&mut b, &sortie, "a16-atelier-temps-tres-long");
    drop(b);
    d.enregistrer_reglages(&ce_app::reglages::Reglages::default())
        .expect("réglages");
    // Écran de portable courant, texte normal.
    let mut b = lancer(1366.0, 768.0, Apparence::default());
    cliquer(&mut b, "continuer", 0);
    b.state_mut().atelier.vue_t = Some(0);
    b.run_steps(200);
    enregistrer(&mut b, &sortie, "a15-atelier-1366");
    drop(b);

    // Premier lancement : rien à reprendre, la première quête est proposée.
    let _ = std::fs::remove_dir_all(&racine);
    let mut b = lancer(1440.0, 900.0, Apparence::default());
    enregistrer(&mut b, &sortie, "a12-premier-lancement");
    drop(b);
    let _ = std::fs::remove_dir_all(&racine);
}

/// L'éditeur de programme de la mémoire morte (Ctrl+P) et l'aide (F1).
#[test]
fn captures_programme_et_aide() {
    let Ok(dossier) = std::env::var("CE_CAPTURES") else {
        eprintln!("CE_CAPTURES n'est pas défini : captures ignorées.");
        return;
    };
    let mut b = banc(1440.0, 900.0, Apparence::default());
    b.run_steps(30);
    let ex = &ce_app::exemples::EXEMPLES[0];
    {
        let app = b.state_mut();
        let a = Apparence::default();
        app.atelier.ouvrir_exemple(ex, ex.nom, &a);
        app.ecran = ce_app::app::Ecran::Atelier;
    }
    b.run_steps(120);
    b.key_press_modifiers(egui::Modifiers::COMMAND, egui::Key::P);
    b.run_steps(20);
    enregistrer(&mut b, &dossier, "p1-editeur-de-programme");
    {
        let e = b.state_mut().fenetres.editeur.as_mut().unwrap();
        e.texte.push_str("\n        SAUTE nulle_part\n");
        e.essayer();
    }
    b.run_steps(5);
    enregistrer(&mut b, &dossier, "p2-erreur-d-assemblage");
    b.state_mut().fenetres.editeur = None;
    b.key_press(egui::Key::F1);
    b.run_steps(20);
    enregistrer(&mut b, &dossier, "p3-aide");
    b.state_mut().fenetres.aide = Some(ce_app::fenetres::Rubrique::Programmer);
    b.run_steps(5);
    enregistrer(&mut b, &dossier, "p4-aide-programmer");
    b.state_mut().fenetres.aide = Some(ce_app::fenetres::Rubrique::APropos);
    b.run_steps(5);
    enregistrer(&mut b, &dossier, "p5-a-propos");
}

/// Captures des branches du parcours, de la fiche « Comprendre » et des fêtes de réussite.
#[test]
fn captures_branches() {
    use ce_app::quete::PARCOURS;
    use ce_app::sauvegarde::Dossier;
    use egui::Pos2;

    let Ok(sortie) = std::env::var("CE_CAPTURES") else {
        eprintln!("CE_CAPTURES n'est pas défini : captures ignorées.");
        return;
    };
    let rang = |cle: &str| PARCOURS.iter().position(|f| f.cle == cle).expect("quête");
    let cliquer = |b: &mut Harness<'static, Application>, p: Pos2| {
        b.hover_at(p);
        b.step();
        b.drag_at(p);
        b.step();
        b.drop_at(p);
        b.step();
    };

    // L'accueil, page Parcours : une branche, puis un chapitre du parcours principal.
    let racine = std::env::temp_dir().join(format!("ce-captures-branches-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&racine);
    let _ = dossier_de_demonstration(&racine);
    let app = Application::nouvelle(Apparence::default(), Surveillance::default())
        .avec_dossier(Dossier::dans(&racine))
        .au_lancement();
    let mut b = Harness::builder()
        .with_size(egui::Vec2::new(1440.0, 900.0))
        .with_pixels_per_point(2.0)
        .with_step_dt(1.0 / 60.0)
        .wgpu()
        .build_ui_state(|ui, app: &mut Application| app.image(ui), app);
    b.run_steps(120);
    let menu = b.state().accueil.repere("menu", 1).expect("menu").center();
    cliquer(&mut b, menu);
    b.run_steps(30);
    b.state_mut().accueil.montrer_quete(rang("carte-reseau"));
    b.hover_at(Pos2::new(10.0, 10.0));
    b.run_steps(60);
    enregistrer(&mut b, &sortie, "r1-branche-reseau");
    b.state_mut().accueil.montrer_quete(rang("pgcd"));
    b.run_steps(60);
    enregistrer(&mut b, &sortie, "r2-branche-maths");
    b.state_mut()
        .accueil
        .montrer_quete(rang("additionneur-rapide"));
    b.run_steps(60);
    enregistrer(&mut b, &sortie, "r3-chapitre-calcul");
    for (cle, nom) in [
        ("hamming-correcteur", "r8-branche-codes"),
        ("morpion", "r9-branche-jeux"),
        ("binaire-decimal", "r10-nombres-remarquables"),
        ("messagerie-chiffree", "r11-grands-projets"),
    ] {
        b.state_mut().accueil.montrer_quete(rang(cle));
        b.run_steps(60);
        enregistrer(&mut b, &sortie, nom);
    }
    let _ = std::fs::remove_dir_all(&racine);

    // L'atelier d'un grand projet : deux ordinateurs à relier.
    let mut b = banc(1440.0, 900.0, Apparence::default());
    let fiche = PARCOURS
        .iter()
        .find(|f| f.cle == "messagerie")
        .expect("messagerie");
    assert!(b
        .state_mut()
        .atelier
        .ouvrir_quete(fiche, "Messagerie", &Apparence::default()));
    b.run_steps(200);
    enregistrer(&mut b, &sortie, "r12-atelier-messagerie");

    // L'atelier : la quête du demi-additionneur, ses étapes et sa fiche « Comprendre ».
    let mut b = banc(1440.0, 900.0, Apparence::default());
    // Toutes les autres quêtes du chapitre Calcul sont faites : celle-ci le termine.
    for f in PARCOURS
        .iter()
        .filter(|f| f.chapitre == "Calcul" && f.cle != "demi-additionneur")
    {
        b.state_mut()
            .atelier
            .parcours
            .faites
            .insert(f.cle.to_owned());
    }
    b.run_steps(400);
    enregistrer(&mut b, &sortie, "r4-quete-et-etapes");
    let comprendre = b
        .state()
        .panneaux
        .repere("comprendre", 0)
        .expect("Comprendre");
    cliquer(&mut b, comprendre.center());
    b.run_steps(40);
    enregistrer(&mut b, &sortie, "r4b-comprendre-deplie");
    cliquer(&mut b, comprendre.center());
    b.run_steps(40);
    let verifier = b.state().panneaux.repere("verifier", 0).expect("Vérifier");
    cliquer(&mut b, verifier.center());
    b.run_steps(14);
    enregistrer(&mut b, &sortie, "r5-badge-qui-arrive");
    b.run_steps(36);
    enregistrer(&mut b, &sortie, "r6-chapitre-termine");
    b.run_steps(240);
    enregistrer(&mut b, &sortie, "r7-apres-la-fete");
}

/// Captures de la construction minimale : l'objectif avant de vérifier, les trois étoiles
/// d'une construction minimale, deux étoiles avec une porte de trop, la fiche d'un plan hors
/// quête, puis les étoiles dans la liste du parcours.
#[test]
fn captures_construction_minimale() {
    use egui::Pos2;

    let Ok(dossier) = std::env::var("CE_CAPTURES") else {
        eprintln!("CE_CAPTURES n'est pas défini : captures ignorées.");
        return;
    };
    let cliquer = |b: &mut Harness<'static, Application>, p: Pos2| {
        b.hover_at(p);
        b.step();
        b.drag_at(p);
        b.step();
        b.drop_at(p);
        b.step();
    };
    let molette = |b: &mut Harness<'static, Application>, p: Pos2, crans: f32| {
        b.hover_at(p);
        b.step();
        b.event(egui::Event::MouseWheel {
            unit: egui::MouseWheelUnit::Point,
            delta: egui::Vec2::new(0.0, crans * 50.0),
            phase: egui::TouchPhase::Move,
            source: egui::MouseWheelSource::Unknown,
            modifiers: egui::Modifiers::NONE,
        });
        b.run_steps(90);
    };
    let mut b = banc(1440.0, 900.0, Apparence::default());
    b.run_steps(400);
    enregistrer(&mut b, &dossier, "c1-objectif");

    let verifier = b.state().panneaux.repere("verifier", 0).expect("Vérifier");
    cliquer(&mut b, verifier.center());
    b.run_steps(200);
    enregistrer(&mut b, &dossier, "c2-minimale");

    // Une porte inutile en plus : la construction reste juste, mais n'est plus minimale.
    {
        let app = b.state_mut();
        app.atelier
            .poser_porte(ce_app::document::Point::nouveau(520.0, 620.0));
        app.atelier.apres_modification(&Apparence::default());
    }
    b.run_steps(30);
    let verifier = b.state().panneaux.repere("verifier", 0).expect("Vérifier");
    cliquer(&mut b, verifier.center());
    b.run_steps(200);
    enregistrer(&mut b, &dossier, "c3-une-porte-de-trop");

    // Les étoiles des quêtes réussies, dans la liste du parcours.
    let panneau = b.state().disposition.expect("disposition").quete.center();
    molette(&mut b, panneau, -20.0);
    enregistrer(&mut b, &dossier, "c4-parcours-etoiles");

    // Hors quête : la fiche du plan le compare à la fonction qu'il calcule.
    b.state_mut().atelier.quete = None;
    b.state_mut().panneaux.defilement_quete = 0.0;
    b.run_steps(60);
    enregistrer(&mut b, &dossier, "c5-fiche-du-plan");
}

/// La solution d'une quête : le bouton « Voir la solution », puis quelques solutions ouvertes.
#[test]
fn captures_solutions() {
    let Ok(dossier) = std::env::var("CE_CAPTURES") else {
        eprintln!("CE_CAPTURES n'est pas défini : captures ignorées.");
        return;
    };
    let mut b = banc(1440.0, 900.0, Apparence::default());
    b.run_steps(400);
    // Le bouton est sous la vérification : on fait défiler la fiche de la quête.
    let panneau = b.state().disposition.expect("disposition").quete.center();
    b.hover_at(panneau);
    b.step();
    for _ in 0..3 {
        b.event(egui::Event::MouseWheel {
            unit: egui::MouseWheelUnit::Point,
            delta: egui::Vec2::new(0.0, -100.0),
            phase: egui::TouchPhase::Move,
            source: egui::MouseWheelSource::Unknown,
            modifiers: egui::Modifiers::NONE,
        });
        b.run_steps(30);
    }
    b.run_steps(90);
    let bouton = b
        .state()
        .panneaux
        .repere("voir-la-solution", 0)
        .expect("Voir la solution");
    b.hover_at(bouton.center());
    b.run_steps(30);
    enregistrer(&mut b, &dossier, "s1-bouton");
    b.drag_at(bouton.center());
    b.step();
    b.drop_at(bouton.center());
    b.run_steps(300);
    enregistrer(&mut b, &dossier, "s2-solution-non");
    for (cle, nom) in [
        ("premier", "s3-solution-premier"),
        ("encodeur", "s4-solution-encodeur"),
        ("ordinateur", "s5-solution-ordinateur"),
    ] {
        b.state_mut()
            .ouvrir(ce_app::panneaux::Ouverture::Solution(cle.to_owned()));
        b.run_steps(300);
        enregistrer(&mut b, &dossier, nom);
    }
}

/// Fils qui se croisent et se superposent : quatre interrupteurs lus dans le désordre par une
/// colonne de portes, puis deux portes qui croisent leurs entrées. `f1` au repos, `f2` en
/// survolant un fil (tout son signal s'allume), `f3` de près, `f4` en mode clair.
#[test]
fn captures_fils_croises() {
    use ce_app::brique::Projet;
    use ce_app::document::{Document, Element, Fil, Point};

    let Ok(dossier) = std::env::var("CE_CAPTURES") else {
        eprintln!("CE_CAPTURES n'est pas défini : captures ignorées.");
        return;
    };
    let mut plan = Document::nouveau();
    let entree = |plan: &mut Document, nom: &str, y: f32| {
        plan.poser(Element::entree(nom, 1), Point::nouveau(61.0, y))
    };
    let a = entree(&mut plan, "a", 120.0);
    let bb = entree(&mut plan, "b", 210.0);
    let c = entree(&mut plan, "c", 300.0);
    let d = entree(&mut plan, "d", 390.0);
    let porte =
        |plan: &mut Document, x: f32, y: f32| plan.poser(Element::nand(""), Point::nouveau(x, y));
    let g1 = porte(&mut plan, 330.0, 120.0);
    let g2 = porte(&mut plan, 330.0, 210.0);
    let g3 = porte(&mut plan, 330.0, 300.0);
    let g4 = porte(&mut plan, 330.0, 390.0);
    let g5 = porte(&mut plan, 560.0, 165.0);
    let g6 = porte(&mut plan, 560.0, 345.0);
    let s1 = plan.poser(Element::sortie("s1", "", 1), Point::nouveau(760.0, 165.0));
    let s2 = plan.poser(Element::sortie("s2", "", 1), Point::nouveau(760.0, 345.0));
    for (puits, entree, source) in [
        (g1, 0, a),
        (g1, 1, c),
        (g2, 0, bb),
        (g2, 1, d),
        (g3, 0, a),
        (g3, 1, d),
        (g4, 0, c),
        (g4, 1, bb),
        (g5, 0, g1),
        (g5, 1, g4),
        (g6, 0, g2),
        (g6, 1, g3),
        (s1, 0, g5),
        (s2, 0, g6),
    ] {
        assert!(plan.relier(Source::de(source), Fil { puits, entree }));
    }
    let ouvrir = |b: &mut Harness<'static, Application>, plan: Document| {
        let app = b.state_mut();
        let ap = app.apparence.clone();
        app.atelier
            .ouvrir(Projet::nouveau("Fils croisés", plan), &ap);
    };
    let mut clair = banc(
        1440.0,
        900.0,
        Apparence {
            mode: Mode::Clair,
            ..Apparence::default()
        },
    );
    ouvrir(&mut clair, plan.clone());
    clair.run_steps(300);
    enregistrer(&mut clair, &dossier, "f4-clair");
    drop(clair);

    let mut b = banc(1440.0, 900.0, Apparence::default());
    ouvrir(&mut b, plan);
    b.run_steps(300);
    enregistrer(&mut b, &dossier, "f1-fils-croises");

    // Survoler le dernier tronçon du fil de a vers g3.
    let (c1, c3) = {
        let app = b.state();
        (
            app.plan.centre_ecran(&app.atelier, g1),
            app.plan.centre_ecran(&app.atelier, g3),
        )
    };
    let k = (c3.y - c1.y) / 180.0;
    let p = c3 + egui::Vec2::new(-90.0, -15.0) * k;
    b.hover_at(p);
    b.run_steps(40);
    enregistrer(&mut b, &dossier, "f2-survol-d-un-signal");

    // De près : molette vers l'avant au croisement des entrées.
    let milieu = c1 + (c3 - c1) * 0.5 + egui::Vec2::new(-100.0 * k, 0.0);
    b.hover_at(milieu);
    b.step();
    b.event(egui::Event::MouseWheel {
        unit: egui::MouseWheelUnit::Point,
        delta: egui::Vec2::new(0.0, 6.0 * 50.0),
        phase: egui::TouchPhase::Move,
        source: egui::MouseWheelSource::Unknown,
        modifiers: egui::Modifiers::NONE,
    });
    b.run_steps(90);
    enregistrer(&mut b, &dossier, "f3-de-pres");
}
