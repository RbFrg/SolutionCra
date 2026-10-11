//! Routes publiques du squelette : `/health` et `/api/config`.

use axum::Json;
use axum::extract::State;
use chrono::NaiveDate;
use serde::Serialize;
use utoipa::ToSchema;

use application::Mode;

use crate::etat::Etat;

#[derive(Debug, Serialize, ToSchema)]
pub struct Health {
    /// Toujours `ok` quand le serveur répond.
    pub status: String,
    /// SHA du commit déployé, comparé par la CI après déploiement (10.5).
    pub version: String,
}

#[derive(Debug, Clone, Copy, Serialize, ToSchema)]
#[serde(rename_all = "snake_case")]
pub enum ModeDto {
    Production,
    Demo,
}

#[derive(Debug, Serialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct Config {
    pub mode: ModeDto,
    /// Présente en démo seulement : date figée (M-04).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub date_figee: Option<NaiveDate>,
    pub build: String,
}

#[utoipa::path(
    get,
    path = "/health",
    tag = "systeme",
    responses((status = 200, description = "Statut et version du build", body = Health))
)]
pub async fn health(State(etat): State<Etat>) -> Json<Health> {
    Json(Health {
        status: "ok".to_owned(),
        version: etat.build.clone(),
    })
}

#[utoipa::path(
    get,
    path = "/api/config",
    tag = "systeme",
    responses((status = 200, description = "Mode d'exécution et build", body = Config))
)]
pub async fn config(State(etat): State<Etat>) -> Json<Config> {
    let mode = etat.app.mode();
    Json(Config {
        mode: match mode {
            Mode::Production => ModeDto::Production,
            Mode::Demo => ModeDto::Demo,
        },
        date_figee: mode.date_figee(),
        build: etat.build.clone(),
    })
}
