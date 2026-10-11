//! Dépôts en mémoire (tests, mode démo) et jeu de démonstration (4.5).

pub mod demo;
pub mod depot;

pub use demo::jeu_demo;
pub use depot::{DepotMemoire, EtatMemoire};
