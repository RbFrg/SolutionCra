//! Protection CSRF des écritures (17.2), règles 1 à 4. Le jeton de session
//! (règle 6) arrive avec l'accès sécurisé (tranche 5).

use axum::extract::{Request, State};
use axum::http::{Method, StatusCode, header};
use axum::middleware::Next;
use axum::response::{IntoResponse, Response};

use crate::etat::Etat;
use crate::probleme::Probleme;

fn est_une_ecriture(methode: &Method) -> bool {
    matches!(
        *methode,
        Method::POST | Method::PUT | Method::PATCH | Method::DELETE
    )
}

/// Routes qui reçoivent un fichier CSV brut (19) au lieu de JSON.
fn attend_du_csv(chemin: &str) -> bool {
    chemin.ends_with("/import")
}

fn refus(code: &str, detail: &str) -> Response {
    Probleme::new(StatusCode::FORBIDDEN, code, "Requête refusée")
        .detail(detail)
        .into_response()
}

pub async fn proteger(State(etat): State<Etat>, requete: Request, suivant: Next) -> Response {
    if !est_une_ecriture(requete.method()) {
        return suivant.run(requete).await;
    }
    let entetes = requete.headers();

    // Règle 1 : une requête venue d'un autre site est refusée.
    if let Some(site) = entetes.get("sec-fetch-site").and_then(|v| v.to_str().ok())
        && site != "same-origin"
        && site != "none"
    {
        return refus("csrf", "Requête venue d'un autre site.");
    }

    // Règle 2 : une origine présente doit être celle du site lui-même (même
    // hôte que l'en-tête Host) ou figurer dans CRA_ALLOWED_ORIGINS.
    if let Some(origine) = entetes.get(header::ORIGIN).and_then(|v| v.to_str().ok()) {
        let origine = origine.trim_end_matches('/');
        let hote = entetes.get(header::HOST).and_then(|v| v.to_str().ok());
        let meme_origine =
            hote.is_some_and(|h| origine.split_once("://").map(|(_, a)| a) == Some(h));
        if !meme_origine && !etat.origines_autorisees.iter().any(|o| o == origine) {
            return refus("csrf", "Origine non autorisée.");
        }
    }

    // Règle 3 : JSON seulement (CSV pour les imports), sauf corps vide.
    let a_un_corps = entetes
        .get(header::CONTENT_LENGTH)
        .and_then(|v| v.to_str().ok())
        .is_none_or(|l| l != "0");
    let type_contenu = entetes
        .get(header::CONTENT_TYPE)
        .and_then(|v| v.to_str().ok())
        .map(|v| {
            v.split(';')
                .next()
                .unwrap_or_default()
                .trim()
                .to_ascii_lowercase()
        });
    let attendu = if attend_du_csv(requete.uri().path()) {
        "text/csv"
    } else {
        "application/json"
    };
    let accepte = match type_contenu.as_deref() {
        Some(t) => t == attendu,
        None => !a_un_corps || *requete.method() == Method::DELETE,
    };
    if !accepte {
        return Probleme::new(
            StatusCode::UNSUPPORTED_MEDIA_TYPE,
            "type-de-contenu",
            "Type de contenu non pris en charge",
        )
        .detail(format!("Le corps doit être en {attendu}."))
        .into_response();
    }

    suivant.run(requete).await
}
