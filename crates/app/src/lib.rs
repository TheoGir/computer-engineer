//! # ce-app
//!
//! Application graphique de Computer Engineer.
//!
//! Ce qui se règle, en un seul endroit :
//! - [`apparence`] : couleurs, formes, animations et seuils de manipulation, modifiables
//!   dans un fichier `apparence.json` relu à chaud ;
//! - [`disposition`] : la place de chaque panneau dans la fenêtre.
//!
//! Ce que l'utilisateur construit, sans rien d'écran :
//! - [`document`] : le circuit en cours d'édition, avec annuler et rétablir ;
//! - [`simulation`] : le pont vers le moteur `ce-moteur` ;
//! - [`quete`] : les quêtes du parcours, vérifiées contre un corrigé `ce-circuit` ;
//! - [`parcours`] : l'avancement, les quêtes réussies et les briques gagnées ;
//! - [`brique`] : le projet et ses briques réutilisables, emboîtables ;
//! - [`pont`] : la traduction du plan vers `ce-circuit`, qui le met à plat pour le moteur ;
//! - [`export`] : le plan en Verilog, pour un autre simulateur ou un FPGA ;
//! - [`importer`] et [`exemples`] : des composants `ce-circuit` devenus projets, dont
//!   l'ordinateur miniature ;
//! - [`format`](mod@format) : formats de fichier numérotés et migrations ;
//! - [`programme`] : le programme de la mémoire morte, écrit dans l'application ;
//! - [`logique`] : table de vérité et expressions simplifiées d'un plan, indices des quêtes ;
//! - [`sauvegarde`] : les projets enregistrés sur le disque ;
//! - [`mise_a_jour`] : les mises à jour automatiques (`self_update`, signatures `zipsign`) ;
//! - [`incident`] : rapports de plantage et d'échec au démarrage ;
//! - [`raccourci`] : les raccourcis du bureau et du menu Démarrer de Windows ;
//! - [`etat`] : l'atelier, qui réunit le tout et porte les actions ;
//! - [`reglages`] : les choix de l'utilisateur (langue, thème, texte, vitesse, lancement, mises à
//!   jour) ;
//! - [`langue`] : l'interface en français ou en anglais, macro [`tr!`].
//!
//! Ce qui s'affiche :
//! - [`routage`] : le tracé des fils ;
//! - [`dessin`] : polices, icônes, animations ;
//! - [`calques`] : les transitions, faites sur ce qui est déjà peint ;
//! - [`plan`] : le circuit dessiné, animé et manipulable ;
//! - [`panneaux`] : barre du haut, briques, quête, infos, chronogramme ;
//! - [`fenetres`] : l'aide (F1) et l'éditeur de programme (Ctrl+P) ;
//! - [`licences`] : les licences des bibliothèques et polices ;
//! - [`accueil`] : l'écran d'accueil, les projets, le parcours et les paramètres ;
//! - [`app`] : la boucle d'une image.

// En premier : la macro `tr!` sert à tous les modules qui suivent.
#[macro_use]
pub mod langue;

pub mod accueil;
pub mod app;
pub mod apparence;
pub mod automate;
pub mod brique;
pub mod calques;
pub mod depannage;
pub mod dessin;
pub mod disposition;
pub mod document;
pub mod economie;
pub mod essai;
pub mod etat;
pub mod exemples;
pub mod export;
pub mod fenetres;
pub mod format;
pub mod importer;
pub mod incident;
pub mod installation;
pub mod licences;
pub mod logique;
pub mod mise_a_jour;
pub mod panneaux;
pub mod parcours;
pub mod photo;
pub mod plan;
pub mod pont;
pub mod programme;
pub mod quete;
pub mod raccourci;
pub mod reglages;
pub mod routage;
pub mod sauvegarde;
pub mod savoir;
pub mod simulation;

// Les traductions de `locales/`, intégrées à l'exécutable (voir [`langue`]).
rust_i18n::i18n!("locales");
