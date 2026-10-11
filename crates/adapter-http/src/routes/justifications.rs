//! Justifier une semaine et lever une justification (tranche 3).

use axum::Json;
use axum::extract::{Path, State};
use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use chrono::{DateTime, NaiveDate, Utc};
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;
use uuid::Uuid;

use domain::{EntreeId, Justification, JustificationId};

use crate::dto::TypeFicheDto;
use crate::etat::Etat;
use crate::extraction::Corps;
use crate::probleme::Probleme;
use crate::routes::semaines::lundi;

#[derive(Debug, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct JustifierDto {
    pub lundi: NaiveDate,
    #[serde(rename = "type")]
    pub type_fiche: TypeFicheDto,
    pub motif_id: Uuid,
    /// Obligatoire pour le motif « Autre » (J-04).
    pub commentaire: Option<String>,
}

#[derive(Debug, Serialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct JustificationDto {
    pub id: Uuid,
    pub lundi: NaiveDate,
    pub numero: u32,
    #[serde(rename = "type")]
    pub type_fiche: TypeFicheDto,
    pub motif_id: Uuid,
    pub motif: String,
    pub commentaire: Option<String>,
    pub creee_le: DateTime<Utc>,
    pub levee_le: Option<DateTime<Utc>>,
}

async fn dto(etat: &Etat, j: Justification) -> Result<JustificationDto, Probleme> {
    let noms = etat.app.noms().await?;
    Ok(JustificationDto {
        id: j.id.uuid(),
        lundi: j.lundi.date(),
        numero: j.lundi.numero(),
        type_fiche: j.type_fiche.into(),
        motif_id: j.motif.uuid(),
        motif: noms.nom(j.motif),
        commentaire: j.commentaire,
        creee_le: j.creee_le,
        levee_le: j.levee_le,
    })
}

#[utoipa::path(
    post,
    path = "/api/justifications",
    tag = "justifications",
    request_body = JustifierDto,
    responses(
        (status = 201, body = JustificationDto),
        (status = 422, body = Probleme, content_type = "application/problem+json")
    )
)]
pub async fn justifier(
    State(etat): State<Etat>,
    Corps(corps): Corps<JustifierDto>,
) -> Result<Response, Probleme> {
    let l = lundi(corps.lundi, "lundi")?;
    let j = etat
        .app
        .justifier_semaine(
            l,
            corps.type_fiche.into(),
            EntreeId(corps.motif_id),
            corps.commentaire.as_deref(),
        )
        .await?;
    Ok((StatusCode::CREATED, Json(dto(&etat, j).await?)).into_response())
}

#[utoipa::path(
    delete,
    path = "/api/justifications/{id}",
    tag = "justifications",
    params(("id" = Uuid, Path)),
    responses(
        (status = 200, description = "Justification levée (la ligne reste)", body = JustificationDto),
        (status = 404, body = Probleme, content_type = "application/problem+json")
    )
)]
pub async fn lever(
    State(etat): State<Etat>,
    Path(id): Path<Uuid>,
) -> Result<Json<JustificationDto>, Probleme> {
    let j = etat.app.lever_justification(JustificationId(id)).await?;
    Ok(Json(dto(&etat, j).await?))
}
