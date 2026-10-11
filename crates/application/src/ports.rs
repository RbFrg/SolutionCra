//! Ports de persistance (4.4). Toute écriture passe par un `Lot` validé en une
//! seule transaction : le changement métier **et** son entrée de journal, ou
//! rien (unité de travail). Le journal n'a que deux accès : `Lot::evenements`
//! (ajouter) et `Depot::journal` (lire), jamais de modification (3.9).

use async_trait::async_trait;
use chrono::{DateTime, Utc};

use domain::journal::{Evenement, FiltreJournal};
use domain::{EntreeReferentiel, Fiche, FicheId, Justification, NomReferentiel, SemainePlan};

/// Fiche telle que stockée : la `version` sert la concurrence optimiste (4.1).
#[derive(Debug, Clone, PartialEq)]
pub struct FicheStockee {
    pub fiche: Fiche,
    pub version: u32,
    pub creee_le: DateTime<Utc>,
    pub modifiee_le: DateTime<Utc>,
}

#[derive(Debug, Clone, PartialEq)]
pub enum EcritureFiche {
    /// Nouvelle fiche, version 1 ; refusée si (lundi, type) existe (R-11).
    Creer(Fiche),
    /// Remplace la fiche si la version stockée vaut `version_attendue`.
    Remplacer {
        fiche: Fiche,
        version_attendue: u32,
    },
    Supprimer {
        id: FicheId,
    },
}

/// Écritures d'une unité de travail.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct Lot {
    pub fiches: Vec<EcritureFiche>,
    /// Lignes du plan à poser (remplacent celles des mêmes lundis).
    pub plan: Vec<SemainePlan>,
    /// Justifications créées ou levées (par identifiant).
    pub justifications: Vec<Justification>,
    /// Entrées de référentiel créées ou modifiées (par identifiant).
    pub referentiels: Vec<(NomReferentiel, EntreeReferentiel)>,
    /// Entrées du journal, en ajout seul.
    pub evenements: Vec<Evenement>,
}

#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum ErreurDepot {
    #[error("version périmée (version courante : {version_courante})")]
    Conflit { version_courante: u32 },
    #[error("une fiche existe déjà pour cette semaine et ce type")]
    FicheExistante { fiche_id: FicheId },
    #[error("fiche introuvable")]
    Introuvable,
    #[error("une justification active existe déjà pour cette semaine et ce type")]
    JustificationActiveExistante,
    #[error("nom déjà présent dans le référentiel")]
    NomEnDouble,
    #[error("erreur de stockage : {0}")]
    Technique(String),
}

#[derive(Debug, Clone, PartialEq)]
pub struct PageJournal {
    pub entrees: Vec<Evenement>,
    pub total: usize,
}

#[async_trait]
pub trait Depot: Send + Sync {
    async fn plan(&self) -> Result<Vec<SemainePlan>, ErreurDepot>;
    async fn fiches(&self) -> Result<Vec<FicheStockee>, ErreurDepot>;
    async fn fiche(&self, id: FicheId) -> Result<Option<FicheStockee>, ErreurDepot>;
    async fn justifications(&self) -> Result<Vec<Justification>, ErreurDepot>;
    async fn referentiel(&self, nom: NomReferentiel)
    -> Result<Vec<EntreeReferentiel>, ErreurDepot>;
    /// Page du journal, du plus récent au plus ancien (V-01, V-08).
    async fn journal(
        &self,
        filtre: &FiltreJournal,
        page: usize,
        taille: usize,
    ) -> Result<PageJournal, ErreurDepot>;
    /// Unité de travail : tout le lot ou rien.
    async fn valider(&self, lot: Lot) -> Result<(), ErreurDepot>;
}

/// Réinitialisation du jeu de départ : démo uniquement (M-05).
#[async_trait]
pub trait Reinitialisable: Send + Sync {
    async fn reinitialiser(&self);
}
