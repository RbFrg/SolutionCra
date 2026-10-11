//! Extracteurs dont les refus sont des `problem+json` (4.1), et traduction
//! des erreurs des cas d'usage.

use axum::extract::rejection::{JsonRejection, QueryRejection};
use axum::extract::{FromRequest, FromRequestParts, Query, Request};
use axum::http::StatusCode;
use axum::http::request::Parts;
use serde::de::DeserializeOwned;

use application::ErreurApp;

use crate::probleme::{ErreurChampDto, Probleme};

/// Corps JSON ; un refus de désérialisation devient un 422 `validation`.
pub struct Corps<T>(pub T);

impl<S, T> FromRequest<S> for Corps<T>
where
    T: DeserializeOwned,
    S: Send + Sync,
{
    type Rejection = Probleme;

    async fn from_request(req: Request, etat: &S) -> Result<Self, Self::Rejection> {
        match axum::Json::<T>::from_request(req, etat).await {
            Ok(axum::Json(valeur)) => Ok(Self(valeur)),
            Err(JsonRejection::MissingJsonContentType(_)) => Err(Probleme::new(
                StatusCode::UNSUPPORTED_MEDIA_TYPE,
                "type-de-contenu",
                "Type de contenu non pris en charge",
            )
            .detail("Le corps doit être en application/json.")),
            Err(erreur) => Err(Probleme::new(
                StatusCode::UNPROCESSABLE_ENTITY,
                "validation",
                "Corps de requête invalide",
            )
            .detail(erreur.body_text())),
        }
    }
}

/// Paramètres de requête ; un refus devient un 400.
pub struct Parametres<T>(pub T);

impl<S, T> FromRequestParts<S> for Parametres<T>
where
    T: DeserializeOwned,
    S: Send + Sync,
{
    type Rejection = Probleme;

    async fn from_request_parts(parts: &mut Parts, etat: &S) -> Result<Self, Self::Rejection> {
        Query::<T>::from_request_parts(parts, etat)
            .await
            .map(|Query(v)| Self(v))
            .map_err(|e: QueryRejection| Probleme::requete_invalide(e.body_text()))
    }
}

impl From<ErreurApp> for Probleme {
    fn from(erreur: ErreurApp) -> Self {
        match erreur {
            ErreurApp::Validation(v) => {
                let mut p = Probleme::new(
                    StatusCode::UNPROCESSABLE_ENTITY,
                    "validation",
                    "Données invalides",
                )
                .detail(if v.erreurs.len() == 1 {
                    "1 champ à corriger.".to_owned()
                } else {
                    format!("{} champs à corriger.", v.erreurs.len())
                });
                p.errors = v
                    .erreurs
                    .into_iter()
                    .map(|e| ErreurChampDto {
                        field: e.champ,
                        message: e.message,
                    })
                    .collect();
                p
            }
            ErreurApp::Regle { code, message } => {
                Probleme::new(StatusCode::UNPROCESSABLE_ENTITY, code, "Action refusée")
                    .detail(message)
            }
            ErreurApp::Introuvable(message) => Probleme::introuvable(message),
            ErreurApp::Conflit { version_courante } => {
                let mut p = Probleme::new(
                    StatusCode::CONFLICT,
                    "conflit-de-version",
                    "Conflit de version",
                )
                .detail("La fiche a été modifiée ailleurs. Rechargez-la avant de continuer.");
                p.version_courante = Some(version_courante);
                p
            }
            ErreurApp::FicheExistante { fiche_id, message } => {
                let mut p =
                    Probleme::new(StatusCode::CONFLICT, "fiche-existante", "Fiche existante")
                        .detail(message);
                p.fiche_id = Some(fiche_id.uuid());
                p
            }
            ErreurApp::ConfirmationRequise { semaines } => {
                let mut p = Probleme::new(
                    StatusCode::CONFLICT,
                    "confirmation-requise",
                    "Confirmation requise",
                )
                .detail(
                    "Des semaines déjà commencées changent de type : confirmez la modification.",
                );
                p.semaines_passees_touchees =
                    Some(semaines.into_iter().map(|l| l.date()).collect());
                p
            }
            ErreurApp::Technique(message) => {
                tracing::error!(%message, "erreur technique");
                Probleme::interne()
            }
        }
    }
}
