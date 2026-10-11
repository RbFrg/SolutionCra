//! Routes métier sans ouvrir de port : H2 à H9, H14 à H17, DEM6, JRN9, exports.

#![allow(clippy::unwrap_used, clippy::expect_used)]

mod commun;

use std::io::Read;

use application::Mode;
use axum::body::Body;
use axum::http::{Method, Request, StatusCode, header};
use commun::*;
use serde_json::{Value, json};

async fn tdb(routeur: &axum::Router) -> Value {
    get(routeur, "/api/tableau-de-bord?annee=2026&type=toutes")
        .await
        .json()
}

async fn fiche_s38(routeur: &axum::Router) -> Value {
    let liste = get(routeur, "/api/fiches").await.json();
    let id = liste[0]["id"].as_str().unwrap().to_owned();
    get(routeur, &format!("/api/fiches/{id}")).await.json()
}

#[tokio::test]
async fn le_tableau_de_bord_expose_les_indicateurs_calcules_par_le_serveur() {
    let r = routeur(Mode::Demo);
    let t = tdb(&r).await;
    assert_eq!(t["aujourdhui"], "2026-10-11");
    assert_eq!(t["indicateurs"]["attendues"], 6);
    assert_eq!(t["indicateurs"]["faites"], 3);
    assert_eq!(t["indicateurs"]["tauxFaites"], 0.6);
    assert_eq!(
        t["indicateurs"]["aFaireCetteSemaine"],
        json!({ "lundi": "2026-10-05", "numero": 41, "type": "entreprise", "echeance": "2026-10-11T21:59:59Z" })
    );
    assert_eq!(t["retards"][0]["lundi"], "2026-09-21");
    assert_eq!(t["retards"][0]["joursRetard"], 14);
    assert_eq!(t["semaines"].as_array().unwrap().len(), 52);
}

#[tokio::test]
async fn une_annee_hors_du_suivi_est_refusee_en_problem_json() {
    let r = get(&routeur(Mode::Demo), "/api/tableau-de-bord?annee=2030").await;
    assert_eq!(r.statut, StatusCode::BAD_REQUEST);
    assert_eq!(r.entetes[header::CONTENT_TYPE], "application/problem+json");
}

#[tokio::test]
async fn get_meta_colonnes_retourne_les_six_libelles_du_type() {
    let r = get(&routeur(Mode::Demo), "/api/meta/colonnes?type=formation").await;
    assert_eq!(
        r.json()["colonnes"][5],
        "Retour du tuteur / Echanges avec tuteur"
    );
    assert_eq!(r.json()["colonnes"].as_array().unwrap().len(), 6);
}

#[tokio::test]
async fn post_fiche_valide_retourne_201_et_location() {
    let r = routeur(Mode::Demo);
    let reponse = ecrire(
        &r,
        Method::POST,
        "/api/fiches",
        json!({ "lundi": "2026-10-05", "type": "entreprise" }),
    )
    .await;
    assert_eq!(reponse.statut, StatusCode::CREATED);
    let id = reponse.json()["id"].as_str().unwrap().to_owned();
    assert_eq!(
        reponse.entetes[header::LOCATION],
        format!("/api/fiches/{id}")
    );
    assert_eq!(reponse.json()["version"], 1);
    assert_eq!(reponse.json()["statut"], "brouillon");
}

#[tokio::test]
async fn une_fiche_existante_retourne_409_avec_son_identifiant() {
    let r = routeur(Mode::Demo);
    let reponse = ecrire(
        &r,
        Method::POST,
        "/api/fiches",
        json!({ "lundi": "2026-09-14", "type": "entreprise" }),
    )
    .await;
    assert_eq!(reponse.statut, StatusCode::CONFLICT);
    assert_eq!(reponse.json()["type"], "urn:cra:probleme:fiche-existante");
    assert_eq!(
        reponse.json()["detail"],
        "Une fiche entreprise existe déjà pour la semaine 38."
    );
    assert!(reponse.json()["ficheId"].is_string());
}

