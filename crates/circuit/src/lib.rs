//! # ce-circuit
//!
//! Composants hiérarchiques du projet Computer Engineer.
//!
//! - [`modele`] : la bibliothèque de composants et son format JSON.
//! - [`elaboration`] : la mise à plat d'un composant en netlist simulable, avec
//!   résolution des chemins hiérarchiques (`cpu.ual.add.s[3]`).
//! - [`banc`] : un banc de test pour piloter un composant par ses broches.
//! - [`analyse`] : circuit combinatoire ou non, chemin critique, table de vérité,
//!   équivalence de deux composants.
//! - [`verilog`] : export Verilog d'une netlist, pour la comparer à un simulateur établi.
//! - [`mod@reference`] : une bibliothèque de référence construite uniquement en NAND
//!   (portes, additionneurs, verrous, bascules, registre, compteur).
//! - [`grands`] : de grands circuits de mesure (mémoire de plusieurs kilo-octets, multiplicateur,
//!   ferme d'ordinateurs).
//! - [`ordinateur`] : un ordinateur complet miniature en NAND (processeur, mémoires, horloge,
//!   écran, clavier), son assembleur et son modèle de référence.
//! - [`classiques`] : les grands classiques de l'électronique numérique en NAND
//!   (multiplexeurs, décodeurs, multiplicateur, diviseur, bascules, pile, file, liaison
//!   série…), corrigés des quêtes de l'application.

pub mod analyse;
pub mod banc;
pub mod classiques;
pub mod elaboration;
pub mod erreur;
pub mod grands;
pub mod modele;
pub mod ordinateur;
pub mod reference;
pub mod verilog;

pub use banc::Banc;
pub use elaboration::Circuit;
pub use erreur::Erreur;
pub use modele::{Bibliotheque, Broche, Composant, Instance, Sens, FORMAT};
