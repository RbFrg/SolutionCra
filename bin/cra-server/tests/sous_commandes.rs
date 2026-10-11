//! B1 à B3 : sous-commandes `healthcheck` et `openapi` ; démarrage selon le mode.

#![allow(clippy::unwrap_used, clippy::expect_used)]

use std::sync::Arc;

use adapter_http::AssetsEnMemoire;
use cra_server::{assembler, construire_app, lire_config, outils};

#[tokio::test]
async fn healthcheck_sort_en_succes_si_health_repond_200() {
    let config = lire_config(|nom| (nom == "APP_MODE").then(|| "demo".to_owned())).unwrap();
    let app = construire_app(&config).await.unwrap();
    let routeur = assembler(&config, app, Arc::new(AssetsEnMemoire::default()));
    let ecouteur = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let adresse = ecouteur.local_addr().unwrap();
    tokio::spawn(async move { axum::serve(ecouteur, routeur).await });

    assert!(outils::healthcheck(adresse).await.is_ok());
}

#[tokio::test]
async fn healthcheck_sort_en_erreur_si_le_serveur_est_arrete() {
    // Un port réservé puis libéré : plus personne n'écoute dessus.
    let adresse = {
        let ecouteur = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        ecouteur.local_addr().unwrap()
    };

    assert!(outils::healthcheck(adresse).await.is_err());
}

#[test]
fn openapi_out_ecrit_un_json_utf8_valide() {
    let dossier = tempfile::tempdir().unwrap();
    let chemin = dossier.path().join("openapi.json");

    outils::ecrire_openapi(&chemin).unwrap();

    let octets = std::fs::read(&chemin).unwrap();
    assert_ne!(&octets[..2], &[0xFF, 0xFE], "pas d'UTF-16");
    let texte = String::from_utf8(octets).unwrap();
    let document: serde_json::Value = serde_json::from_str(&texte).unwrap();
    assert!(document["openapi"].as_str().unwrap().starts_with("3.1"));
    assert!(document["paths"]["/health"].is_object());
}

#[test]
fn adresse_d_ecoute_non_specifiee_devient_localhost() {
    let locale = outils::adresse_locale("0.0.0.0:8080".parse().unwrap());
    assert_eq!(locale.to_string(), "127.0.0.1:8080");
}

#[tokio::test]
async fn le_mode_production_cree_la_base_et_amorce_les_motifs() {
    let dossier = tempfile::tempdir().unwrap();
    let chemin = dossier.path().join("cra.db");
    let chemin_texte = chemin.to_string_lossy().into_owned();
    let config = lire_config(|nom| (nom == "CRA_DB_PATH").then(|| chemin_texte.clone())).unwrap();

    let app = construire_app(&config).await.unwrap();

    assert!(chemin.exists());
    let motifs = app
        .referentiel(domain::NomReferentiel::Motifs, false)
        .await
        .unwrap();
    assert_eq!(motifs.len(), 5);
}

#[tokio::test]
async fn le_mode_demo_utilise_les_depots_en_memoire_et_aucune_base() {
    let config = lire_config(|nom| (nom == "APP_MODE").then(|| "demo".to_owned())).unwrap();
    // La base par défaut serait `cra.db` : la démo ne doit jamais l'ouvrir.
    let base = config.db_path.clone();
    let existait = base.exists();

    let app = construire_app(&config).await.unwrap();

    assert_eq!(
        app.lister_fiches(&Default::default()).await.unwrap().len(),
        3
    );
    assert_eq!(base.exists(), existait);
}
