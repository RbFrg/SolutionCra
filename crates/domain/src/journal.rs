//! Journal des événements, en ajout seul (3.9).

use chrono::{DateTime, NaiveDate, Utc};
use serde::{Deserialize, Serialize};

use crate::ids::EvenementId;
use crate::semaine::Lundi;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TypeEvenement {
    FicheCreee,
    StatutModifie,
    FicheModifieeApresTransmission,
    FicheSupprimee,
    SemaineJustifiee,
    JustificationLevee,
    CalendrierModifie,
    ReferentielModifie,
    Import,
    Export,
}

impl TypeEvenement {
    pub const TOUS: [TypeEvenement; 10] = [
        TypeEvenement::FicheCreee,
        TypeEvenement::StatutModifie,
        TypeEvenement::FicheModifieeApresTransmission,
        TypeEvenement::FicheSupprimee,
        TypeEvenement::SemaineJustifiee,
        TypeEvenement::JustificationLevee,
        TypeEvenement::CalendrierModifie,
        TypeEvenement::ReferentielModifie,
        TypeEvenement::Import,
        TypeEvenement::Export,
    ];

    pub fn code(self) -> &'static str {
        match self {
            TypeEvenement::FicheCreee => "fiche_creee",
            TypeEvenement::StatutModifie => "statut_modifie",
            TypeEvenement::FicheModifieeApresTransmission => "fiche_modifiee_apres_transmission",
            TypeEvenement::FicheSupprimee => "fiche_supprimee",
            TypeEvenement::SemaineJustifiee => "semaine_justifiee",
            TypeEvenement::JustificationLevee => "justification_levee",
            TypeEvenement::CalendrierModifie => "calendrier_modifie",
            TypeEvenement::ReferentielModifie => "referentiel_modifie",
            TypeEvenement::Import => "import",
            TypeEvenement::Export => "export",
        }
    }

    pub fn libelle(self) -> &'static str {
        match self {
            TypeEvenement::FicheCreee => "Fiche créée",
            TypeEvenement::StatutModifie => "Statut modifié",
            TypeEvenement::FicheModifieeApresTransmission => "Fiche modifiée après transmission",
            TypeEvenement::FicheSupprimee => "Fiche supprimée",
            TypeEvenement::SemaineJustifiee => "Semaine justifiée",
            TypeEvenement::JustificationLevee => "Justification levée",
            TypeEvenement::CalendrierModifie => "Calendrier modifié",
            TypeEvenement::ReferentielModifie => "Référentiel modifié",
            TypeEvenement::Import => "Import",
            TypeEvenement::Export => "Export",
        }
    }

    pub fn depuis_code(code: &str) -> Option<Self> {
        Self::TOUS.into_iter().find(|t| t.code() == code)
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Evenement {
    pub id: EvenementId,
    /// UTC ; affiché en Europe/Paris.
    pub horodatage: DateTime<Utc>,
    pub type_evenement: TypeEvenement,
    pub cible_type: String,
    /// Sans clé étrangère : l'entrée survit à sa cible (V-06).
    pub cible_id: String,
    pub lundi: Option<Lundi>,
    pub resume: String,
    pub avant: Option<serde_json::Value>,
    pub apres: Option<serde_json::Value>,
}

/// Filtre du journal (V-03) et pagination par 50 au plus récent d'abord (V-08).
#[derive(Debug, Clone, Default, PartialEq)]
pub struct FiltreJournal {
    pub type_evenement: Option<TypeEvenement>,
    pub du: Option<NaiveDate>,
    pub au: Option<NaiveDate>,
    pub lundi: Option<Lundi>,
}

pub const TAILLE_PAGE: usize = 50;
pub const TAILLE_PAGE_MAX: usize = 200;

impl FiltreJournal {
    pub fn accepte(&self, e: &Evenement) -> bool {
        let jour = crate::semaine::date_paris(e.horodatage);
        self.type_evenement.is_none_or(|t| t == e.type_evenement)
            && self.du.is_none_or(|du| jour >= du)
            && self.au.is_none_or(|au| jour <= au)
            && self.lundi.is_none_or(|l| e.lundi == Some(l))
    }
}

/// Trie du plus récent au plus ancien (identifiant en second critère).
pub fn trier_recent_d_abord(evenements: &mut [Evenement]) {
    evenements.sort_by(|a, b| b.horodatage.cmp(&a.horodatage).then(b.id.cmp(&a.id)));
}
