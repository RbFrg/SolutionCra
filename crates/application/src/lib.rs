//! Cas d'usage et ports (4.4). Dépend de `domain` seulement ; les adaptateurs
//! implémentent les ports, `bin/cra-server` les assemble.

pub mod app;
pub mod erreur;
pub mod exports;
pub mod fiches;
pub mod horloge;
pub mod ids;
pub mod journal;
pub mod justifications;
pub mod mode;
pub mod ports;
pub mod referentiels;
pub mod semaines;

pub use app::{App, Noms};
pub use erreur::{ErreurApp, Resultat};
pub use horloge::{Clock, FixedClock, SystemClock};
pub use ids::{IdGenerator, SequenceIds, UuidV7};
pub use mode::Mode;
pub use ports::{
    Depot, EcritureFiche, ErreurDepot, FicheStockee, Lot, PageJournal, Reinitialisable,
};
