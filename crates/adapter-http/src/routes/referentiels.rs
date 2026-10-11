//! Référentiels : missions, compétences, motifs (G-01 à G-08).

use axum::Json;
use axum::extract::{Path, State};
use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use serde::{Deserialize, Deserializer, Serialize};
use utoipa::{IntoParams, ToSchema};
use uuid::Uuid;

use application::referentiels::{ActionImport, ApercuImport, ModificationEntree};
use domain::{EntreeId, EntreeReferentiel, NomReferentiel};

use crate::etat::Etat;
use crate::extraction::{Corps, Parametres};
use crate::probleme::Probleme;

#[derive(Debug, Clone, Copy, Deserialize, Serialize, ToSchema)]
#[serde(rename_all = "snake_case")]
pub enum NomReferentielDto {
    Missions,
    Competences,
    Motifs,
}

impl From<NomReferentielDto> for NomReferentiel {
    fn from(n: NomReferentielDto) -> Self {
        match n {
            NomReferentielDto::Missions => Self::Missions,
            NomReferentielDto::Competences => Self::Competences,
            NomReferentielDto::Motifs => Self::Motifs,
        }
    }
}

#[derive(Debug, Serialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct EntreeDto {
    pub id: Uuid,
    pub nom: String,
    pub code: Option<String>,
    pub archivee: bool,
}

impl From<EntreeReferentiel> for EntreeDto {
    fn from(e: EntreeReferentiel) -> Self {
        Self {
            id: e.id.uuid(),
            nom: e.nom,
            code: e.code,
            archivee: e.archivee,
        }
    }
}

#[derive(Debug, Serialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct EntreeEnregistreeDto {
    pub entree: EntreeDto,
    /// Avertissement non bloquant (compétence sans verbe à l'infinitif).
    pub avertissement: Option<String>,
}

#[derive(Debug, Deserialize, IntoParams)]
#[into_params(parameter_in = Query)]
pub struct RequeteReferentiel {
    /// Inclure les entrées archivées.
    #[serde(default)]
    pub archivees: bool,
}

#[utoipa::path(
    get,
    path = "/api/referentiels/{nom}",
    operation_id = "lister_referentiel",
    tag = "referentiels",
    params(("nom" = NomReferentielDto, Path), RequeteReferentiel),
    responses((status = 200, body = Vec<EntreeDto>))
)]
pub async fn lister(
    State(etat): State<Etat>,
    Path(nom): Path<NomReferentielDto>,
    Parametres(requete): Parametres<RequeteReferentiel>,
) -> Result<Json<Vec<EntreeDto>>, Probleme> {
    let entrees = etat.app.referentiel(nom.into(), requete.archivees).await?;
    Ok(Json(entrees.into_iter().map(EntreeDto::from).collect()))
}

#[derive(Debug, Deserialize, ToSchema)]
pub struct AjouterEntreeDto {
    pub nom: String,
    pub code: Option<String>,
}

#[utoipa::path(
    post,
    path = "/api/referentiels/{nom}",
    tag = "referentiels",
    params(("nom" = NomReferentielDto, Path)),
    request_body = AjouterEntreeDto,
    responses(
        (status = 201, body = EntreeEnregistreeDto),
        (status = 422, body = Probleme, content_type = "application/problem+json")
    )
)]
pub async fn ajouter(
    State(etat): State<Etat>,
    Path(nom): Path<NomReferentielDto>,
    Corps(corps): Corps<AjouterEntreeDto>,
) -> Result<Response, Probleme> {
    let (entree, avertissement) = etat
        .app
        .ajouter_entree(nom.into(), &corps.nom, corps.code.as_deref())
        .await?;
    Ok((
        StatusCode::CREATED,
        Json(EntreeEnregistreeDto {
            entree: entree.into(),
            avertissement,
        }),
    )
        .into_response())
}

/// Distingue « absent » (inchangé) de `null` (effacer).
fn present<'de, D: Deserializer<'de>>(d: D) -> Result<Option<Option<String>>, D::Error> {
    Option::<String>::deserialize(d).map(Some)
}

