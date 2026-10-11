//! Tableau de bord, semaines, plan d'alternance et alertes (tranche 1).

use axum::Json;
use axum::extract::State;
use chrono::{DateTime, NaiveDate, Utc};
use serde::{Deserialize, Serialize};
use utoipa::{IntoParams, ToSchema};

use domain::semaine::date_paris;
use domain::{AnneeScolaire, Lundi};

use crate::dto::{FiltreTypeDto, SemaineDto, TypeAttenduDto, TypeFicheDto};
use crate::etat::Etat;
use crate::extraction::{Corps, Parametres};
use crate::probleme::Probleme;

#[derive(Debug, Deserialize, IntoParams)]
#[into_params(parameter_in = Query)]
pub struct RequeteAnnee {
    /// Année scolaire de début (2026 ou 2027) ; par défaut, celle d'aujourd'hui.
    pub annee: Option<i32>,
    /// Filtre de type ; `toutes` par défaut.
    #[serde(rename = "type", default)]
    #[param(rename = "type", inline)]
    pub type_fiche: FiltreTypeDto,
}

pub(crate) fn annee(etat: &Etat, annee: Option<i32>) -> Result<AnneeScolaire, Probleme> {
    match annee {
        Some(a) => AnneeScolaire::new(a).map_err(|e| Probleme::requete_invalide(e.to_string())),
        None => Ok(AnneeScolaire::de(date_paris(etat.app.maintenant()))),
    }
}

pub(crate) fn lundi(date: NaiveDate, champ: &str) -> Result<Lundi, Probleme> {
    Lundi::new(date).map_err(|e| {
        let mut v = domain::ErreurValidation::default();
        v.ajouter(champ, e.to_string());
        application::ErreurApp::Validation(v).into()
    })
}

#[derive(Debug, Serialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct AFaireDto {
    pub lundi: NaiveDate,
    pub numero: u32,
    #[serde(rename = "type")]
    pub type_fiche: TypeFicheDto,
    pub echeance: DateTime<Utc>,
}

#[derive(Debug, Serialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct IndicateursDto {
    pub attendues: u32,
    pub faites: u32,
    pub justifiees: u32,
    pub en_retard: u32,
    pub a_faire: u32,
    /// Dénominateur du taux : attendues moins justifiées (J-05).
    pub comptees: u32,
    /// Faites / comptées ; absent sans semaine attendue.
    pub taux_faites: Option<f64>,
    pub a_faire_cette_semaine: Option<AFaireDto>,
}

#[derive(Debug, Serialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct TableauDeBordDto {
    /// Date de Paris selon le serveur (figée en démo).
    pub aujourdhui: NaiveDate,
    pub semaine_courante: Option<NaiveDate>,
    pub annee: i32,
    pub indicateurs: IndicateursDto,
    /// Semaines en retard, de la plus ancienne à la plus récente.
    pub retards: Vec<SemaineDto>,
    /// Toutes les semaines de l'année (calendrier et frise).
    pub semaines: Vec<SemaineDto>,
}

#[utoipa::path(
    get,
    path = "/api/tableau-de-bord",
    tag = "semaines",
    params(RequeteAnnee),
    responses(
        (status = 200, body = TableauDeBordDto),
        (status = 400, body = Probleme, content_type = "application/problem+json")
    )
)]
pub async fn tableau_de_bord(
    State(etat): State<Etat>,
    Parametres(requete): Parametres<RequeteAnnee>,
) -> Result<Json<TableauDeBordDto>, Probleme> {
    let a = annee(&etat, requete.annee)?;
    let tdb = etat
        .app
        .tableau_de_bord(a, requete.type_fiche.into())
        .await?;
    let c = tdb.compteurs;
    Ok(Json(TableauDeBordDto {
        aujourdhui: tdb.aujourdhui,
        semaine_courante: tdb.semaine_courante.map(Lundi::date),
        annee: a.valeur(),
        indicateurs: IndicateursDto {
            attendues: c.attendues,
            faites: c.faites,
            justifiees: c.justifiees,
            en_retard: c.en_retard,
            a_faire: c.a_faire,
            comptees: c.attendues - c.justifiees,
            taux_faites: c.taux_faites,
            a_faire_cette_semaine: tdb.a_faire_cette_semaine.map(|s| AFaireDto {
                lundi: s.lundi.date(),
                numero: s.lundi.numero(),
                type_fiche: s.type_fiche.into(),
                echeance: s.echeance,
            }),
        },
        retards: tdb.retards.iter().map(SemaineDto::from).collect(),
        semaines: tdb.semaines.iter().map(SemaineDto::from).collect(),
    }))
}

