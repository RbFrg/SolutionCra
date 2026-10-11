//! Erreurs de l'API en `application/problem+json` (RFC 9457, 4.1).

use axum::Json;
use axum::http::{HeaderValue, StatusCode, header};
use axum::response::{IntoResponse, Response};
use chrono::NaiveDate;
use serde::Serialize;
use utoipa::ToSchema;
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct ErreurChampDto {
    /// Chemin du champ, par exemple `lignes[2].mission`.
    pub field: String,
    pub message: String,
}

#[derive(Debug, Clone, Serialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct Probleme {
    #[serde(rename = "type")]
    pub type_probleme: String,
    pub title: String,
    pub status: u16,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub detail: Option<String>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub errors: Vec<ErreurChampDto>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub version_courante: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub fiche_id: Option<Uuid>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub semaines_passees_touchees: Option<Vec<NaiveDate>>,
}

impl Probleme {
    pub fn new(statut: StatusCode, code: &str, titre: &str) -> Self {
        Self {
            type_probleme: format!("urn:cra:probleme:{code}"),
            title: titre.to_owned(),
            status: statut.as_u16(),
            detail: None,
            errors: Vec::new(),
            version_courante: None,
            fiche_id: None,
            semaines_passees_touchees: None,
        }
    }

    pub fn detail(mut self, detail: impl Into<String>) -> Self {
        self.detail = Some(detail.into());
        self
    }

    pub fn introuvable(detail: &str) -> Self {
        Self::new(
            StatusCode::NOT_FOUND,
            "introuvable",
            "Ressource introuvable",
        )
        .detail(detail)
    }

    pub fn requete_invalide(detail: impl Into<String>) -> Self {
        Self::new(
            StatusCode::BAD_REQUEST,
            "requete-invalide",
            "Requête invalide",
        )
        .detail(detail)
    }

    pub fn interne() -> Self {
        Self::new(
            StatusCode::INTERNAL_SERVER_ERROR,
            "erreur-interne",
            "Erreur interne",
        )
        .detail("Une erreur inattendue s'est produite. Rechargez la page.")
    }
}

impl IntoResponse for Probleme {
    fn into_response(self) -> Response {
        let statut = StatusCode::from_u16(self.status).unwrap_or(StatusCode::INTERNAL_SERVER_ERROR);
        let mut reponse = (statut, Json(self)).into_response();
        reponse.headers_mut().insert(
            header::CONTENT_TYPE,
            HeaderValue::from_static("application/problem+json"),
        );
        reponse
    }
}

/// Repli de `/api/*` : jamais l'interface, toujours un 404 JSON (H12).
pub async fn api_inconnue() -> Probleme {
    Probleme::introuvable("Cette route d'API n'existe pas.")
}

/// Méthode non prévue sur une route existante (le journal ne se modifie pas : JRN9).
pub async fn methode_refusee() -> Probleme {
    Probleme::new(
        StatusCode::METHOD_NOT_ALLOWED,
        "methode-refusee",
        "Méthode non autorisée",
    )
}
