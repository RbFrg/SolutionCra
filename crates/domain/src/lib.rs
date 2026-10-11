//! Cœur métier : synchrone, pur, sans I/O ni horloge (4.4). Les instants sont
//! des paramètres ; l'heure de référence est Europe/Paris.

pub mod bilan;
pub mod colonnes;
pub mod duree;
pub mod erreur;
pub mod fiche;
pub mod ids;
pub mod journal;
pub mod justification;
pub mod plan;
pub mod referentiel;
pub mod semaine;
pub mod statut;

pub use duree::Duree;
pub use erreur::{ErreurChamp, ErreurValidation};
pub use fiche::{Fiche, Ligne, LigneSaisie, StatutFiche};
pub use ids::{
    CompetenceId, EntreeId, EvenementId, FicheId, JustificationId, LigneId, MissionId, MotifId,
};
pub use journal::{Evenement, FiltreJournal, TypeEvenement};
pub use justification::Justification;
pub use plan::{SemainePlan, TypeAttendu, TypeFiche};
pub use referentiel::{EntreeReferentiel, NomReferentiel};
pub use semaine::{AnneeScolaire, Lundi};
pub use statut::{FiltreType, StatutSemaine};
