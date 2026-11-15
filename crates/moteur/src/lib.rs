//! # ce-moteur
//!
//! Noyau de simulation logique du projet Computer Engineer.
//!
//! - [`valeur`] : la logique à quatre valeurs (`0`, `1`, `X`, `Z`), la porte NAND,
//!   le tampon trois états et la résolution des pilotes multiples.
//! - [`netlist`] : un circuit « à plat », c'est-à-dire des portes reliées par des nets.
//! - [`simulateur`] : le simulateur événementiel optimisé.
//! - [`specification`] : la sémantique de référence, écrite de la façon la plus naïve
//!   possible. Elle sert de définition : le simulateur optimisé doit donner exactement
//!   les mêmes valeurs qu'elle, à chaque instant et sur chaque net.
//! - [`alea`] : un générateur pseudo-aléatoire déterministe pour les tests.
//!
//! Le modèle est décrit et démontré dans `docs/semantique.md`.

pub mod alea;
pub mod netlist;
pub mod simulateur;
pub mod specification;
pub mod valeur;

pub use netlist::{ErreurNetlist, GenrePorte, Horloge, NetId, Netlist, Porte};
pub use simulateur::{
    Changement, Options, Simulateur, Stabilite, Statistiques, StatistiquesSommeil,
};
pub use valeur::Valeur;