#[derive(Debug, Deserialize, ToSchema)]
pub struct ModifierEntreeDto {
    pub nom: Option<String>,
    /// `null` efface le code ; absent, il est conservé.
    #[serde(default, deserialize_with = "present")]
    #[schema(value_type = Option<String>)]
    pub code: Option<Option<String>>,
    pub archivee: Option<bool>,
}

#[utoipa::path(
    patch,
    path = "/api/referentiels/{nom}/{id}",
    tag = "referentiels",
    params(("nom" = NomReferentielDto, Path), ("id" = Uuid, Path)),
    request_body = ModifierEntreeDto,
    responses(
        (status = 200, body = EntreeEnregistreeDto),
        (status = 422, body = Probleme, content_type = "application/problem+json")
    )
)]
pub async fn modifier(
    State(etat): State<Etat>,
    Path((nom, id)): Path<(NomReferentielDto, Uuid)>,
    Corps(corps): Corps<ModifierEntreeDto>,
) -> Result<Json<EntreeEnregistreeDto>, Probleme> {
    let (entree, avertissement) = etat
        .app
        .modifier_entree(
            nom.into(),
            EntreeId(id),
            ModificationEntree {
                nom: corps.nom,
                code: corps.code,
                archivee: corps.archivee,
            },
        )
        .await?;
    Ok(Json(EntreeEnregistreeDto {
        entree: entree.into(),
        avertissement,
    }))
}

#[derive(Debug, Clone, Copy, Deserialize, ToSchema)]
#[serde(rename_all = "snake_case")]
pub enum ModeImport {
    Apercu,
    Valider,
}

#[derive(Debug, Deserialize, IntoParams)]
#[into_params(parameter_in = Query)]
pub struct RequeteImport {
    #[param(inline)]
    pub mode: ModeImport,
}

#[derive(Debug, Serialize, ToSchema)]
#[serde(rename_all = "snake_case")]
pub enum ActionImportDto {
    Creer,
    Ignorer,
}

#[derive(Debug, Serialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct LigneImportDto {
    pub ligne: usize,
    pub nom: String,
    pub code: Option<String>,
    pub archivee: bool,
    pub action: ActionImportDto,
}

#[derive(Debug, Serialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct MessageLigneDto {
    pub ligne: usize,
    pub message: String,
}

#[derive(Debug, Serialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct ApercuImportDto {
    pub lignes: Vec<LigneImportDto>,
    pub erreurs: Vec<MessageLigneDto>,
    pub avertissements: Vec<MessageLigneDto>,
    pub a_creer: usize,
    pub ecrit: bool,
}

fn apercu_dto(a: ApercuImport, ecrit: bool) -> ApercuImportDto {
    ApercuImportDto {
        a_creer: a.a_creer(),
        lignes: a
            .lignes
            .into_iter()
            .map(|l| LigneImportDto {
                ligne: l.ligne,
                nom: l.nom,
                code: l.code,
                archivee: l.archivee,
                action: match l.action {
                    ActionImport::Creer => ActionImportDto::Creer,
                    ActionImport::Ignorer => ActionImportDto::Ignorer,
                },
            })
            .collect(),
        erreurs: a
            .erreurs
            .into_iter()
            .map(|m| MessageLigneDto {
                ligne: m.ligne,
                message: m.message,
            })
            .collect(),
        avertissements: a
            .avertissements
            .into_iter()
            .map(|m| MessageLigneDto {
                ligne: m.ligne,
                message: m.message,
            })
            .collect(),
        ecrit,
    }
}

#[utoipa::path(
    post,
    path = "/api/referentiels/{nom}/import",
    tag = "referentiels",
    params(("nom" = NomReferentielDto, Path), RequeteImport),
    request_body(content = String, content_type = "text/csv"),
    responses(
        (status = 200, body = ApercuImportDto),
        (status = 422, body = Probleme, content_type = "application/problem+json")
    )
)]
pub async fn importer(
    State(etat): State<Etat>,
    Path(nom): Path<NomReferentielDto>,
    Parametres(requete): Parametres<RequeteImport>,
    corps: String,
) -> Result<Json<ApercuImportDto>, Probleme> {
    let valider = matches!(requete.mode, ModeImport::Valider);
    let apercu = etat
        .app
        .importer_referentiel(nom.into(), &corps, valider)
        .await?;
    Ok(Json(apercu_dto(apercu, valider)))
}
