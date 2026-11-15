//! L'application en anglais : on parcourt les écrans (accueil, parcours, projets,
//! paramètres, atelier, chaque quête, l'aide, les fenêtres d'outils) et on relit tout le texte
//! dessiné, à la recherche d'un reste de français.
//!
//! Le texte est lu dans les formes qu'egui s'apprête à peindre : celui des boutons comme celui
//! que le plan dessine lui-même. Les noms des broches (raz, clk, ecran…) sont des noms de
//! circuit, écrits sans accent, et restent tels quels.

use ce_app::accueil::Rubrique as Menu;
use ce_app::app::{Application, Ecran};
use ce_app::apparence::{Apparence, Surveillance};
use ce_app::fenetres::Rubrique;
use ce_app::langue::{self, Langue};
use ce_app::panneaux::Ouverture;
use ce_app::quete::PARCOURS;
use ce_app::reglages::Reglages;
use ce_app::sauvegarde::Dossier;
use egui::epaint::Shape;
use egui::{Key, Modifiers, Vec2};
use egui_kittest::Harness;

type Banc = Harness<'static, Application>;

/// Les mots français courants qui n'existent pas en anglais.
const MOTS_FRANCAIS: &[&str] = &[
    "le",
    "la",
    "les",
    "des",
    "du",
    "une",
    "un",
    "est",
    "pour",
    "dans",
    "avec",
    "sur",
    "et",
    "ou",
    "pas",
    "qui",
    "que",
    "ce",
    "cette",
    "sont",
    "aux",
    "au",
    "ton",
    "ta",
    "tes",
    "ses",
    "son",
    "sa",
    "leur",
    "mais",
    "donc",
    "quand",
    "puis",
    "tout",
    "tous",
    "rien",
    "porte",
    "portes",
    "entrée",
    "fil",
    "fils",
    "lampe",
    "brique",
    "briques",
    "quête",
    "projet",
    "projets",
    "ouvrir",
    "fermer",
    "annuler",
    "vérifier",
    "suivant",
    "précédent",
    "aide",
];

/// Les noms de circuit livrés (briques, broches) écrits en français : en anglais, ils
/// s'affichent traduits (`ET` devient `AND`, `raz` devient `reset`).
fn noms_francais() -> std::collections::BTreeSet<String> {
    langue::NOMS_ANGLAIS
        .iter()
        .map(|(fr, _)| (*fr).to_owned())
        .collect()
}

/// Les mots accentués permis en anglais : le nom de la langue, et ce que l'anglais emprunte.
const PERMIS: &[&str] = &["Français", "déjà", "café"];

/// Tout le texte peint dans la dernière image.
fn textes(b: &Banc) -> Vec<String> {
    fn lire(forme: &Shape, sortie: &mut Vec<String>) {
        match forme {
            Shape::Text(t) => sortie.push(t.galley.text().to_owned()),
            Shape::Vec(v) => v.iter().for_each(|f| lire(f, sortie)),
            _ => {}
        }
    }
    let mut sortie = Vec::new();
    for c in &b.output().shapes {
        lire(&c.shape, &mut sortie);
    }
    sortie
}

/// Les mots qui trahissent du français dans ce texte (hors chemins de fichiers), noms de
/// briques et de broches compris.
fn francais(texte: &str, noms: &std::collections::BTreeSet<String>) -> Vec<String> {
    // Un chemin absolu, sous Linux (/tmp/…) ou sous Windows (C:\Users\…).
    let chemin_windows = texte.as_bytes().get(1..3) == Some(b":\\".as_slice());
    if texte.starts_with('/') || chemin_windows {
        return Vec::new();
    }
    // « in front of » est de l'anglais, pas la broche `front`.
    let texte = texte.replace("in front of", "");
    // Ce qui est entre guillemets anglais est cité tel quel : un nom de dossier, de broche.
    let mut hors_citations = String::new();
    for (i, morceau) in texte.split(['“', '”']).enumerate() {
        if i % 2 == 0 {
            hors_citations.push_str(morceau);
            hors_citations.push(' ');
        }
    }
    hors_citations
        .split(|c: char| !c.is_alphanumeric() && c != '\'' && c != '’' && c != '_')
        .filter(|mot| !mot.is_empty() && !PERMIS.contains(mot))
        .filter(|mot| {
            noms.contains(*mot)
                || mot.chars().any(|c| "àâçéèêëîïôûùœÀÂÇÉÈÊÎÔÛ".contains(c))
                || MOTS_FRANCAIS.contains(&mot.to_lowercase().as_str())
                || mot.starts_with("l'")
                || mot.starts_with("d'")
                || mot.starts_with("qu'")
        })
        .map(str::to_owned)
        .collect()
}

/// Relève le français de l'image en cours, avec le nom de l'écran.
fn relever(b: &Banc, ecran: &str, trouve: &mut Vec<String>) {
    let noms = noms_francais();
    for t in textes(b) {
        let mots = francais(&t, &noms);
        if !mots.is_empty() {
            let extrait: String = t.chars().take(100).collect();
            let ligne = format!("{ecran} : {mots:?} dans « {extrait} »");
            if !trouve.contains(&ligne) {
                trouve.push(ligne);
            }
        }
    }
}

