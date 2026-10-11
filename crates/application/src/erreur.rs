use domain::{ErreurValidation, FicheId, Lundi};

use crate::ports::ErreurDepot;

/// Erreurs des cas d'usage, traduites en `problem+json` par l'adaptateur HTTP.
#[derive(Debug, Clone, PartialEq, thiserror::Error)]
pub enum ErreurApp {
    #[error("{0}")]
    Validation(ErreurValidation),
    /// Règle métier refusée (Terminée sans ligne, semaine non justifiable…).
    #[error("{message}")]
    Regle { code: &'static str, message: String },
    #[error("{0}")]
    Introuvable(&'static str),
    #[error("version périmée")]
    Conflit { version_courante: u32 },
    #[error("{message}")]
    FicheExistante { fiche_id: FicheId, message: String },
    #[error("des semaines passées sont touchées")]
    ConfirmationRequise { semaines: Vec<Lundi> },
    #[error("{0}")]
    Technique(String),
}

impl ErreurApp {
    pub fn regle(code: &'static str, message: impl Into<String>) -> Self {
        Self::Regle {
            code,
            message: message.into(),
        }
    }
}

impl From<ErreurDepot> for ErreurApp {
    fn from(erreur: ErreurDepot) -> Self {
        match erreur {
            ErreurDepot::Conflit { version_courante } => Self::Conflit { version_courante },
            ErreurDepot::FicheExistante { fiche_id } => Self::FicheExistante {
                fiche_id,
                message: "Une fiche existe déjà pour cette semaine.".to_owned(),
            },
            ErreurDepot::Introuvable => Self::Introuvable("Fiche introuvable."),
            ErreurDepot::JustificationActiveExistante => {
                Self::regle("deja-justifiee", "Cette semaine est déjà justifiée.")
            }
            ErreurDepot::NomEnDouble => {
                let mut v = ErreurValidation::default();
                v.ajouter("nom", "Ce nom existe déjà dans le référentiel.");
                Self::Validation(v)
            }
            ErreurDepot::Technique(m) => Self::Technique(m),
        }
    }
}

impl From<ErreurValidation> for ErreurApp {
    fn from(erreur: ErreurValidation) -> Self {
        Self::Validation(erreur)
    }
}

pub type Resultat<T> = Result<T, ErreurApp>;
