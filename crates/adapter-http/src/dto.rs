//! Énumérations de l'API (snake_case) et conversions depuis le domaine.
//! Le domaine ne porte pas `utoipa` : seuls les DTO le font (4.1).

use chrono::{DateTime, NaiveDate, Utc};
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;
use uuid::Uuid;

use domain::statut::SemaineCalculee;
use domain::{FiltreType, StatutFiche, StatutSemaine, TypeAttendu, TypeFiche};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "snake_case")]
pub enum TypeFicheDto {
    Entreprise,
    Formation,
}

impl From<TypeFiche> for TypeFicheDto {
    fn from(t: TypeFiche) -> Self {
        match t {
            TypeFiche::Entreprise => Self::Entreprise,
            TypeFiche::Formation => Self::Formation,
        }
    }
}

impl From<TypeFicheDto> for TypeFiche {
    fn from(t: TypeFicheDto) -> Self {
        match t {
            TypeFicheDto::Entreprise => Self::Entreprise,
            TypeFicheDto::Formation => Self::Formation,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "snake_case")]
pub enum TypeAttenduDto {
    Entreprise,
    Formation,
    Aucune,
}

impl From<TypeAttendu> for TypeAttenduDto {
    fn from(t: TypeAttendu) -> Self {
        match t {
            TypeAttendu::Entreprise => Self::Entreprise,
            TypeAttendu::Formation => Self::Formation,
            TypeAttendu::Aucune => Self::Aucune,
        }
    }
}

impl From<TypeAttenduDto> for TypeAttendu {
    fn from(t: TypeAttenduDto) -> Self {
        match t {
            TypeAttenduDto::Entreprise => Self::Entreprise,
            TypeAttenduDto::Formation => Self::Formation,
            TypeAttenduDto::Aucune => Self::Aucune,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "snake_case")]
pub enum StatutFicheDto {
    Brouillon,
    Terminee,
    Transmise,
    Validee,
}

impl From<StatutFiche> for StatutFicheDto {
    fn from(s: StatutFiche) -> Self {
        match s {
            StatutFiche::Brouillon => Self::Brouillon,
            StatutFiche::Terminee => Self::Terminee,
            StatutFiche::Transmise => Self::Transmise,
            StatutFiche::Validee => Self::Validee,
        }
    }
}

impl From<StatutFicheDto> for StatutFiche {
    fn from(s: StatutFicheDto) -> Self {
        match s {
            StatutFicheDto::Brouillon => Self::Brouillon,
            StatutFicheDto::Terminee => Self::Terminee,
            StatutFicheDto::Transmise => Self::Transmise,
            StatutFicheDto::Validee => Self::Validee,
        }
    }
}

/// Statut de semaine **calculé par le serveur** ; le front ne le recalcule jamais.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "snake_case")]
pub enum StatutSemaineDto {
    Faite,
    AFaire,
    EnRetard,
    AVenir,
    NonAttendue,
    Justifiee,
}

impl From<StatutSemaine> for StatutSemaineDto {
    fn from(s: StatutSemaine) -> Self {
        match s {
            StatutSemaine::Faite => Self::Faite,
            StatutSemaine::AFaire => Self::AFaire,
            StatutSemaine::EnRetard => Self::EnRetard,
            StatutSemaine::AVenir => Self::AVenir,
            StatutSemaine::NonAttendue => Self::NonAttendue,
            StatutSemaine::Justifiee => Self::Justifiee,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "snake_case")]
pub enum FiltreTypeDto {
    #[default]
    Toutes,
    Entreprise,
    Formation,
}

impl From<FiltreTypeDto> for FiltreType {
    fn from(f: FiltreTypeDto) -> Self {
        match f {
            FiltreTypeDto::Toutes => Self::Toutes,
            FiltreTypeDto::Entreprise => Self::Entreprise,
            FiltreTypeDto::Formation => Self::Formation,
        }
    }
}

/// Une semaine et son statut, pour le calendrier, la frise et le plan.
#[derive(Debug, Clone, Serialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct SemaineDto {
    pub lundi: NaiveDate,
    pub dimanche: NaiveDate,
    /// Numéro ISO 8601.
    pub numero: u32,
    pub type_attendu: TypeAttenduDto,
    pub libelle: Option<String>,
    /// Type évalué : celui du filtre, ou celui du plan pour « toutes ».
    #[serde(rename = "type")]
    pub type_fiche: Option<TypeFicheDto>,
    pub statut: StatutSemaineDto,
    pub fiche_id: Option<Uuid>,
    pub statut_fiche: Option<StatutFicheDto>,
    pub justification_id: Option<Uuid>,
    pub motif_id: Option<Uuid>,
    pub jours_retard: Option<i64>,
    /// Dimanche 23h59m59, heure de Paris.
    pub echeance: DateTime<Utc>,
}

impl From<&SemaineCalculee> for SemaineDto {
    fn from(s: &SemaineCalculee) -> Self {
        Self {
            lundi: s.lundi.date(),
            dimanche: s.lundi.dimanche(),
            numero: s.lundi.numero(),
            type_attendu: s.type_attendu.into(),
            libelle: s.libelle.clone(),
            type_fiche: s.type_evalue.map(Into::into),
            statut: s.statut.into(),
            fiche_id: s.fiche.map(|(id, _)| id.uuid()),
            statut_fiche: s.fiche.map(|(_, st)| st.into()),
            justification_id: s.justification.map(|(id, _)| id.uuid()),
            motif_id: s.justification.map(|(_, m)| m.uuid()),
            jours_retard: s.jours_retard,
            echeance: s.lundi.echeance(),
        }
    }
}

/// Minutes vers heures décimales (3,5 h).
pub fn heures(minutes: u32) -> f64 {
    f64::from(minutes) / 60.0
}