fn cliquer(b: &mut Banc, p: egui::Pos2) {
    b.hover_at(p);
    b.step();
    b.drag_at(p);
    b.step();
    b.drop_at(p);
    b.run_steps(20);
    b.hover_at(egui::Pos2::new(2.0, 2.0));
    b.run_steps(5);
}

#[test]
fn rien_ne_reste_en_francais() {
    langue::choisir_pour_ce_fil(Some(Langue::Anglais));
    let racine = std::env::temp_dir().join(format!("ce-anglais-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&racine);
    let d = Dossier::dans(&racine);
    d.enregistrer_reglages(&Reglages {
        langue: Some(Langue::Anglais),
        ..Reglages::default()
    })
    .expect("réglages");
    let app = Application::nouvelle(Apparence::default(), Surveillance::default())
        .avec_dossier(Dossier::dans(&racine))
        .au_lancement();
    let mut b = Harness::builder()
        .with_size(Vec2::new(1440.0, 900.0))
        .with_pixels_per_point(1.0)
        .with_step_dt(1.0 / 60.0)
        .build_ui_state(|ui, app: &mut Application| app.image(ui), app);
    b.run_steps(120);
    assert_eq!(langue::actuelle(), Langue::Anglais);
    let mut trouve = Vec::new();

    // L'accueil et ses quatre rubriques.
    for (rang, menu) in [
        Menu::Accueil,
        Menu::Parcours,
        Menu::Projets,
        Menu::Parametres,
    ]
    .into_iter()
    .enumerate()
    {
        b.state_mut().accueil.rubrique = menu;
        b.run_steps(30);
        relever(&b, &format!("accueil {rang}"), &mut trouve);
    }
    // Chaque chapitre du parcours, et chaque quête dedans.
    b.state_mut().accueil.rubrique = Menu::Parcours;
    for (c, (_, quetes)) in ce_app::accueil::chapitres().into_iter().enumerate() {
        for q in quetes {
            let a = &mut b.state_mut().accueil;
            a.chapitre = c;
            a.quete = Some(q);
            b.run_steps(8);
            relever(
                &b,
                &format!("parcours, quête {}", PARCOURS[q].cle),
                &mut trouve,
            );
        }
    }

    // L'atelier : chaque quête ouverte, puis les exemples.
    for f in PARCOURS {
        b.state_mut().ouvrir(Ouverture::Quete(f.cle.to_owned()));
        b.state_mut().ecran = Ecran::Atelier;
        b.run_steps(40);
        relever(&b, &format!("atelier, quête {}", f.cle), &mut trouve);
        // L'inspecteur, sur chaque élément du plan de la quête.
        let ids: Vec<_> = b
            .state()
            .atelier
            .doc
            .elements
            .iter()
            .map(|e| e.id)
            .collect();
        for id in ids.into_iter().take(6) {
            b.state_mut().atelier.selection = Some(ce_app::etat::Selection::Elements(vec![id]));
            b.run_steps(3);
            relever(&b, &format!("inspecteur, quête {}", f.cle), &mut trouve);
        }
        b.state_mut().atelier.selection = None;
    }
    for i in 0..ce_app::exemples::EXEMPLES.len() {
        b.state_mut().ouvrir(Ouverture::Exemple(i));
        b.run_steps(60);
        relever(&b, &format!("atelier, exemple {i}"), &mut trouve);
    }

    // Les fenêtres d'outils, sur l'ordinateur miniature.
    b.state_mut().ouvrir(Ouverture::Exemple(0));
    b.run_steps(60);
    for (touche, nom) in [
        (Key::P, "programme"),
        (Key::E, "analyse"),
        (Key::K, "automate"),
        (Key::T, "banc d'essai"),
    ] {
        b.key_press_modifiers(Modifiers::COMMAND, touche);
        b.run_steps(20);
        relever(&b, nom, &mut trouve);
        let f = &mut b.state_mut().fenetres;
        f.editeur = None;
        f.analyse = None;
        f.automate = None;
        f.banc = None;
        b.run_steps(5);
    }

    // L'aide, rubrique par rubrique.
    for r in [
        Rubrique::PremiersPas,
        Rubrique::Raccourcis,
        Rubrique::Programmer,
        Rubrique::Algebre,
        Rubrique::Lexique,
        Rubrique::Fichiers,
        Rubrique::APropos,
    ] {
        b.state_mut().fenetres.aide = Some(r);
        b.run_steps(10);
        relever(&b, &format!("aide {r:?}"), &mut trouve);
    }
    b.state_mut().fenetres.aide = None;
    b.run_steps(5);

    // La documentation, page par page.
    for p in std::iter::once(ce_app::documentation::Page::Sommaire)
        .chain(ce_app::documentation::toutes_les_pages())
    {
        let nom = format!("documentation {p:?}");
        b.state_mut().fenetres.documentation = Some(ce_app::fenetre_doc::Documentation::sur(p));
        b.run_steps(3);
        relever(&b, &nom, &mut trouve);
    }
    b.state_mut().fenetres.documentation = None;
    b.run_steps(5);

    // Un projet vide, et le retour à l'accueil.
    b.state_mut().ouvrir(Ouverture::Nouveau);
    b.run_steps(40);
    relever(&b, "projet vide", &mut trouve);
    if let Some(m) = b.state().panneaux.repere("marque", 0) {
        cliquer(&mut b, m.center());
        relever(&b, "retour à l'accueil", &mut trouve);
    }

    drop(b);
    let _ = std::fs::remove_dir_all(&racine);
    langue::choisir_pour_ce_fil(None);
    assert!(
        trouve.is_empty(),
        "{} textes encore en français :\n{}",
        trouve.len(),
        trouve.join("\n")
    );
}

/// Captures de l'application en anglais, avec `CE_CAPTURES=/dossier` (il faut un pilote
/// graphique, Vulkan logiciel « lavapipe » sous Linux par exemple).
#[test]
fn captures_en_anglais() {
    let Ok(sortie) = std::env::var("CE_CAPTURES") else {
        eprintln!("CE_CAPTURES n'est pas défini : captures ignorées.");
        return;
    };
    langue::choisir_pour_ce_fil(Some(Langue::Anglais));
    let racine = std::env::temp_dir().join(format!("ce-anglais-captures-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&racine);
    let d = Dossier::dans(&racine);
    d.enregistrer_reglages(&Reglages {
        langue: Some(Langue::Anglais),
        ..Reglages::default()
    })
    .expect("réglages");
    let app = Application::nouvelle(Apparence::default(), Surveillance::default())
        .avec_dossier(Dossier::dans(&racine))
        .au_lancement();
    let mut b = Harness::builder()
        .with_size(Vec2::new(1440.0, 900.0))
        .with_pixels_per_point(2.0)
        .with_step_dt(1.0 / 60.0)
        .wgpu()
        .build_ui_state(|ui, app: &mut Application| app.image(ui), app);
    b.run_steps(120);
    let enregistrer = |b: &mut Banc, nom: &str| {
        let image = b.render().expect("rendu");
        std::fs::create_dir_all(&sortie).expect("dossier de sortie");
        image
            .save(format!("{sortie}/{nom}.png"))
            .expect("écriture de la capture");
    };
    enregistrer(&mut b, "en1-accueil");
    b.state_mut().accueil.rubrique = Menu::Parcours;
    {
        let a = &mut b.state_mut().accueil;
        a.chapitre = 1;
        a.quete = Some(ce_app::accueil::chapitres()[1].1[1]);
    }
    b.run_steps(40);
    enregistrer(&mut b, "en2-parcours");
    b.state_mut().accueil.rubrique = Menu::Parametres;
    b.run_steps(40);
    enregistrer(&mut b, "en3-parametres");
    b.state_mut()
        .ouvrir(Ouverture::Quete(PARCOURS[3].cle.to_owned()));
    b.state_mut().ecran = Ecran::Atelier;
    b.run_steps(200);
    enregistrer(&mut b, "en4-quete");
    b.state_mut().ouvrir(Ouverture::Exemple(0));
    b.run_steps(200);
    enregistrer(&mut b, "en5-ordinateur");
    b.key_press_modifiers(Modifiers::COMMAND, Key::P);
    b.run_steps(20);
    enregistrer(&mut b, "en6-programme");
    b.state_mut().fenetres.editeur = None;
    b.state_mut().fenetres.aide = Some(Rubrique::PremiersPas);
    b.run_steps(20);
    enregistrer(&mut b, "en7-aide");
    drop(b);
    let _ = std::fs::remove_dir_all(&racine);
    langue::choisir_pour_ce_fil(None);
}

/// Les noms ne sont traduits qu'à l'affichage : un exemple ouvert en anglais enregistre
/// exactement les mêmes briques et le même plan qu'en français (seul le nom du projet suit la
/// langue), et un projet s'échange donc d'une langue à l'autre.
#[test]
fn un_projet_est_le_meme_dans_les_deux_langues() {
    let ouvrir = |langue_choisie: Langue| {
        langue::choisir_pour_ce_fil(Some(langue_choisie));
        let mut app = Application::nouvelle(Apparence::default(), Surveillance::default());
        app.ouvrir(Ouverture::Exemple(0));
        let p = &app.atelier.projet;
        let valeur = (
            serde_json::to_value(&p.principal).expect("plan"),
            serde_json::to_value(&p.modeles).expect("briques"),
        );
        langue::choisir_pour_ce_fil(None);
        valeur
    };
    let francais = ouvrir(Langue::Francais);
    let anglais = ouvrir(Langue::Anglais);
    assert_eq!(francais, anglais);
    assert!(francais.1.to_string().contains("\"PROCESSEUR\""));
}
