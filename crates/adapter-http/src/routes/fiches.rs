//! Fiches : liste, création, lecture, sauvegarde, statut, suppression, export.

use axum::Json;
use axum::extract::{Path, State};
use axum::http::{HeaderMap, HeaderValue, StatusCode, header};
use axum::response::{IntoResponse, Response};
use chrono::{DateTime, NaiveDate, Utc};
use serde::{Deserialize, Serialize};
use utoipa::{IntoParams, ToSchema};
use uuid::Uuid;

use application::exports::{Fichier, FormatDocument};
use application::fiches::FiltreFiches;
use application::{FicheStockee, Noms};
use domain::colonnes::colonnes;
use domain::{EntreeId, FicheId, LigneId, LigneSaisie};

use crate::dto::{FiltreTypeDto, StatutFicheDto, TypeFicheDto, heures};
use crate::etat::Etat;
use crate::extraction::{Corps, Parametres};
use crate::probleme::Probleme;
use crate::routes::semaines::lundi;

#[derive(Debug, Deserialize, IntoParams)]
#[into_params(parameter_in = Query)]
pub struct RequeteColonnes {
    #[serde(rename = "type")]
    #[param(rename = "type", inline)]
    pub type_fiche: TypeFicheDto,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct ColonnesDto {
    #[serde(rename = "type")]
    pub type_fiche: TypeFicheDto,
    /// Les six libellés, dans l'ordre (18.4).
    pub colonnes: Vec<String>,
}

#[utoipa::path(
    get,
    path = "/api/meta/colonnes",
    tag = "fiches",
    params(RequeteColonnes),
    responses((status = 200, body = ColonnesDto))
)]
pub async fn meta_colonnes(Parametres(requete): Parametres<RequeteColonnes>) -> Json<ColonnesDto> {
    Json(ColonnesDto {
        type_fiche: requete.type_fiche,
        colonnes: colonnes(requete.type_fiche.into())
            .iter()
            .map(|c| (*c).to_owned())
            .collect(),
    })
}

#[derive(Debug, Serialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct EntreeNommeeDto {
    pub id: Uuid,
    pub nom: String,
    pub archivee: bool,
}

#[derive(Debug, Serialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct LigneDto {
    pub id: Uuid,
    pub date: NaiveDate,
    pub mission_id: Uuid,
    /// Nom en lecture seule, ignoré à l'écriture.
    pub mission: EntreeNommeeDto,
    pub competence_ids: Vec<Uuid>,
    pub competences: Vec<EntreeNommeeDto>,
    pub duree_heures: f64,
    pub etapes: String,
    pub echanges_tuteur: Option<String>,
}

#[derive(Debug, Serialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct FicheDto {
    pub id: Uuid,
    pub lundi: NaiveDate,
    pub dimanche: NaiveDate,
    pub numero: u32,
    #[serde(rename = "type")]
    pub type_fiche: TypeFicheDto,
    pub statut: StatutFicheDto,
    pub terminee_le: Option<DateTime<Utc>>,
    pub transmise_le: Option<NaiveDate>,
    pub validee_le: Option<NaiveDate>,
    pub modifiee_apres_transmission: bool,
    pub modifiee_apres_transmission_le: Option<DateTime<Utc>>,
    /// Concurrence optimiste : à renvoyer à chaque écriture (4.1).
    pub version: u32,
    pub total_heures: f64,
    /// Les six libellés de colonnes du type (18.4).
    pub colonnes: Vec<String>,
    pub lignes: Vec<LigneDto>,
    pub creee_le: DateTime<Utc>,
    pub modifiee_le: DateTime<Utc>,
}

fn nommee(noms: &Noms, id: EntreeId) -> EntreeNommeeDto {
    let entree = noms.entree(id);
    EntreeNommeeDto {
        id: id.uuid(),
        nom: noms.nom(id),
        archivee: entree.is_some_and(|e| e.archivee),
    }
}

