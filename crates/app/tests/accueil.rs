//! L'écran d'accueil, piloté comme par une souris et un clavier, sans fenêtre : reprendre le
//! dernier projet, choisir une quête, gérer ses projets, changer les paramètres.

use std::path::{Path, PathBuf};

use ce_app::accueil::Rubrique;
use ce_app::app::{Application, Ecran};
use ce_app::apparence::{Apparence, Mode, Surveillance};
use ce_app::brique::Projet;
use ce_app::document::Document;
use ce_app::panneaux::Ouverture;
use ce_app::reglages::Lancement;
use ce_app::sauvegarde::Dossier;
use egui::{Pos2, Vec2};
use egui_kittest::Harness;

type Banc = Harness<'static, Application>;

/// Un dossier de données neuf pour un essai.
fn dossier(nom: &str) -> PathBuf {
    let racine = std::env::temp_dir().join(format!("ce-accueil-{nom}-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&racine);
    racine
}

/// L'application telle qu'au lancement, avec ce dossier de données.
fn lancer(racine: &Path, largeur: f32, hauteur: f32) -> Banc {
    let app = Application::nouvelle(Apparence::default(), Surveillance::default())
        .avec_dossier(Dossier::dans(racine))
        .au_lancement();
    let mut b = Harness::builder()
        .with_size(Vec2::new(largeur, hauteur))
        .with_pixels_per_point(1.0)
        .with_step_dt(1.0 / 60.0)
        .build_ui_state(|ui, app: &mut Application| app.image(ui), app);
    b.run_steps(60);
    b
}

/// Deux projets enregistrés, le second ouvert en dernier.
fn deux_projets(racine: &Path) {
    let d = Dossier::dans(racine);
    let mut quete = Projet::nouveau("Ma porte NON", Document::nouveau());
    quete.quete = Some("non".to_owned());
    d.enregistrer(&quete).unwrap();
    let f = d
        .enregistrer(&Projet::nouveau(
            "Additionneur",
            Document::demi_additionneur(),
        ))
        .unwrap();
    d.noter_dernier(&f).unwrap();
}

/// Le centre d'une commande de l'écran affiché.
fn repere(b: &Banc, nom: &str, rang: usize) -> Pos2 {
    let app = b.state();
    match app.ecran {
        Ecran::Accueil => app.accueil.repere(nom, rang),
        Ecran::Atelier => app.panneaux.repere(nom, rang),
    }
    .unwrap_or_else(|| panic!("la commande {nom} {rang} n'est pas affichée"))
    .center()
}

fn cliquer(b: &mut Banc, nom: &str, rang: usize) {
    let p = repere(b, nom, rang);
    b.hover_at(p);
    b.step();
    b.drag_at(p);
    b.step();
    b.drop_at(p);
    b.run_steps(20);
}

fn taper(b: &mut Banc, texte: &str) {
    for c in texte.chars() {
        b.event(egui::Event::Text(c.to_string()));
        b.step();
    }
}

fn touche(b: &mut Banc, k: egui::Key) {
    b.key_press(k);
    b.run_steps(10);
}

#[test]
fn au_lancement_l_accueil_propose_de_reprendre_le_dernier_projet() {
    let racine = dossier("reprendre");
    deux_projets(&racine);
    let mut b = lancer(&racine, 1440.0, 900.0);
    assert_eq!(b.state().ecran, Ecran::Accueil);
    assert_eq!(b.state().accueil.rubrique, Rubrique::Accueil);
    assert_eq!(b.state().atelier.projet.nom, "Additionneur");
    // Le projet récent qui n'est pas celui à reprendre est proposé.
    repere(&b, "projet-recent", 0);
    repere(&b, "nouveau", 0);
    // Entrée : on reprend là où on en était.
    touche(&mut b, egui::Key::Enter);
    assert_eq!(b.state().ecran, Ecran::Atelier);
    assert_eq!(b.state().atelier.projet.nom, "Additionneur");

    // Le nom de l'application ramène à l'accueil ; Échap y ramène au projet.
    cliquer(&mut b, "marque", 0);
    assert_eq!(b.state().ecran, Ecran::Accueil);
    touche(&mut b, egui::Key::Escape);
    assert_eq!(b.state().ecran, Ecran::Atelier);

    // Le bouton « Continuer » fait de même.
    cliquer(&mut b, "marque", 0);
    cliquer(&mut b, "continuer", 0);
    assert_eq!(b.state().ecran, Ecran::Atelier);

    // Un projet récent s'ouvre d'un clic.
    cliquer(&mut b, "marque", 0);
    cliquer(&mut b, "projet-recent", 0);
    assert_eq!(b.state().ecran, Ecran::Atelier);
    assert_eq!(b.state().atelier.projet.nom, "Ma porte NON");
    assert_eq!(b.state().atelier.quete.as_ref().map(|q| q.cle), Some("non"));
    let _ = std::fs::remove_dir_all(&racine);
}

#[test]
fn au_premier_lancement_l_accueil_mene_a_la_premiere_quete() {
    let racine = dossier("premier");
    let mut b = lancer(&racine, 1440.0, 900.0);
    assert_eq!(b.state().ecran, Ecran::Accueil);
    assert!(
        b.state().accueil.repere("continuer", 0).is_none(),
        "rien à reprendre"
    );
    assert!(
        b.state().atelier.messages.is_empty(),
        "pas d'astuce du plan sur l'accueil"
    );
    cliquer(&mut b, "commencer-le-parcours", 0);
    assert_eq!(b.state().ecran, Ecran::Atelier);
    assert_eq!(b.state().atelier.quete.as_ref().map(|q| q.cle), Some("non"));
    // Le projet de la quête est enregistré et devient le projet à reprendre.
    assert_eq!(
        Dossier::dans(&racine).dernier().as_deref(),
        Some("La porte NON")
    );
    let _ = std::fs::remove_dir_all(&racine);
}

#[test]
fn le_parcours_se_parcourt_par_chapitre_et_une_quete_s_ouvre() {
    let racine = dossier("parcours");
    deux_projets(&racine);
    let mut b = lancer(&racine, 1440.0, 900.0);
    cliquer(&mut b, "menu", 1);
    assert_eq!(b.state().accueil.rubrique, Rubrique::Parcours);
    // Par défaut : le chapitre et la quête conseillés (la porte NON, rang 0).
    assert_eq!(b.state().accueil.chapitre, 0);
    assert_eq!(b.state().accueil.quete, Some(0));
    // La porte NON a déjà un projet : on peut le reprendre.
    repere(&b, "reprendre", 0);

    // Deuxième chapitre, deuxième quête de ce chapitre.
    cliquer(&mut b, "chapitre", 1);
    assert_eq!(b.state().accueil.chapitre, 1);
    let premiere = ce_app::accueil::chapitres()[1].1[1];
    cliquer(&mut b, "quete", premiere);
    assert_eq!(b.state().accueil.quete, Some(premiere));
    assert!(
        b.state().accueil.repere("reprendre", 0).is_none(),
        "aucun projet pour cette quête"
    );
    cliquer(&mut b, "commencer", 0);
    assert_eq!(b.state().ecran, Ecran::Atelier);
    let cle = ce_app::quete::PARCOURS[premiere].cle;
    assert_eq!(b.state().atelier.quete.as_ref().map(|q| q.cle), Some(cle));

    // Échap revient de la rubrique à l'accueil.
    cliquer(&mut b, "marque", 0);
    cliquer(&mut b, "menu", 1);
    touche(&mut b, egui::Key::Escape);
    assert_eq!(b.state().accueil.rubrique, Rubrique::Accueil);
    assert_eq!(b.state().ecran, Ecran::Accueil);
    let _ = std::fs::remove_dir_all(&racine);
}

#[test]
fn les_projets_se_renomment_se_dupliquent_et_vont_a_la_corbeille() {
    let racine = dossier("projets");
    deux_projets(&racine);
    let d = Dossier::dans(&racine);
    let mut b = lancer(&racine, 1440.0, 900.0);
    cliquer(&mut b, "menu", 2);
    assert_eq!(b.state().accueil.rubrique, Rubrique::Projets);
    assert_eq!(b.state().projets.len(), 2);
    let rang = |b: &Banc, fichier: &str| {
        b.state()
            .projets
            .iter()
            .position(|f| f.fichier == fichier)
            .unwrap_or_else(|| panic!("{fichier} n'est pas listé"))
    };

    // Renommer un projet qui n'est pas ouvert.
    let i = rang(&b, "Ma porte NON");
    cliquer(&mut b, "renommer", i);
    assert!(b.state().accueil.en_saisie());
    taper(&mut b, "Porte NON finie");
    touche(&mut b, egui::Key::Enter);
    assert!(!b.state().accueil.en_saisie());
    assert!(d.existe("Porte NON finie") && !d.existe("Ma porte NON"));
    // Échap abandonne une saisie.
    let i = rang(&b, "Porte NON finie");
    cliquer(&mut b, "renommer", i);
    taper(&mut b, "autre");
    touche(&mut b, egui::Key::Escape);
    assert!(d.existe("Porte NON finie"));
    assert_eq!(b.state().accueil.rubrique, Rubrique::Projets);

    // Renommer le projet ouvert : l'atelier suit.
    let i = rang(&b, "Additionneur");
    cliquer(&mut b, "renommer", i);
    taper(&mut b, "Demi-additionneur");
    touche(&mut b, egui::Key::Enter);
    assert_eq!(b.state().atelier.projet.nom, "Demi-additionneur");
    assert!(d.existe("Demi-additionneur") && !d.existe("Additionneur"));
    assert_eq!(d.dernier().as_deref(), Some("Demi-additionneur"));

    // Dupliquer.
    let i = rang(&b, "Porte NON finie");
    cliquer(&mut b, "dupliquer", i);
    assert!(d.existe("Porte NON finie (copie)"));
    assert_eq!(b.state().projets.len(), 3);

    // La corbeille demande confirmation ; « Garder » renonce.
    let i = rang(&b, "Porte NON finie (copie)");
    cliquer(&mut b, "supprimer", i);
    cliquer(&mut b, "garder", 0);
    assert!(d.existe("Porte NON finie (copie)"));
    cliquer(&mut b, "supprimer", i);
    cliquer(&mut b, "confirmer", 0);
    assert!(!d.existe("Porte NON finie (copie)"));
    assert_eq!(b.state().projets.len(), 2);
    // « Annuler » dans le message le ramène.
    cliquer(&mut b, "restaurer", 0);
    assert!(d.existe("Porte NON finie (copie)"));
    assert_eq!(b.state().projets.len(), 3);

    // Mettre à la corbeille le projet ouvert : il n'est plus proposé à la reprise.
    let i = rang(&b, "Demi-additionneur");
    cliquer(&mut b, "supprimer", i);
    cliquer(&mut b, "confirmer", 0);
    assert!(!d.existe("Demi-additionneur"));
    assert_eq!(b.state().courant(), None);
    b.run_steps(120);
    assert!(
        !d.existe("Demi-additionneur"),
        "le projet jeté n'est pas réécrit par l'enregistrement automatique"
    );
    cliquer(&mut b, "menu", 0);
    assert!(b.state().accueil.repere("continuer", 0).is_none());
    // Et « Annuler » le rend de nouveau reprenable.
    cliquer(&mut b, "restaurer", 0);
    assert_eq!(b.state().courant().as_deref(), Some("Demi-additionneur"));
    b.run_steps(5);
    repere(&b, "continuer", 0);

    // Les exemples s'ouvrent depuis la liste.
    cliquer(&mut b, "menu", 2);
    cliquer(&mut b, "exemple", 0);
    assert_eq!(b.state().ecran, Ecran::Atelier);
    assert_eq!(b.state().atelier.projet.nom, "Ordinateur miniature");
    let _ = std::fs::remove_dir_all(&racine);
}

#[test]
fn un_projet_illisible_au_lancement_n_est_ni_ecrase_ni_passe_sous_silence() {
    let racine = dossier("illisible");
    let d = Dossier::dans(&racine);
    // Le premier projet de l'utilisateur porte le nom du plan de démonstration.
    let premier = d
        .enregistrer(&Projet::nouveau("Premier circuit", Document::nouveau()))
        .unwrap();
    let avant = std::fs::read(racine.join("projets").join("Premier circuit.json")).unwrap();
    // Le dernier projet ouvert ne se relit plus (écrit par une autre version, abîmé…).
    std::fs::write(
        racine.join("projets").join("Mon processeur.json"),
        "{ abîmé",
    )
    .unwrap();
    d.noter_dernier("Mon processeur").unwrap();

    let mut b = lancer(&racine, 1440.0, 900.0);
    assert_eq!(b.state().ecran, Ecran::Accueil);
    assert!(
        b.state()
            .atelier
            .messages
            .iter()
            .any(|m| m.texte.contains("n'a pas pu être rouvert")),
        "l'erreur reste affichée sur l'accueil"
    );
    // Ouvrir autre chose n'écrit pas le plan de démonstration par-dessus un projet.
    cliquer(&mut b, "nouveau", 0);
    assert_eq!(b.state().ecran, Ecran::Atelier);
    assert_eq!(
        std::fs::read(racine.join("projets").join("Premier circuit.json")).unwrap(),
        avant
    );
    assert_eq!(
        std::fs::read_to_string(racine.join("projets").join("Mon processeur.json")).unwrap(),
        "{ abîmé"
    );
    assert!(d.existe(&premier));
    let _ = std::fs::remove_dir_all(&racine);
}

/// Le nombre d'éléments du plan principal d'un projet enregistré.
fn elements(d: &Dossier, fichier: &str) -> usize {
    d.ouvrir(fichier).unwrap().principal.elements.len()
}

#[test]
fn un_exemple_n_est_ecrit_que_modifie_et_n_ecrase_pas_celui_qu_on_a_modifie() {
    let racine = dossier("exemple");
    let d = Dossier::dans(&racine);
    let projets = racine.join("projets");
    let mut b = lancer(&racine, 1440.0, 900.0);
    // Regarder l'exemple deux fois ne laisse aucune copie sur le disque.
    for _ in 0..2 {
        cliquer(&mut b, "menu", 2);
        cliquer(&mut b, "exemple", 0);
        assert_eq!(b.state().ecran, Ecran::Atelier);
        assert_eq!(b.state().atelier.projet.nom, "Ordinateur miniature");
        b.run_steps(240);
        cliquer(&mut b, "marque", 0);
        assert!(d.lister().is_empty(), "un exemple intact n'est pas écrit");
    }
    // On peut y revenir depuis l'accueil, et le modifier l'enregistre.
    repere(&b, "continuer", 0);
    cliquer(&mut b, "continuer", 0);
    let intact = b.state().atelier.projet.principal.elements.len();
    cliquer(&mut b, "nand", 0);
    b.state_mut().enregistrer();
    assert_eq!(d.lister(), ["Ordinateur miniature"]);
    assert_eq!(elements(&d, "Ordinateur miniature"), intact + 1);
    let modifie = std::fs::read(projets.join("Ordinateur miniature.json")).unwrap();
    // Rouvrir l'exemple donne une copie neuve sous un autre nom : l'ancienne reste.
    cliquer(&mut b, "marque", 0);
    cliquer(&mut b, "menu", 2);
    cliquer(&mut b, "exemple", 0);
    assert_eq!(b.state().atelier.projet.nom, "Ordinateur miniature 2");
    assert_eq!(d.lister(), ["Ordinateur miniature"]);
    cliquer(&mut b, "nand", 0);
    cliquer(&mut b, "nand", 0);
    b.state_mut().enregistrer();
    assert_eq!(
        d.lister(),
        ["Ordinateur miniature", "Ordinateur miniature 2"]
    );
    assert_eq!(elements(&d, "Ordinateur miniature 2"), intact + 2);
    assert_eq!(
        std::fs::read(projets.join("Ordinateur miniature.json")).unwrap(),
        modifie,
        "la première copie garde son contenu, différent de celui de la seconde"
    );
    assert_eq!(elements(&d, "Ordinateur miniature"), intact + 1);
    let _ = std::fs::remove_dir_all(&racine);
}

#[test]
fn un_projet_copie_hors_de_l_application_n_ecrase_pas_l_original() {
    let racine = dossier("copie");
    deux_projets(&racine);
    let d = Dossier::dans(&racine);
    let projets = racine.join("projets");
    // Une copie faite dans l'explorateur de fichiers : même contenu, donc même nom de projet.
    std::fs::copy(
        projets.join("Additionneur.json"),
        projets.join("Additionneur - Copie.json"),
    )
    .unwrap();
    let original = std::fs::read(projets.join("Additionneur.json")).unwrap();
    let mut b = lancer(&racine, 1440.0, 900.0);
    // La liste la montre sous le nom de son fichier, et elle s'ouvre sous ce nom.
    assert!(b
        .state()
        .projets
        .iter()
        .any(|f| f.fichier == "Additionneur - Copie" && f.nom == "Additionneur - Copie"));
    b.state_mut()
        .ouvrir(Ouverture::Projet("Additionneur - Copie".to_owned()));
    b.run_steps(30);
    assert_eq!(b.state().atelier.projet.nom, "Additionneur - Copie");
    assert_eq!(b.state().courant().as_deref(), Some("Additionneur - Copie"));
    assert_eq!(d.dernier().as_deref(), Some("Additionneur - Copie"));
    // Une vraie modification (une porte de plus) l'enregistre chez lui, sans toucher à
    // l'original : les deux fichiers n'ont plus le même contenu.
    let avant = elements(&d, "Additionneur");
    cliquer(&mut b, "nand", 0);
    b.state_mut().enregistrer();
    assert_eq!(
        std::fs::read(projets.join("Additionneur.json")).unwrap(),
        original
    );
    assert_eq!(elements(&d, "Additionneur"), avant);
    assert_eq!(elements(&d, "Additionneur - Copie"), avant + 1);
    assert_eq!(
        d.ouvrir("Additionneur - Copie").unwrap().nom,
        "Additionneur - Copie"
    );
    assert_eq!(d.lister().len(), 3);
    let _ = std::fs::remove_dir_all(&racine);
}

#[test]
fn un_fichier_au_nom_inhabituel_reste_a_sa_place_tant_qu_on_ne_le_renomme_pas() {
    let racine = dossier("nom-inhabituel");
    deux_projets(&racine);
    let d = Dossier::dans(&racine);
    let projets = racine.join("projets");
    // Un nom que l'application n'aurait pas écrit ainsi : points, esperluette, dièse.
    let fichier = "Additionneur v1.2 & retenue #3";
    std::fs::rename(
        projets.join("Additionneur.json"),
        projets.join(format!("{fichier}.json")),
    )
    .unwrap();
    let mut b = lancer(&racine, 1440.0, 900.0);
    b.state_mut().ouvrir(Ouverture::Projet(fichier.to_owned()));
    b.run_steps(30);
    assert_eq!(b.state().atelier.projet.nom, fichier);
    let avant = elements(&d, fichier);
    cliquer(&mut b, "nand", 0);
    b.state_mut().enregistrer();
    // Il est enregistré dans son fichier, sans copie ni faux message de nom déjà pris.
    assert_eq!(d.lister(), [fichier, "Ma porte NON"]);
    assert_eq!(elements(&d, fichier), avant + 1);
    assert_eq!(b.state().courant().as_deref(), Some(fichier));
    assert!(!b
        .state()
        .atelier
        .messages
        .iter()
        .any(|m| m.texte.contains("s'appelle déjà")));
    // Valider son nom sans le changer, depuis la liste, ne le déplace pas non plus.
    assert_eq!(d.renommer(fichier, fichier).as_deref(), Ok(fichier));
    // Le renommer pour de bon lui donne un fichier au nom de l'application.
    assert!(b.state_mut().atelier.renommer_projet("Additionneur 4 bits"));
    b.state_mut().enregistrer();
    assert_eq!(d.lister(), ["Additionneur 4 bits", "Ma porte NON"]);
    let _ = std::fs::remove_dir_all(&racine);
}

#[test]
fn un_avancement_illisible_est_garde_et_n_est_pas_ecrase() {
    let racine = dossier("parcours-illisible");
    deux_projets(&racine);
    let abime = "{ \"faites\": [\"non\", \"et\"";
    std::fs::write(racine.join("parcours.json"), abime).unwrap();
    let mut b = lancer(&racine, 1440.0, 900.0);
    assert!(
        b.state()
            .atelier
            .messages
            .iter()
            .any(|m| m.texte.contains("parcours-illisible.json")),
        "le message dit où l'ancien avancement est gardé"
    );
    assert!(!racine.join("parcours.json").exists());
    // L'avancement de la séance s'enregistre à côté, sans toucher au fichier gardé.
    b.state_mut().atelier.parcours.version += 1;
    b.run_steps(5);
    assert!(racine.join("parcours.json").exists());
    assert_eq!(
        std::fs::read_to_string(racine.join("parcours-illisible.json")).unwrap(),
        abime
    );
    let _ = std::fs::remove_dir_all(&racine);
}

#[test]
fn des_reglages_abimes_gardent_ce_qui_se_lit_et_le_disent() {
    let racine = dossier("reglages-abimes");
    std::fs::create_dir_all(&racine).unwrap();
    // Une virgule en trop après une ligne enlevée à la main : le fichier n'est plus du JSON.
    std::fs::write(
        racine.join("reglages.json"),
        "{\n  \"theme\": \"clair\",\n  \"mises_a_jour_automatiques\": false,\n}\n",
    )
    .unwrap();
    let b = lancer(&racine, 1440.0, 900.0);
    let app = b.state();
    assert!(!app.reglages.mises_a_jour_automatiques);
    assert_eq!(app.apparence.mode, Mode::Clair);
    assert!(app
        .atelier
        .messages
        .iter()
        .any(|m| m.texte.contains("reglages.json")));
    let _ = std::fs::remove_dir_all(&racine);
}

#[test]
fn les_messages_passagers_s_en_vont_aussi_de_l_accueil() {
    let racine = dossier("messages");
    deux_projets(&racine);
    let mut b = lancer(&racine, 1440.0, 900.0);
    assert_eq!(b.state().ecran, Ecran::Accueil);
    b.state_mut().atelier.dire("Copie enregistrée");
    b.run_steps(30);
    assert_eq!(b.state().atelier.messages.len(), 1);
    // Quelques secondes plus tard, il est parti : il ne cache plus rien et ne capte plus de clic.
    b.run_steps(6 * 60);
    assert!(b.state().atelier.messages.is_empty());
    let _ = std::fs::remove_dir_all(&racine);
}

#[test]
fn les_parametres_s_appliquent_et_survivent_au_relancement() {
    let racine = dossier("parametres");
    deux_projets(&racine);
    let mut b = lancer(&racine, 1440.0, 900.0);
    cliquer(&mut b, "menu", 3);
    assert_eq!(b.state().accueil.rubrique, Rubrique::Parametres);
    assert_eq!(b.state().apparence.mode, Mode::Sombre);
    cliquer(&mut b, "reglage-theme", 1);
    assert_eq!(b.state().apparence.mode, Mode::Clair);
    cliquer(&mut b, "reglage-texte", 2);
    assert!((b.state().apparence.formes.echelle_texte - 1.15).abs() < 1e-4);
    cliquer(&mut b, "reglage-animations", 1);
    assert!(b.state().apparence.mouvement.sobre);
    cliquer(&mut b, "reglage-vitesse", 2);
    assert_eq!(b.state().apparence.mouvement.vitesse_initiale, 2);
    assert_eq!(b.state().atelier.vitesse, 2);
    cliquer(&mut b, "reglage-mises-a-jour", 1);
    assert!(!b.state().reglages.mises_a_jour_automatiques);
    cliquer(&mut b, "reglage-lancement", 1);
    assert_eq!(b.state().reglages.au_lancement, Lancement::Reprendre);
    drop(b);

    // Relancée, l'application garde tout, et va droit au projet.
    let reglages = Dossier::dans(&racine).reglages();
    assert_eq!(reglages.theme, Some(Mode::Clair));
    let b = lancer(&racine, 1440.0, 900.0);
    assert_eq!(b.state().ecran, Ecran::Atelier);
    assert_eq!(b.state().atelier.projet.nom, "Additionneur");
    assert_eq!(b.state().apparence.mode, Mode::Clair);
    assert!(b.state().apparence.mouvement.sobre);
    assert_eq!(b.state().atelier.vitesse, 2);
    drop(b);

    // Le bouton du thème dans l'atelier est retenu lui aussi.
    let mut b = lancer(&racine, 1440.0, 900.0);
    b.state_mut().changer_mode();
    assert_eq!(Dossier::dans(&racine).reglages().theme, Some(Mode::Sombre));
    let _ = std::fs::remove_dir_all(&racine);
}

#[test]
fn l_accueil_tient_dans_une_petite_fenetre() {
    let racine = dossier("petite");
    deux_projets(&racine);
    let mut b = lancer(&racine, 1040.0, 660.0);
    for rubrique in 0..4 {
        cliquer(&mut b, "menu", rubrique);
        // Tout ce qui est cliquable reste dans la fenêtre, à l'horizontale.
        let app = b.state();
        assert!(app.accueil.repere("menu", rubrique).is_some());
    }
    // La molette fait défiler les paramètres, plus hauts que la fenêtre.
    let avant = b.state().accueil.defilement;
    b.hover_at(Pos2::new(600.0, 400.0));
    b.event(egui::Event::MouseWheel {
        unit: egui::MouseWheelUnit::Point,
        delta: Vec2::new(0.0, -400.0),
        phase: egui::TouchPhase::Move,
        source: egui::MouseWheelSource::Unknown,
        modifiers: egui::Modifiers::NONE,
    });
    b.run_steps(30);
    assert!(b.state().accueil.defilement > avant);
    let _ = std::fs::remove_dir_all(&racine);
}

#[test]
fn un_enregistrement_rate_est_dit_une_fois_puis_retente() {
    let racine = dossier("ecriture-ratee");
    // Le dossier des projets est inutilisable : un fichier porte son nom.
    std::fs::create_dir_all(&racine).unwrap();
    std::fs::write(racine.join("projets"), "pas un dossier").unwrap();
    let d = Dossier::dans(&racine);
    let mut b = lancer(&racine, 1440.0, 900.0);
    b.state_mut().ouvrir(Ouverture::Nouveau);
    b.run_steps(30);
    cliquer(&mut b, "nand", 0);
    b.run_steps(600);
    let echecs = |b: &Banc| {
        b.state()
            .atelier
            .messages
            .iter()
            .filter(|m| m.texte.contains("n'a pas pu être enregistré"))
            .count()
    };
    assert_eq!(echecs(&b), 1, "l'échec est dit, une seule fois");
    assert!(d.lister().is_empty());
    // Le dossier redevient utilisable : sans nouvelle modification, le projet finit écrit.
    std::fs::remove_file(racine.join("projets")).unwrap();
    b.run_steps(600);
    assert_eq!(d.lister(), ["Nouveau projet"]);
    assert_eq!(elements(&d, "Nouveau projet"), 1);
    let _ = std::fs::remove_dir_all(&racine);
}

/// Un faux système de raccourcis : un dossier par lieu, un raccourci est un fichier texte qui
/// contient le chemin de l'exécutable.
struct FauxShell(PathBuf);

impl ce_app::raccourci::Systeme for FauxShell {
    fn dossier(&self, lieu: ce_app::raccourci::Lieu) -> Option<PathBuf> {
        Some(self.0.join(format!("{lieu:?}")))
    }
    fn ecrire(&self, fichier: &Path, cible: &Path) -> Result<(), String> {
        std::fs::write(fichier, cible.to_string_lossy().as_bytes()).map_err(|e| e.to_string())
    }
    fn cible(&self, fichier: &Path) -> Result<PathBuf, String> {
        std::fs::read_to_string(fichier)
            .map(PathBuf::from)
            .map_err(|e| e.to_string())
    }
}

#[test]
fn les_raccourcis_sont_poses_au_premier_lancement_et_se_reglent_dans_les_parametres() {
    use ce_app::raccourci::{Raccourcis, NOM_DU_FICHIER};
    let racine = dossier("raccourcis");
    let systeme = racine.join("systeme");
    std::fs::create_dir_all(&systeme).unwrap();
    let exe = systeme.join("computer-engineer.exe");
    std::fs::write(&exe, b"exe").unwrap();
    let bureau = systeme.join("Bureau").join(NOM_DU_FICHIER);
    let menu = systeme.join("MenuDemarrer").join(NOM_DU_FICHIER);
    let lancer_avec_raccourcis = |racine: &Path| {
        let app = Application::nouvelle(Apparence::default(), Surveillance::default())
            .avec_dossier(Dossier::dans(racine))
            .au_lancement()
            .avec_raccourcis(Raccourcis::avec(
                Box::new(FauxShell(systeme.clone())),
                exe.clone(),
                false,
            ));
        let mut b = Harness::builder()
            .with_size(Vec2::new(1440.0, 1500.0))
            .with_pixels_per_point(1.0)
            .with_step_dt(1.0 / 60.0)
            .build_ui_state(|ui, app: &mut Application| app.image(ui), app);
        b.run_steps(60);
        b
    };

    // Premier lancement : les deux raccourcis sont posés, et l'application le dit.
    let mut b = lancer_avec_raccourcis(&racine);
    assert!(bureau.is_file() && menu.is_file());
    assert!(b
        .state()
        .atelier
        .messages
        .iter()
        .any(|m| m.texte.contains("Raccourcis ajoutés")));
    assert!(Dossier::dans(&racine).reglages().raccourcis_proposes);

    // Paramètres : retirer celui du bureau, puis le remettre.
    cliquer(&mut b, "menu", 3);
    assert_eq!(b.state().accueil.rubrique, Rubrique::Parametres);
    cliquer(&mut b, "reglage-raccourci-bureau", 1);
    assert!(!bureau.is_file(), "« Non » retire le raccourci du bureau");
    assert!(menu.is_file());
    cliquer(&mut b, "reglage-raccourci-bureau", 0);
    assert!(bureau.is_file(), "« Oui » le remet");
    cliquer(&mut b, "reglage-raccourci-demarrer", 1);
    assert!(!menu.is_file());
    drop(b);

    // Relancée, l'application ne remet pas le raccourci retiré, et ne redit rien.
    let b = lancer_avec_raccourcis(&racine);
    assert!(bureau.is_file() && !menu.is_file());
    assert!(!b
        .state()
        .atelier
        .messages
        .iter()
        .any(|m| m.texte.contains("Raccourci")));
    drop(b);
    let _ = std::fs::remove_dir_all(&racine);
}
