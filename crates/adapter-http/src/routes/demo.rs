//! Routes de la démo : **absentes du routeur de production** (DEM6).

use axum::extract::State;
use axum::http::StatusCode;

use crate::etat::Etat;

#[utoipa::path(
    post,
    path = "/api/demo/entrer",
    tag = "demo",
    responses((status = 204, description = "Démo uniquement : entre sans mot de passe"))
)]
pub async fn entrer() -> StatusCode {
    // L'accès sécurisé (tranche 5) posera ici une session sans secret.
    StatusCode::NO_CONTENT
}

#[utoipa::path(
    post,
    path = "/api/demo/reinitialiser",
    tag = "demo",
    responses((status = 204, description = "Démo uniquement : restaure le jeu de départ (M-05)"))
)]
pub async fn reinitialiser(State(etat): State<Etat>) -> StatusCode {
    etat.app.reinitialiser_demo().await;
    StatusCode::NO_CONTENT
}