pub(crate) fn fiche_dto(s: &FicheStockee, noms: &Noms) -> FicheDto {
    let f = &s.fiche;
    FicheDto {
        id: f.id.uuid(),
        lundi: f.lundi.date(),
        dimanche: f.lundi.dimanche(),
        numero: f.lundi.numero(),
        type_fiche: f.type_fiche.into(),
        statut: f.statut.into(),
        terminee_le: f.terminee_le,
        transmise_le: f.transmise_le,
        validee_le: f.validee_le,
        modifiee_apres_transmission: f.modifiee_apres_transmission(),
        modifiee_apres_transmission_le: f.modifiee_apres_transmission_le,
        version: s.version,
        total_heures: heures(f.total_minutes()),
        colonnes: colonnes(f.type_fiche)
            .iter()
            .map(|c| (*c).to_owned())
            .collect(),
        lignes: f
            .lignes
            .iter()
            .map(|l| LigneDto {
                id: l.id.uuid(),
                date: l.date,
                mission_id: l.mission.uuid(),
                mission: nommee(noms, l.mission),
                competence_ids: l.competences.iter().map(|c| c.uuid()).collect(),
                competences: l.competences.iter().map(|c| nommee(noms, *c)).collect(),
                duree_heures: l.duree.en_heures(),
                etapes: l.etapes.clone(),
                echanges_tuteur: l.echanges_tuteur.clone(),
            })
            .collect(),
        creee_le: s.creee_le,
        modifiee_le: s.modifiee_le,
    }
}

#[derive(Debug, Serialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct FicheResumeDto {
    pub id: Uuid,
    pub lundi: NaiveDate,
    pub dimanche: NaiveDate,
    pub numero: u32,
    #[serde(rename = "type")]
    pub type_fiche: TypeFicheDto,
    pub statut: StatutFicheDto,
    pub version: u32,
    pub total_heures: f64,
    pub nb_lignes: usize,
    /// Mission de la première ligne, pour la liste.
    pub mission_principale: Option<String>,
    pub transmise_le: Option<NaiveDate>,
    pub modifiee_apres_transmission: bool,
    pub modifiee_le: DateTime<Utc>,
}

#[derive(Debug, Deserialize, IntoParams)]
#[into_params(parameter_in = Query)]
pub struct RequeteFiches {
    pub annee: Option<i32>,
    #[serde(rename = "type", default)]
    #[param(rename = "type", inline)]
    pub type_fiche: FiltreTypeDto,
    #[param(inline)]
    pub statut: Option<StatutFicheDto>,
}

#[utoipa::path(
    get,
    path = "/api/fiches",
    operation_id = "lister_fiches",
    tag = "fiches",
    params(RequeteFiches),
    responses((status = 200, body = Vec<FicheResumeDto>))
)]
pub async fn lister(
    State(etat): State<Etat>,
    Parametres(requete): Parametres<RequeteFiches>,
) -> Result<Json<Vec<FicheResumeDto>>, Probleme> {
    let annee = match requete.annee {
        Some(a) => Some(crate::routes::semaines::annee(&etat, Some(a))?),
        None => None,
    };
    let fiches = etat
        .app
        .lister_fiches(&FiltreFiches {
            annee,
            type_fiche: requete.type_fiche.into(),
            statut: requete.statut.map(Into::into),
        })
        .await?;
    let noms = etat.app.noms().await?;
    Ok(Json(
        fiches
            .iter()
            .map(|s| FicheResumeDto {
                id: s.fiche.id.uuid(),
                lundi: s.fiche.lundi.date(),
                dimanche: s.fiche.lundi.dimanche(),
                numero: s.fiche.lundi.numero(),
                type_fiche: s.fiche.type_fiche.into(),
                statut: s.fiche.statut.into(),
                version: s.version,
                total_heures: heures(s.fiche.total_minutes()),
                nb_lignes: s.fiche.lignes.len(),
                mission_principale: s.fiche.lignes.first().map(|l| noms.nom(l.mission)),
                transmise_le: s.fiche.transmise_le,
                modifiee_apres_transmission: s.fiche.modifiee_apres_transmission(),
                modifiee_le: s.modifiee_le,
            })
            .collect(),
    ))
}