#[tokio::test]
async fn put_avec_duree_hors_quart_d_heure_retourne_422_avec_le_champ() {
    let r = routeur(Mode::Demo);
    let fiche = fiche_s38(&r).await;
    let ligne = &fiche["lignes"][0];
    let corps = json!({
        "version": fiche["version"],
        "lignes": [{
            "id": ligne["id"], "date": "2026-09-15", "missionId": ligne["missionId"],
            "competenceIds": [], "dureeHeures": 1.1, "etapes": "x", "echangesTuteur": null
        }]
    });
    let reponse = ecrire(
        &r,
        Method::PUT,
        &format!("/api/fiches/{}", fiche["id"].as_str().unwrap()),
        corps,
    )
    .await;
    assert_eq!(reponse.statut, StatusCode::UNPROCESSABLE_ENTITY);
    assert_eq!(
        reponse.entetes[header::CONTENT_TYPE],
        "application/problem+json"
    );
    assert_eq!(
        reponse.json()["errors"][0]["field"],
        "lignes[0].dureeHeures"
    );
}

#[tokio::test]
async fn put_avec_version_perimee_retourne_409() {
    let r = routeur(Mode::Demo);
    let fiche = fiche_s38(&r).await;
    let corps = json!({ "version": 1, "lignes": [] });
    let reponse = ecrire(
        &r,
        Method::PUT,
        &format!("/api/fiches/{}", fiche["id"].as_str().unwrap()),
        corps,
    )
    .await;
    assert_eq!(reponse.statut, StatusCode::CONFLICT);
    assert_eq!(
        reponse.json()["type"],
        "urn:cra:probleme:conflit-de-version"
    );
    assert_eq!(reponse.json()["versionCourante"], 3);
}

#[tokio::test]
async fn fiche_inconnue_retourne_404_problem_json() {
    let r = get(
        &routeur(Mode::Demo),
        "/api/fiches/0199f5c2-7b3a-7c1e-9a52-3f0d2b6e8a41",
    )
    .await;
    assert_eq!(r.statut, StatusCode::NOT_FOUND);
    assert_eq!(r.entetes[header::CONTENT_TYPE], "application/problem+json");
}

#[tokio::test]
async fn marquer_terminee_sans_ligne_retourne_422_avec_le_message() {
    let r = routeur(Mode::Demo);
    let creee = ecrire(
        &r,
        Method::POST,
        "/api/fiches",
        json!({ "lundi": "2026-10-05", "type": "entreprise" }),
    )
    .await
    .json();
    let reponse = ecrire(
        &r,
        Method::PUT,
        &format!("/api/fiches/{}/statut", creee["id"].as_str().unwrap()),
        json!({ "statut": "terminee", "version": 1 }),
    )
    .await;
    assert_eq!(reponse.statut, StatusCode::UNPROCESSABLE_ENTITY);
    assert_eq!(
        reponse.json()["detail"],
        "Ajoutez au moins une ligne avant de terminer la fiche."
    );
}

#[tokio::test]
async fn justifier_puis_lever_met_a_jour_le_tableau_de_bord() {
    let r = routeur(Mode::Demo);
    let motifs = get(&r, "/api/referentiels/motifs").await.json();
    let conges = motifs
        .as_array()
        .unwrap()
        .iter()
        .find(|m| m["nom"] == "Congés")
        .unwrap()["id"]
        .clone();
    let j = ecrire(
        &r,
        Method::POST,
        "/api/justifications",
        json!({ "lundi": "2026-09-21", "type": "entreprise", "motifId": conges }),
    )
    .await;
    assert_eq!(j.statut, StatusCode::CREATED);
    assert_eq!(tdb(&r).await["indicateurs"]["enRetard"], 0);

    let id = j.json()["id"].as_str().unwrap().to_owned();
    let levee = envoyer(
        &r,
        Request::delete(format!("/api/justifications/{id}"))
            .header("sec-fetch-site", "same-origin")
            .body(Body::empty())
            .unwrap(),
    )
    .await;
    assert_eq!(levee.statut, StatusCode::OK);
    assert!(levee.json()["leveeLe"].is_string());
    assert_eq!(tdb(&r).await["indicateurs"]["enRetard"], 1);
}

