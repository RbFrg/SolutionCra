#![allow(dead_code, clippy::unwrap_used, clippy::expect_used)]

use std::sync::Arc;

use adapter_export::Exports;
use adapter_http::{AssetsEnMemoire, Etat, router};
use adapter_memory::{DepotMemoire, EtatMemoire, jeu_demo};
use application::mode::instant_demo;
use application::{App, FixedClock, Mode, SequenceIds};
use axum::Router;
use axum::body::Body;
use axum::http::{HeaderMap, Method, Request, StatusCode};
use http_body_util::BodyExt;
use serde_json::Value;
use tower::ServiceExt;

pub fn routeur_avec(etat: EtatMemoire, mode: Mode) -> Router {
    let depot = DepotMemoire::new(etat);
    let app = App::new(
        Arc::new(depot.clone()),
        Arc::new(FixedClock::new(instant_demo())),
        Arc::new(SequenceIds::new(1_800_000_000_000)),
        Arc::new(Exports),
        mode,
    )
    .avec_reinitialisation(Arc::new(depot));
    let assets = AssetsEnMemoire::default()
        .avec("index.html", "<!doctype html><title>Fiches CRA</title>")
        .avec("assets/index-3a7f9c1.js", "console.log('cra')");
    router(Etat {
        app: Arc::new(app),
        build: "3a7f9c1".to_owned(),
        assets: Arc::new(assets),
        origines_autorisees: Arc::new(vec!["https://cra.example.org".to_owned()]),
    })
}

/// Le jeu de démo, servi en mode démo ou en mode production.
pub fn routeur(mode: Mode) -> Router {
    routeur_avec(jeu_demo(), mode)
}

pub struct Reponse {
    pub statut: StatusCode,
    pub entetes: HeaderMap,
    pub corps: Vec<u8>,
}

impl Reponse {
    pub fn json(&self) -> Value {
        serde_json::from_slice(&self.corps).unwrap()
    }

    pub fn texte(&self) -> String {
        String::from_utf8_lossy(&self.corps).into_owned()
    }
}

pub async fn envoyer(routeur: &Router, requete: Request<Body>) -> Reponse {
    let reponse = routeur.clone().oneshot(requete).await.unwrap();
    let statut = reponse.status();
    let entetes = reponse.headers().clone();
    let corps = reponse
        .into_body()
        .collect()
        .await
        .unwrap()
        .to_bytes()
        .to_vec();
    Reponse {
        statut,
        entetes,
        corps,
    }
}

pub async fn get(routeur: &Router, chemin: &str) -> Reponse {
    envoyer(routeur, Request::get(chemin).body(Body::empty()).unwrap()).await
}

/// Écriture JSON telle que l'envoie le navigateur depuis la même origine.
pub async fn ecrire(routeur: &Router, methode: Method, chemin: &str, corps: Value) -> Reponse {
    envoyer(
        routeur,
        Request::builder()
            .method(methode)
            .uri(chemin)
            .header("content-type", "application/json")
            .header("sec-fetch-site", "same-origin")
            .header("origin", "https://cra.example.org")
            .body(Body::from(corps.to_string()))
            .unwrap(),
    )
    .await
}