#[derive(Debug, Deserialize, ToSchema)]
pub struct CreerFicheDto {
    pub lundi: NaiveDate,
    #[serde(rename = "type")]
    pub type_fiche: TypeFicheDto,
}

#[utoipa::path(
    post,
    path = "/api/fiches",
    tag = "fiches",
    request_body = CreerFicheDto,
    responses(
        (status = 201, body = FicheDto, headers(("Location" = String, description = "/api/fiches/{id}"))),
        (status = 409, description = "fiche-existante", body = Probleme, content_type = "application/problem+json"),
        (status = 422, body = Probleme, content_type = "application/problem+json")
    )
)]
pub async fn creer(
    State(etat): State<Etat>,
    Corps(corps): Corps<CreerFicheDto>,
) -> Result<Response, Probleme> {
    let l = lundi(corps.lundi, "lundi")?;
    let fiche = etat.app.creer_fiche(l, corps.type_fiche.into()).await?;
    let noms = etat.app.noms().await?;
    let mut entetes = HeaderMap::new();
    if let Ok(v) = HeaderValue::from_str(&format!("/api/fiches/{}", fiche.fiche.id)) {
        entetes.insert(header::LOCATION, v);
    }
    Ok((StatusCode::CREATED, entetes, Json(fiche_dto(&fiche, &noms))).into_response())
}

#[utoipa::path(
    get,
    path = "/api/fiches/{id}",
    operation_id = "lire_fiche",
    tag = "fiches",
    params(("id" = Uuid, Path)),
    responses(
        (status = 200, body = FicheDto),
        (status = 404, body = Probleme, content_type = "application/problem+json")
    )
)]
pub async fn lire(
    State(etat): State<Etat>,
    Path(id): Path<Uuid>,
) -> Result<Json<FicheDto>, Probleme> {
    let fiche = etat.app.lire_fiche(FicheId(id)).await?;
    Ok(Json(fiche_dto(&fiche, &etat.app.noms().await?)))
}

#[derive(Debug, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct LigneSaisieDto {
    /// Nul pour une nouvelle ligne.
    pub id: Option<Uuid>,
    pub date: NaiveDate,
    pub mission_id: Uuid,
    #[serde(default)]
    pub competence_ids: Vec<Uuid>,
    /// Heures décimales au quart d'heure (0,25 à 12).
    pub duree_heures: f64,
    #[serde(default)]
    pub etapes: String,
    pub echanges_tuteur: Option<String>,
}

#[derive(Debug, Deserialize, ToSchema)]
pub struct SauvegarderFicheDto {
    pub version: u32,
    pub lignes: Vec<LigneSaisieDto>,
}

#[utoipa::path(
    put,
    path = "/api/fiches/{id}",
    tag = "fiches",
    params(("id" = Uuid, Path)),
    request_body = SauvegarderFicheDto,
    responses(
        (status = 200, body = FicheDto),
        (status = 409, description = "conflit-de-version", body = Probleme, content_type = "application/problem+json"),
        (status = 422, body = Probleme, content_type = "application/problem+json")
    )
)]
pub async fn sauvegarder(
    State(etat): State<Etat>,
    Path(id): Path<Uuid>,
    Corps(corps): Corps<SauvegarderFicheDto>,
) -> Result<Json<FicheDto>, Probleme> {
    let saisies: Vec<LigneSaisie> = corps
        .lignes
        .into_iter()
        .map(|l| LigneSaisie {
            id: l.id.map(LigneId),
            date: l.date,
            mission: EntreeId(l.mission_id),
            competences: l.competence_ids.into_iter().map(EntreeId).collect(),
            duree_heures: l.duree_heures,
            etapes: l.etapes,
            echanges_tuteur: l.echanges_tuteur,
        })
        .collect();
    let fiche = etat
        .app
        .sauvegarder_fiche(FicheId(id), corps.version, &saisies)
        .await?;
    Ok(Json(fiche_dto(&fiche, &etat.app.noms().await?)))
}