#[tokio::test]
async fn modifier_une_semaine_passee_sans_confirmation_retourne_409_confirmation_requise() {
    let r = routeur(Mode::Demo);
    let corps = json!({ "du": "2026-09-21", "au": "2026-09-21", "typeAttendu": "aucune" });
    let reponse = ecrire(&r, Method::PUT, "/api/plan", corps).await;
    assert_eq!(reponse.statut, StatusCode::CONFLICT);
    assert_eq!(
        reponse.json()["semainesPasseesTouchees"],
        json!(["2026-09-21"])
    );

    let confirme = json!({ "du": "2026-09-21", "au": "2026-09-21", "typeAttendu": "aucune", "confirmer": true });
    assert_eq!(
        ecrire(&r, Method::PUT, "/api/plan", confirme).await.statut,
        StatusCode::OK
    );
}

#[tokio::test]
async fn ecriture_avec_content_type_non_json_retourne_415() {
    let r = routeur(Mode::Demo);
    let reponse = envoyer(
        &r,
        Request::post("/api/fiches")
            .header("content-type", "text/plain")
            .body(Body::from("lundi=2026-10-05"))
            .unwrap(),
    )
    .await;
    assert_eq!(reponse.statut, StatusCode::UNSUPPORTED_MEDIA_TYPE);
}

