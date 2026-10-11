//! Squelette : /health, /api/config et service du front (H1, H7, H10 à H13, DEM7).

#![allow(clippy::unwrap_used, clippy::expect_used)]

mod commun;

use application::Mode;
use axum::http::{StatusCode, header};
use commun::*;
use serde_json::json;

#[tokio::test]
async fn get_health_retourne_200() {
    assert_eq!(
        get(&routeur(Mode::Production), "/health").await.statut,
        StatusCode::OK
    );
}

#[tokio::test]
async fn health_expose_statut_et_version_du_build() {
    let r = get(&routeur(Mode::Production), "/health").await;
    assert_eq!(r.json(), json!({ "status": "ok", "version": "3a7f9c1" }));
}

#[tokio::test]
async fn config_expose_le_mode_production_sans_date_figee() {
    let r = get(&routeur(Mode::Production), "/api/config").await;
    assert_eq!(r.statut, StatusCode::OK);
    assert_eq!(
        r.json(),
        json!({ "mode": "production", "build": "3a7f9c1" })
    );
}

#[tokio::test]
async fn config_expose_le_mode_et_la_date_figee() {
    let r = get(&routeur(Mode::Demo), "/api/config").await;
    assert_eq!(
        r.json(),
        json!({ "mode": "demo", "dateFigee": "2026-10-11", "build": "3a7f9c1" })
    );
}

#[tokio::test]
async fn get_racine_sert_index_html_sans_cache() {
    let r = get(&routeur(Mode::Production), "/").await;
    assert_eq!(r.statut, StatusCode::OK);
    assert_eq!(r.entetes[header::CACHE_CONTROL], "no-cache");
    assert!(
        r.entetes[header::CONTENT_TYPE]
            .to_str()
            .unwrap()
            .starts_with("text/html")
    );
    assert!(r.texte().contains("Fiches CRA"));
}

#[tokio::test]
async fn chemin_inconnu_hors_api_retombe_sur_index_html() {
    let r = get(&routeur(Mode::Production), "/fiches/0199f5c2").await;
    assert_eq!(r.statut, StatusCode::OK);
    assert_eq!(r.entetes[header::CACHE_CONTROL], "no-cache");
    assert!(r.texte().contains("Fiches CRA"));
}

#[tokio::test]
async fn chemin_api_inconnu_retourne_404_json_et_non_index_html() {
    let r = get(&routeur(Mode::Production), "/api/n-existe-pas").await;
    assert_eq!(r.statut, StatusCode::NOT_FOUND);
    assert_eq!(r.entetes[header::CONTENT_TYPE], "application/problem+json");
    assert_eq!(r.json()["status"], 404);
}

#[tokio::test]
async fn assets_haches_sont_servis_avec_cache_immuable() {
    let r = get(&routeur(Mode::Production), "/assets/index-3a7f9c1.js").await;
    assert_eq!(r.statut, StatusCode::OK);
    assert_eq!(
        r.entetes[header::CACHE_CONTROL],
        "public, max-age=31536000, immutable"
    );
    assert!(
        r.entetes[header::CONTENT_TYPE]
            .to_str()
            .unwrap()
            .contains("javascript")
    );
}

#[tokio::test]
async fn asset_absent_retourne_404_et_non_index_html() {
    let r = get(&routeur(Mode::Production), "/assets/absent.js").await;
    assert_eq!(r.statut, StatusCode::NOT_FOUND);
}

#[tokio::test]
async fn les_reponses_de_l_api_sont_en_no_store() {
    let r = get(&routeur(Mode::Production), "/api/tableau-de-bord").await;
    assert_eq!(r.entetes[header::CACHE_CONTROL], "no-store");
}
