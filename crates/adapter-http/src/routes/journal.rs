//! Journal des événements : lecture seule (V-05), export CSV et JSON (V-07).

use axum::Json;
use axum::extract::State;
use axum::response::Response;
use chrono::{DateTime, NaiveDate, Utc};
use serde::{Deserialize, Serialize};
use utoipa::{IntoParams, ToSchema};
use uuid::Uuid;

use application::exports::FormatDonnees;
use domain::journal::{Evenement, FiltreJournal, TypeEvenement};

use crate::etat::Etat;
use crate::extraction::Parametres;
use crate::probleme::Probleme;
use crate::routes::fiches::reponse_fichier;
use crate::routes::semaines::lundi;

#[derive(Debug, Clone, Copy, Deserialize, Serialize, ToSchema)]
#[serde(rename_all = "snake_case")]
pub enum TypeEvenementDto {
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

impl From<TypeEvenement> for TypeEvenementDto {
    fn from(t: TypeEvenement) -> Self {
        match t {
            TypeEvenement::FicheCreee => Self::FicheCreee,
            TypeEvenement::StatutModifie => Self::StatutModifie,
            TypeEvenement::FicheModifieeApresTransmission => Self::FicheModifieeApresTransmission,
            TypeEvenement::FicheSupprimee => Self::FicheSupprimee,
            TypeEvenement::SemaineJustifiee => Self::SemaineJustifiee,
            TypeEvenement::JustificationLevee => Self::JustificationLevee,
            TypeEvenement::CalendrierModifie => Self::CalendrierModifie,
            TypeEvenement::ReferentielModifie => Self::ReferentielModifie,
            TypeEvenement::Import => Self::Import,
            TypeEvenement::Export => Self::Export,
        }
    }
}

impl From<TypeEvenementDto> for TypeEvenement {
    fn from(t: TypeEvenementDto) -> Self {
        match t {
            TypeEvenementDto::FicheCreee => Self::FicheCreee,
            TypeEvenementDto::StatutModifie => Self::StatutModifie,
            TypeEvenementDto::FicheModifieeApresTransmission => {
                Self::FicheModifieeApresTransmission
            }
            TypeEvenementDto::FicheSupprimee => Self::FicheSupprimee,
            TypeEvenementDto::SemaineJustifiee => Self::SemaineJustifiee,
            TypeEvenementDto::JustificationLevee => Self::JustificationLevee,
            TypeEvenementDto::CalendrierModifie => Self::CalendrierModifie,
            TypeEvenementDto::ReferentielModifie => Self::ReferentielModifie,
            TypeEvenementDto::Import => Self::Import,
            TypeEvenementDto::Export => Self::Export,
        }
    }
}

#[derive(Debug, Serialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct EvenementDto {
    pub id: Uuid,
    /// UTC ; le front l'affiche en Europe/Paris.
    pub horodatage: DateTime<Utc>,
    #[serde(rename = "type")]
    pub type_evenement: TypeEvenementDto,
    pub libelle: String,
    pub cible_type: String,
    pub cible_id: String,
    pub lundi: Option<NaiveDate>,
    pub numero: Option<u32>,
    pub resume: String,
    #[schema(value_type = Option<Object>)]
    pub avant: Option<serde_json::Value>,
    #[schema(value_type = Option<Object>)]
    pub apres: Option<serde_json::Value>,
}

impl From<Evenement> for EvenementDto {
    fn from(e: Evenement) -> Self {
        Self {
            id: e.id.uuid(),
            horodatage: e.horodatage,
            type_evenement: e.type_evenement.into(),
            libelle: e.type_evenement.libelle().to_owned(),
            cible_type: e.cible_type,
            cible_id: e.cible_id,
            lundi: e.lundi.map(|l| l.date()),
            numero: e.lundi.map(|l| l.numero()),
            resume: e.resume,
            avant: e.avant,
            apres: e.apres,
        }
    }
}

#[derive(Debug, Serialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct JournalDto {
    pub entrees: Vec<EvenementDto>,
    pub total: usize,
    pub page: usize,
    pub taille: usize,
    pub pages: usize,
}

#[derive(Debug, Deserialize, IntoParams)]
#[into_params(parameter_in = Query)]
pub struct RequeteJournal {
    #[serde(rename = "type")]
    #[param(rename = "type", inline)]
    pub type_evenement: Option<TypeEvenementDto>,
    pub du: Option<NaiveDate>,
    pub au: Option<NaiveDate>,
    /// Lundi de la semaine concernée.
    pub semaine: Option<NaiveDate>,
    pub page: Option<usize>,
    /// 50 par défaut, 200 au plus.
    pub taille: Option<usize>,
}

fn filtre(requete: &RequeteJournal) -> Result<FiltreJournal, Probleme> {
    Ok(FiltreJournal {
        type_evenement: requete.type_evenement.map(Into::into),
        du: requete.du,
        au: requete.au,
        lundi: requete.semaine.map(|s| lundi(s, "semaine")).transpose()?,
    })
}

#[utoipa::path(
    get,
    path = "/api/journal",
    tag = "journal",
    params(RequeteJournal),
    responses((status = 200, body = JournalDto))
)]
pub async fn lire(
    State(etat): State<Etat>,
    Parametres(requete): Parametres<RequeteJournal>,
) -> Result<Json<JournalDto>, Probleme> {
    let (page, numero, taille) = etat
        .app
        .journal(
            &filtre(&requete)?,
            requete.page.unwrap_or(1),
            requete.taille,
        )
        .await?;
    Ok(Json(JournalDto {
        total: page.total,
        page: numero,
        taille,
        pages: page.total.div_ceil(taille).max(1),
        entrees: page.entrees.into_iter().map(EvenementDto::from).collect(),
    }))
}

#[derive(Debug, Clone, Copy, Deserialize, ToSchema)]
#[serde(rename_all = "snake_case")]
pub enum FormatDonneesDto {
    Csv,
    Json,
}

impl From<FormatDonneesDto> for FormatDonnees {
    fn from(f: FormatDonneesDto) -> Self {
        match f {
            FormatDonneesDto::Csv => Self::Csv,
            FormatDonneesDto::Json => Self::Json,
        }
    }
}

#[derive(Debug, Deserialize, IntoParams)]
#[into_params(parameter_in = Query)]
pub struct RequeteExportJournal {
    #[param(inline)]
    pub format: FormatDonneesDto,
    #[serde(rename = "type")]
    #[param(rename = "type", inline)]
    pub type_evenement: Option<TypeEvenementDto>,
    pub du: Option<NaiveDate>,
    pub au: Option<NaiveDate>,
    pub semaine: Option<NaiveDate>,
}

#[utoipa::path(
    get,
    path = "/api/journal/export",
    tag = "journal",
    params(RequeteExportJournal),
    responses((status = 200, description = "Journal en CSV ou JSON", content_type = "application/octet-stream"))
)]
pub async fn exporter(
    State(etat): State<Etat>,
    Parametres(requete): Parametres<RequeteExportJournal>,
) -> Result<Response, Probleme> {
    let f = filtre(&RequeteJournal {
        type_evenement: requete.type_evenement,
        du: requete.du,
        au: requete.au,
        semaine: requete.semaine,
        page: None,
        taille: None,
    })?;
    let fichier = etat.app.exporter_journal(&f, requete.format.into()).await?;
    Ok(reponse_fichier(fichier, false))
}