#[utoipa::path(
    get,
    path = "/api/semaines",
    tag = "semaines",
    params(RequeteAnnee),
    responses((status = 200, body = Vec<SemaineDto>))
)]
pub async fn semaines(
    State(etat): State<Etat>,
    Parametres(requete): Parametres<RequeteAnnee>,
) -> Result<Json<Vec<SemaineDto>>, Probleme> {
    let a = annee(&etat, requete.annee)?;
    let semaines = etat.app.semaines(a, requete.type_fiche.into()).await?;
    Ok(Json(semaines.iter().map(SemaineDto::from).collect()))
}

#[derive(Debug, Deserialize, IntoParams)]
#[into_params(parameter_in = Query)]
pub struct RequetePlan {
    pub annee: Option<i32>,
}

#[derive(Debug, Serialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct PlanDto {
    pub annee: i32,
    pub aujourdhui: NaiveDate,
    /// 52 ou 53 semaines ; une semaine sans ligne vaut Entreprise (P-04).
    pub semaines: Vec<SemaineDto>,
}

#[utoipa::path(
    get,
    path = "/api/plan",
    tag = "semaines",
    params(RequetePlan),
    responses((status = 200, body = PlanDto))
)]
pub async fn plan(
    State(etat): State<Etat>,
    Parametres(requete): Parametres<RequetePlan>,
) -> Result<Json<PlanDto>, Probleme> {
    let a = annee(&etat, requete.annee)?;
    let semaines = etat.app.semaines(a, domain::FiltreType::Toutes).await?;
    Ok(Json(PlanDto {
        annee: a.valeur(),
        aujourdhui: date_paris(etat.app.maintenant()),
        semaines: semaines.iter().map(SemaineDto::from).collect(),
    }))
}

#[derive(Debug, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct AppliquerPlanDto {
    pub du: NaiveDate,
    pub au: NaiveDate,
    pub type_attendu: TypeAttenduDto,
    pub libelle: Option<String>,
    /// À renvoyer à `true` après un 409 `confirmation-requise` (P-06).
    #[serde(default)]
    pub confirmer: bool,
}

#[derive(Debug, Serialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct SemainePlanDto {
    pub lundi: NaiveDate,
    pub numero: u32,
    pub type_attendu: TypeAttenduDto,
    pub libelle: Option<String>,
}

#[utoipa::path(
    put,
    path = "/api/plan",
    tag = "semaines",
    request_body = AppliquerPlanDto,
    responses(
        (status = 200, body = Vec<SemainePlanDto>),
        (status = 409, description = "confirmation-requise", body = Probleme, content_type = "application/problem+json"),
        (status = 422, body = Probleme, content_type = "application/problem+json")
    )
)]
pub async fn appliquer_plan(
    State(etat): State<Etat>,
    Corps(corps): Corps<AppliquerPlanDto>,
) -> Result<Json<Vec<SemainePlanDto>>, Probleme> {
    let du = lundi(corps.du, "du")?;
    let au = lundi(corps.au, "au")?;
    let lignes = etat
        .app
        .appliquer_plan(
            du,
            au,
            corps.type_attendu.into(),
            corps.libelle.as_deref(),
            corps.confirmer,
        )
        .await?;
    Ok(Json(
        lignes
            .into_iter()
            .map(|l| SemainePlanDto {
                lundi: l.lundi.date(),
                numero: l.lundi.numero(),
                type_attendu: l.type_attendu.into(),
                libelle: l.libelle,
            })
            .collect(),
    ))
}

#[derive(Debug, Serialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct AlerteDto {
    pub lundi: NaiveDate,
    pub numero: u32,
    #[serde(rename = "type")]
    pub type_fiche: Option<TypeFicheDto>,
    pub jours_retard: Option<i64>,
    pub echeance: DateTime<Utc>,
}

/// Document versionné lu par n8n (4.6) : un changement incompatible crée un nouveau chemin.
#[derive(Debug, Serialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct AlertesDto {
    pub schema_version: u32,
    pub genere_le: DateTime<Utc>,
    pub aujourdhui: NaiveDate,
    pub en_retard: Vec<AlerteDto>,
    pub a_faire: Vec<AlerteDto>,
}

#[utoipa::path(
    get,
    path = "/api/alertes",
    tag = "semaines",
    responses((status = 200, body = AlertesDto))
)]
pub async fn alertes(State(etat): State<Etat>) -> Result<Json<AlertesDto>, Probleme> {
    let a = etat.app.alertes().await?;
    let dto = |s: &domain::statut::SemaineCalculee| AlerteDto {
        lundi: s.lundi.date(),
        numero: s.lundi.numero(),
        type_fiche: s.type_evalue.map(Into::into),
        jours_retard: s.jours_retard,
        echeance: s.lundi.echeance(),
    };
    Ok(Json(AlertesDto {
        schema_version: 1,
        genere_le: a.genere_le,
        aujourdhui: a.aujourdhui,
        en_retard: a.en_retard.iter().map(dto).collect(),
        a_faire: a.a_faire.iter().map(dto).collect(),
    }))
}