#[tokio::test]
async fn ecriture_avec_sec_fetch_site_cross_site_retourne_403() {
    let r = routeur(Mode::Demo);
    let reponse = envoyer(
        &r,
        Request::post("/api/fiches")
            .header("content-type", "application/json")
            .header("sec-fetch-site", "cross-site")
            .body(Body::from(r#"{"lundi":"2026-10-05","type":"entreprise"}"#))
            .unwrap(),
    )
    .await;
    assert_eq!(reponse.statut, StatusCode::FORBIDDEN);
    assert_eq!(reponse.json()["type"], "urn:cra:probleme:csrf");
}

#[tokio::test]
async fn ecriture_avec_origin_etrangere_retourne_403() {
    let r = routeur(Mode::Demo);
    let reponse = envoyer(
        &r,
        Request::post("/api/fiches")
            .header("content-type", "application/json")
            .header("origin", "https://pirate.example.com")
            .body(Body::from(r#"{"lundi":"2026-10-05","type":"entreprise"}"#))
            .unwrap(),
    )
    .await;
    assert_eq!(reponse.statut, StatusCode::FORBIDDEN);
}

#[tokio::test]
async fn ecriture_de_la_meme_origine_que_l_hote_est_acceptee() {
    // Non-régression : le front servi par le binaire écrit sur sa propre
    // origine, absente de CRA_ALLOWED_ORIGINS ; c'était un 403.
    let r = routeur(Mode::Demo);
    let reponse = envoyer(
        &r,
        Request::post("/api/fiches")
            .header("content-type", "application/json")
            .header("host", "127.0.0.1:8080")
            .header("origin", "http://127.0.0.1:8080")
            .header("sec-fetch-site", "same-origin")
            .body(Body::from(r#"{"lundi":"2026-10-05","type":"entreprise"}"#))
            .unwrap(),
    )
    .await;
    assert_eq!(reponse.statut, StatusCode::CREATED);
}

#[tokio::test]
async fn une_origine_etrangere_reste_refusee_meme_avec_un_host() {
    let r = routeur(Mode::Demo);
    let reponse = envoyer(
        &r,
        Request::post("/api/fiches")
            .header("content-type", "application/json")
            .header("host", "127.0.0.1:8080")
            .header("origin", "https://pirate.example.com")
            .body(Body::from(r#"{"lundi":"2026-10-05","type":"entreprise"}"#))
            .unwrap(),
    )
    .await;
    assert_eq!(reponse.statut, StatusCode::FORBIDDEN);
}

#[tokio::test]
async fn ecriture_sans_en_tetes_de_navigateur_est_acceptee() {
    let r = routeur(Mode::Demo);
    let reponse = envoyer(
        &r,
        Request::post("/api/fiches")
            .header("content-type", "application/json")
            .body(Body::from(r#"{"lundi":"2026-10-05","type":"entreprise"}"#))
            .unwrap(),
    )
    .await;
    assert_eq!(reponse.statut, StatusCode::CREATED);
}

#[tokio::test]
async fn les_routes_demo_n_existent_pas_en_production() {
    let r = routeur(Mode::Production);
    let reponse = ecrire(&r, Method::POST, "/api/demo/reinitialiser", json!({})).await;
    assert_eq!(reponse.statut, StatusCode::NOT_FOUND);
    assert_eq!(
        reponse.entetes[header::CONTENT_TYPE],
        "application/problem+json"
    );
}

#[tokio::test]
async fn reinitialiser_la_demo_restaure_le_jeu_de_depart() {
    let r = routeur(Mode::Demo);
    ecrire(
        &r,
        Method::POST,
        "/api/fiches",
        json!({ "lundi": "2026-10-05", "type": "entreprise" }),
    )
    .await;
    assert_eq!(
        get(&r, "/api/fiches")
            .await
            .json()
            .as_array()
            .unwrap()
            .len(),
        4
    );
    let reponse = ecrire(&r, Method::POST, "/api/demo/reinitialiser", json!({})).await;
    assert_eq!(reponse.statut, StatusCode::NO_CONTENT);
    assert_eq!(
        get(&r, "/api/fiches")
            .await
            .json()
            .as_array()
            .unwrap()
            .len(),
        3
    );
}

#[tokio::test]
async fn aucune_route_ne_modifie_ni_ne_supprime_le_journal() {
    let r = routeur(Mode::Demo);
    for methode in [Method::PUT, Method::PATCH, Method::DELETE] {
        let reponse = ecrire(&r, methode, "/api/journal", json!({})).await;
        assert_eq!(reponse.statut, StatusCode::METHOD_NOT_ALLOWED);
    }
}

#[tokio::test]
async fn le_journal_est_pagine_et_filtre() {
    let r = routeur(Mode::Demo);
    let page = get(&r, "/api/journal?taille=10").await.json();
    assert_eq!(
        (page["total"].as_u64(), page["pages"].as_u64()),
        (Some(24), Some(3))
    );
    assert_eq!(page["entrees"][0]["type"], "statut_modifie");
    let filtre = get(&r, "/api/journal?type=semaine_justifiee").await.json();
    assert_eq!(filtre["total"], 1);
}

#[tokio::test]
async fn get_export_html_retourne_text_html_avec_styles_en_ligne() {
    let r = routeur(Mode::Demo);
    let fiche = fiche_s38(&r).await;
    let reponse = get(
        &r,
        &format!(
            "/api/fiches/{}/export?format=html",
            fiche["id"].as_str().unwrap()
        ),
    )
    .await;
    assert_eq!(reponse.statut, StatusCode::OK);
    assert!(
        reponse.entetes[header::CONTENT_TYPE]
            .to_str()
            .unwrap()
            .starts_with("text/html")
    );
    let html = reponse.texte();
    assert!(html.contains("style=\""));
    assert!(!html.contains("class="));
    assert!(html.contains("Évaluation par le tuteur"));
    assert!(html.contains("Démonstration"));
}

#[tokio::test]
async fn get_export_docx_retourne_type_et_disposition_attachment() {
    let r = routeur(Mode::Demo);
    let fiche = fiche_s38(&r).await;
    let reponse = get(
        &r,
        &format!(
            "/api/fiches/{}/export?format=docx",
            fiche["id"].as_str().unwrap()
        ),
    )
    .await;
    assert_eq!(reponse.statut, StatusCode::OK);
    assert!(
        reponse.entetes[header::CONTENT_TYPE]
            .to_str()
            .unwrap()
            .contains("wordprocessingml")
    );
    assert!(
        reponse.entetes[header::CONTENT_DISPOSITION]
            .to_str()
            .unwrap()
            .starts_with("attachment")
    );
    let mut archive = zip::ZipArchive::new(std::io::Cursor::new(reponse.corps)).unwrap();
    let mut xml = String::new();
    archive
        .by_name("word/document.xml")
        .unwrap()
        .read_to_string(&mut xml)
        .unwrap();
    assert!(xml.contains("Mise en place du pipeline"));
}

#[tokio::test]
async fn l_export_complet_json_contient_toutes_les_tables_et_schema_version() {
    let r = routeur(Mode::Demo);
    let reponse = get(&r, "/api/export/complet?format=json").await;
    let document = reponse.json();
    assert_eq!(document["schemaVersion"], 1);
    for t in [
        "mission",
        "competence",
        "motif",
        "semaine_plan",
        "fiche",
        "ligne",
        "ligne_competence",
        "justification",
        "evenement",
    ] {
        assert!(document["tables"][t].is_array(), "{t}");
    }
    assert_eq!(document["tables"]["ligne"][0]["dureeMinutes"], 240);
}

#[tokio::test]
async fn l_export_complet_csv_est_une_archive_avec_un_fichier_par_table() {
    let r = routeur(Mode::Demo);
    let reponse = get(&r, "/api/export/complet?format=csv").await;
    let mut archive = zip::ZipArchive::new(std::io::Cursor::new(reponse.corps)).unwrap();
    assert_eq!(archive.len(), 10);
    let mut lisezmoi = String::new();
    archive
        .by_name("LISEZMOI.txt")
        .unwrap()
        .read_to_string(&mut lisezmoi)
        .unwrap();
    assert!(lisezmoi.contains("Version de schéma : 1"));
}

#[tokio::test]
async fn un_tableau_du_bilan_s_exporte_en_csv_utf8() {
    let r = routeur(Mode::Demo);
    let reponse = get(&r, "/api/bilan/temps-par-mission.csv?annee=2026").await;
    assert_eq!(reponse.statut, StatusCode::OK);
    let csv = reponse.texte();
    assert!(csv.starts_with('\u{feff}'));
    assert!(csv.contains("Prise en main de l'environnement Oracle;10"));
}

#[tokio::test]
async fn le_bilan_reprend_les_valeurs_de_la_maquette() {
    let b = get(&routeur(Mode::Demo), "/api/bilan?annee=2026")
        .await
        .json();
    assert_eq!(b["ponctualite"]["taux"], 0.5);
    assert_eq!(b["totalHeures"], 25.0);
    assert_eq!(b["parMission"][0]["valeur"], 10.0);
}

#[tokio::test]
async fn le_lot_d_une_periode_sans_fiche_retourne_404() {
    let r = get(
        &routeur(Mode::Demo),
        "/api/export/lot?periode=mois&valeur=2026-11&format=pdf",
    )
    .await;
    assert_eq!(r.statut, StatusCode::NOT_FOUND);
    let ok = get(
        &routeur(Mode::Demo),
        "/api/export/lot?periode=annee&valeur=2026&format=pdf",
    )
    .await;
    assert_eq!(ok.statut, StatusCode::OK);
    assert!(ok.corps.starts_with(b"%PDF-"));
}

#[tokio::test]
async fn les_alertes_suivent_le_schema_version_1() {
    let a = get(&routeur(Mode::Demo), "/api/alertes").await.json();
    assert_eq!(a["schemaVersion"], 1);
    assert_eq!(a["enRetard"][0]["joursRetard"], 14);
    assert_eq!(a["aFaire"][0]["lundi"], "2026-10-05");
}

#[tokio::test]
async fn importer_un_referentiel_exige_du_csv() {
    let r = routeur(Mode::Demo);
    let apercu = envoyer(
        &r,
        Request::post("/api/referentiels/missions/import?mode=apercu")
            .header("content-type", "text/csv")
            .body(Body::from("nom\nVeille technique\n"))
            .unwrap(),
    )
    .await;
    assert_eq!(apercu.statut, StatusCode::OK);
    assert_eq!(apercu.json()["aCreer"], 1);
    let json = ecrire(
        &r,
        Method::POST,
        "/api/referentiels/missions/import?mode=apercu",
        json!({}),
    )
    .await;
    assert_eq!(json.statut, StatusCode::UNSUPPORTED_MEDIA_TYPE);
}
