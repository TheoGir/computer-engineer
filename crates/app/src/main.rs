//! Lanceur de Computer Engineer.
//!
//! - sans argument : ouvre l'application ;
//! - `--ecrire-apparence [fichier]` : écrit l'apparence par défaut dans un fichier JSON à
//!   modifier (par défaut `apparence.json` dans le dossier courant) ;
//! - `--verifier` : contrôle rapide sans fenêtre (moteur, quête, ordinateur miniature,
//!   apparence), code de sortie 0 si tout va bien ;
//! - `--version` : affiche la version ;
//! - `--mettre-a-jour` : cherche, vérifie et installe la dernière version publiée, sans fenêtre
//!   (code de sortie 0 si l'application est à jour ou vient d'être mise à jour).

// En version finale sous Windows, pas de console noire derrière la fenêtre.
#![cfg_attr(all(not(debug_assertions), windows), windows_subsystem = "windows")]

use std::process::ExitCode;

use ce_app::app::Application;
use ce_app::apparence::{Apparence, FICHIER_APPARENCE};
use ce_app::document::Genre;
use ce_app::etat::{Atelier, Etat};
use ce_app::exemples::EXEMPLES;
use ce_app::incident;

fn main() -> ExitCode {
    let args: Vec<String> = std::env::args().skip(1).collect();
    match args.first().map(String::as_str) {
        Some("--version") => {
            println!("Computer Engineer {}", env!("CARGO_PKG_VERSION"));
            ExitCode::SUCCESS
        }
        Some("--ecrire-apparence") => {
            let chemin = args.get(1).map_or(FICHIER_APPARENCE, String::as_str);
            match std::fs::write(chemin, Apparence::default().en_json()) {
                Ok(()) => {
                    println!("Apparence par défaut écrite dans {chemin}. Modifie ce fichier : l'application le relit pendant qu'elle tourne.");
                    ExitCode::SUCCESS
                }
                Err(e) => {
                    eprintln!("Impossible d'écrire {chemin} : {e}");
                    ExitCode::FAILURE
                }
            }
        }
        Some("--verifier") => verifier(),
        Some("--mettre-a-jour") => mettre_a_jour(source()),
        Some(autre) => {
            eprintln!("Argument inconnu : {autre}. Arguments acceptés : --ecrire-apparence [fichier], --verifier, --mettre-a-jour, --version.");
            ExitCode::FAILURE
        }
        None => lancer(),
    }
}

/// La source des mises à jour : l'officielle.
#[cfg(not(feature = "banc-mise-a-jour"))]
fn source() -> ce_app::mise_a_jour::Source {
    ce_app::mise_a_jour::Source::officielle()
}

/// Exécutable d'essai seulement (fonction `banc-mise-a-jour`, jamais publiée) : la source vient
/// des variables `CE_BANC_API` (faux serveur de versions) et `CE_BANC_CLE` (clé publique d'essai,
/// 64 chiffres hexadécimaux). Sert à vérifier dans la CI Windows qu'un exécutable en marche se
/// remplace lui-même.
#[cfg(feature = "banc-mise-a-jour")]
fn source() -> ce_app::mise_a_jour::Source {
    let mut s = ce_app::mise_a_jour::Source::officielle();
    if let Ok(api) = std::env::var("CE_BANC_API") {
        s.api = Some(api);
        s.proprietaire = "essai".into();
        s.depot = "versions".into();
    }
    if let Ok(hex) = std::env::var("CE_BANC_CLE") {
        let octets: Vec<u8> = (0..hex.len() / 2)
            .filter_map(|i| u8::from_str_radix(hex.get(2 * i..2 * i + 2)?, 16).ok())
            .collect();
        s.cles = octets.try_into().ok().into_iter().collect();
    }
    s
}

/// Installe la dernière version publiée, sans fenêtre.
fn mettre_a_jour(source: ce_app::mise_a_jour::Source) -> ExitCode {
    match source.installer() {
        Ok(Some(version)) => {
            println!("Version {version} installée. Elle démarre au prochain lancement.");
            ExitCode::SUCCESS
        }
        Ok(None) => {
            println!("Déjà à jour ({}).", source.version);
            ExitCode::SUCCESS
        }
        Err(e) => {
            eprintln!("Mise à jour impossible : {e}");
            ExitCode::FAILURE
        }
    }
}

