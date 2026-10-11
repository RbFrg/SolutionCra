//! Adaptateur HTTP (axum) : API JSON sous `/api`, `/health`, service du front.
//! Les DTO sont distincts du domaine et portent seuls `utoipa` (4.1).

// Les gestionnaires renvoient `Result<_, Probleme>` : la taille de l'erreur
// (un document problem+json) ne pèse rien face au coût d'une requête HTTP.
#![allow(clippy::result_large_err)]

pub mod assets;
pub mod csrf;
pub mod dto;
pub mod etat;
pub mod extraction;
pub mod probleme;
pub mod routes;

use axum::Router;
use axum::http::{HeaderValue, header};
use axum::middleware;
use axum::routing::{get, patch, post, put};
use tower_http::limit::RequestBodyLimitLayer;
use tower_http::set_header::SetResponseHeaderLayer;
use tower_http::trace::TraceLayer;
use utoipa::OpenApi;

pub use assets::{Assets, AssetsEnMemoire};
pub use etat::Etat;

use application::Mode;
use routes::{bilan, demo, fiches, journal, justifications, referentiels, semaines, systeme};

#[derive(OpenApi)]
#[openapi(
    info(
        title = "Solution CRA Powered by Rust",
        description = "API JSON du suivi hebdomadaire des fiches CRA.",
        version = "1"
    ),
    paths(
        systeme::health,
        systeme::config,
        semaines::tableau_de_bord,
        semaines::semaines,
        semaines::plan,
        semaines::appliquer_plan,
        semaines::alertes,
        fiches::meta_colonnes,
        fiches::lister,
        fiches::creer,
        fiches::lire,
        fiches::sauvegarder,
        fiches::changer_statut,
        fiches::supprimer,
        fiches::exporter,
        justifications::justifier,
        justifications::lever,
        referentiels::lister,
        referentiels::ajouter,
        referentiels::modifier,
        referentiels::importer,
        journal::lire,
        journal::exporter,
        bilan::bilan,
        bilan::bilan_csv,
        bilan::lot,
        bilan::complet,
        demo::entrer,
        demo::reinitialiser,
    ),
    // Les schémas cités seulement par un paramètre de chemin ne sont pas
    // collectés automatiquement : sans eux, openapi-typescript échoue.
    components(schemas(
        probleme::Probleme,
        probleme::ErreurChampDto,
        referentiels::NomReferentielDto
    ))
)]
pub struct ApiDoc;

/// Document OpenAPI 3.1, source de `openapi.json` et des types TypeScript.
pub fn openapi() -> utoipa::openapi::OpenApi {
    ApiDoc::openapi()
}

/// Taille maximale d'un corps de requête (18.5).
const LIMITE_CORPS: usize = 1024 * 1024;

pub fn router(etat: Etat) -> Router {
    let mut api = Router::new()
        .route("/config", get(systeme::config))
        .route("/tableau-de-bord", get(semaines::tableau_de_bord))
        .route("/semaines", get(semaines::semaines))
        .route("/plan", get(semaines::plan).put(semaines::appliquer_plan))
        .route("/alertes", get(semaines::alertes))
        .route("/meta/colonnes", get(fiches::meta_colonnes))
        .route("/fiches", get(fiches::lister).post(fiches::creer))
        .route(
            "/fiches/{id}",
            get(fiches::lire)
                .put(fiches::sauvegarder)
                .delete(fiches::supprimer),
        )
        .route("/fiches/{id}/statut", put(fiches::changer_statut))
        .route("/fiches/{id}/export", get(fiches::exporter))
        .route("/justifications", post(justifications::justifier))
        .route(
            "/justifications/{id}",
            axum::routing::delete(justifications::lever),
        )
        .route(
            "/referentiels/{nom}",
            get(referentiels::lister).post(referentiels::ajouter),
        )
        .route("/referentiels/{nom}/{id}", patch(referentiels::modifier))
        .route("/referentiels/{nom}/import", post(referentiels::importer))
        .route("/journal", get(journal::lire))
        .route("/journal/export", get(journal::exporter))
        .route("/bilan", get(bilan::bilan))
        .route("/bilan/{fichier}", get(bilan::bilan_csv))
        .route("/export/lot", get(bilan::lot))
        .route("/export/complet", get(bilan::complet));

    // Les routes de la démo n'existent pas en production (DEM6).
    if etat.app.mode() == Mode::Demo {
        api = api
            .route("/demo/entrer", post(demo::entrer))
            .route("/demo/reinitialiser", post(demo::reinitialiser));
    }

    let api = api
        .fallback(probleme::api_inconnue)
        .method_not_allowed_fallback(probleme::methode_refusee)
        .layer(SetResponseHeaderLayer::if_not_present(
            header::CACHE_CONTROL,
            HeaderValue::from_static("no-store"),
        ))
        .layer(middleware::from_fn_with_state(etat.clone(), csrf::proteger));

    Router::new()
        .route("/health", get(systeme::health))
        .nest("/api", api)
        .fallback(assets::servir_front)
        .layer(RequestBodyLimitLayer::new(LIMITE_CORPS))
        .layer(TraceLayer::new_for_http())
        .with_state(etat)
}