#[derive(Debug, Deserialize, ToSchema)]
pub struct ChangerStatutDto {
    pub statut: StatutFicheDto,
    /// Date de Paris, lue pour `transmise` et `validee` ; aujourd'hui par défaut.
    pub date: Option<NaiveDate>,
    pub version: u32,
}

#[utoipa::path(
    put,
    path = "/api/fiches/{id}/statut",
    tag = "fiches",
    params(("id" = Uuid, Path)),
    request_body = ChangerStatutDto,
    responses(
        (status = 200, body = FicheDto),
        (status = 409, body = Probleme, content_type = "application/problem+json"),
        (status = 422, description = "terminee-sans-ligne", body = Probleme, content_type = "application/problem+json")
    )
)]
pub async fn changer_statut(
    State(etat): State<Etat>,
    Path(id): Path<Uuid>,
    Corps(corps): Corps<ChangerStatutDto>,
) -> Result<Json<FicheDto>, Probleme> {
    let fiche = etat
        .app
        .changer_statut(FicheId(id), corps.version, corps.statut.into(), corps.date)
        .await?;
    Ok(Json(fiche_dto(&fiche, &etat.app.noms().await?)))
}

#[utoipa::path(
    delete,
    path = "/api/fiches/{id}",
    tag = "fiches",
    params(("id" = Uuid, Path)),
    responses(
        (status = 204, description = "Fiche supprimée ; le journal la garde"),
        (status = 404, body = Probleme, content_type = "application/problem+json")
    )
)]
pub async fn supprimer(
    State(etat): State<Etat>,
    Path(id): Path<Uuid>,
) -> Result<StatusCode, Probleme> {
    etat.app.supprimer_fiche(FicheId(id)).await?;
    Ok(StatusCode::NO_CONTENT)
}

#[derive(Debug, Clone, Copy, Deserialize, ToSchema)]
#[serde(rename_all = "snake_case")]
pub enum FormatFicheDto {
    Html,
    Docx,
    Pdf,
}

impl From<FormatFicheDto> for FormatDocument {
    fn from(f: FormatFicheDto) -> Self {
        match f {
            FormatFicheDto::Html => Self::Html,
            FormatFicheDto::Docx => Self::Docx,
            FormatFicheDto::Pdf => Self::Pdf,
        }
    }
}

#[derive(Debug, Deserialize, IntoParams)]
#[into_params(parameter_in = Query)]
pub struct RequeteExport {
    #[param(inline)]
    pub format: FormatFicheDto,
}

/// Réponse fichier : `attachment` sauf pour le HTML (copie pour OneNote).
pub(crate) fn reponse_fichier(fichier: Fichier, en_ligne: bool) -> Response {
    let mut reponse = fichier.contenu.into_response();
    let entetes = reponse.headers_mut();
    if let Ok(v) = HeaderValue::from_str(&fichier.type_mime) {
        entetes.insert(header::CONTENT_TYPE, v);
    }
    let disposition = format!(
        "{}; filename=\"{}\"",
        if en_ligne { "inline" } else { "attachment" },
        fichier.nom.replace('"', "")
    );
    if let Ok(v) = HeaderValue::from_str(&disposition) {
        entetes.insert(header::CONTENT_DISPOSITION, v);
    }
    entetes.insert(header::CACHE_CONTROL, HeaderValue::from_static("no-store"));
    reponse
}

#[utoipa::path(
    get,
    path = "/api/fiches/{id}/export",
    operation_id = "exporter_fiche",
    tag = "exports",
    params(("id" = Uuid, Path), RequeteExport),
    responses(
        (status = 200, description = "Fichier HTML (styles en ligne), Word ou PDF", content_type = "application/octet-stream"),
        (status = 404, body = Probleme, content_type = "application/problem+json")
    )
)]
pub async fn exporter(
    State(etat): State<Etat>,
    Path(id): Path<Uuid>,
    Parametres(requete): Parametres<RequeteExport>,
) -> Result<Response, Probleme> {
    let format: FormatDocument = requete.format.into();
    let fichier = etat.app.exporter_fiche(FicheId(id), format).await?;
    Ok(reponse_fichier(fichier, format == FormatDocument::Html))
}