/// Contrôle sans fenêtre : l'atelier démarre, le demi-additionneur se stabilise sur les bonnes
/// valeurs, les quatre cas de la quête se valident, l'ordinateur miniature calcule 6 × 7 avec
/// son clavier et son écran, l'apparence fait l'aller-retour en JSON.
fn verifier() -> ExitCode {
    // Installée : la version inscrite dans les Paramètres de Windows suit celle-ci (comme au
    // lancement de la fenêtre), ce que la CI vérifie après avoir installé l'application.
    if ce_app::installation::tenir_la_version_a_jour(env!("CARGO_PKG_VERSION")) {
        println!("Version inscrite dans les Paramètres de Windows mise à jour.");
    }
    let a = Apparence::default();
    let mut at = Atelier::nouveau(&a);
    let mut pas = 0;
    let entrees: Vec<_> = at.session.entrees.keys().copied().collect();
    // Code de Gray sur les deux interrupteurs : 10 (départ), 11, 01, 00.
    for bascule in [
        None,
        entrees.get(1).copied(),
        entrees.first().copied(),
        entrees.get(1).copied(),
    ] {
        if let Some(id) = bascule {
            at.basculer(id);
        }
        while at.etat() != Etat::Stable && pas < 100 {
            at.avancer(&a, at.duree_pas(&a));
            pas += 1;
        }
    }
    let quete = at.quete.as_ref().is_some_and(|q| q.reussie());

    // L'ordinateur miniature, ouvert comme dans l'application.
    let mut ecran = String::new();
    let portes = if at.ouvrir_exemple(&EXEMPLES[0], EXEMPLES[0].nom, &a) {
        let trouver = |at: &Atelier, genre: Genre| {
            at.doc
                .elements
                .iter()
                .find(|e| e.genre == genre)
                .map(|e| e.id)
        };
        at.taper(b'6');
        at.taper(b'7');
        at.session.avancer_vite(128 * 400);
        if let Some(id) = trouver(&at, Genre::Ecran) {
            ecran = at.session.ecran(id).to_owned();
        }
        at.session.nb_portes()
    } else {
        0
    };
    let ordinateur = ecran == "6*7=42";
    let json = Apparence::de_json(&a.en_json()).is_ok_and(|b| b == a);
    println!(
        "Computer Engineer {} : quête {}, {pas} pas simulés ; ordinateur de {portes} portes, écran « {ecran} » ({}) ; apparence {}.",
        env!("CARGO_PKG_VERSION"),
        if quete { "réussie" } else { "NON réussie" },
        if ordinateur { "juste" } else { "FAUX" },
        if json {
            "relue à l'identique"
        } else {
            "NON relue"
        }
    );
    if quete && ordinateur && json {
        ExitCode::SUCCESS
    } else {
        ExitCode::FAILURE
    }
}

/// L'icône de la fenêtre : 128 × 128 pixels RVBA, dessinés par `outils/icone.py`.
fn icone_de_fenetre() -> egui::IconData {
    const COTE: u32 = 128;
    let rgba = include_bytes!("../icone/fenetre.rgba").to_vec();
    debug_assert_eq!(rgba.len(), (COTE * COTE * 4) as usize);
    egui::IconData {
        rgba,
        width: COTE,
        height: COTE,
    }
}

/// Le dossier des rapports : dans le dossier de données, sinon dans le dossier temporaire.
fn dossier_des_rapports() -> std::path::PathBuf {
    ce_app::sauvegarde::Dossier::par_defaut()
        .map_or_else(
            || std::env::temp_dir().join("Computer Engineer"),
            |d| d.racine().to_owned(),
        )
        .join(incident::DOSSIER_RAPPORTS)
}

fn lancer() -> ExitCode {
    let rapports = dossier_des_rapports();
    incident::installer_le_rapport_de_plantage(rapports.clone());
    // Une seule fenêtre à la fois sur le dossier de données : deux fenêtres ouvertes sur le même
    // projet s'écraseraient l'une l'autre en l'enregistrant.
    let _verrou = match ce_app::sauvegarde::Dossier::par_defaut() {
        Some(d) => match incident::verrouiller(d.racine(), std::time::Duration::from_secs(10)) {
            incident::Verrou::Occupe => {
                incident::annoncer_un_echec(
                    "Computer Engineer est déjà ouvert dans une autre fenêtre.\n\nUtilise cette fenêtre-là (regarde dans la barre des tâches) : deux fenêtres ouvertes sur les mêmes projets s'écraseraient l'une l'autre.",
                );
                eprintln!("Computer Engineer est déjà ouvert dans une autre fenêtre.");
                return ExitCode::SUCCESS;
            }
            verrou => Some(verrou),
        },
        None => None,
    };
    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_title("Computer Engineer")
            .with_inner_size([1440.0, 900.0])
            .with_min_inner_size([1040.0, 660.0])
            .with_app_id("computer-engineer")
            .with_icon(icone_de_fenetre()),
        ..Default::default()
    };
    match eframe::run_native(
        "Computer Engineer",
        options,
        Box::new(|cc| Ok(Box::new(Application::pour_eframe(cc)))),
    ) {
        Ok(()) => ExitCode::SUCCESS,
        Err(e) => {
            let detail = format!("L'application n'a pas pu démarrer : {e}");
            eprintln!("{detail}");
            let texte = incident::texte_du_rapport("démarrage impossible", &detail);
            let ou = incident::ecrire_un_rapport(&rapports, "demarrage", &texte).map_or_else(
                |_| String::new(),
                |p| format!("\n\nUn rapport a été écrit dans {}.", p.display()),
            );
            incident::annoncer_un_echec(&format!(
                "Computer Engineer n'a pas pu ouvrir sa fenêtre.\n\n{e}\n\nLa cause la plus fréquente est un pilote de carte graphique trop ancien : mettre à jour le pilote (Windows Update ou le site du fabricant) règle en général le problème.{ou}"
            ));
            ExitCode::FAILURE
        }
    }
}
