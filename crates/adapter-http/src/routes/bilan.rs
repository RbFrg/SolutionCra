//! Bilan (B-01 à B-08), exports de lots et export complet (D-01 à D-05).

use axum::Json;
use axum::extract::{Path, State};
use axum::response::Response;
use serde::{Deserialize, Serialize};
use utoipa::{IntoParams, ToSchema};
use uuid::Uuid;

use application::exports::{Periode, TableauBilan, nom_du_mois};
use domain::AnneeScolaire;

use crate::dto::{FiltreTypeDto, heures};
use crate::etat::Etat;
use crate::extraction::Parametres;
use crate::probleme::Probleme;
use crate::routes::fiches::{FormatFicheDto, reponse_fichier};
use crate::routes::journal::FormatDonneesDto;
use crate::routes::semaines::{RequeteAnnee, annee};

#[derive(Debug, Serialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct TotauxDto {
    pub attendues: u32,
    pub faites: u32,
    pub justifiees: u32,
    pub en_retard: u32,
    pub a_faire: u32,
    pub taux_faites: Option<f64>,
}

#[derive(Debug, Serialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct PonctualiteDto {
    pub ponctuelles: u32,
    /// Semaines échues, attendues, non justifiées.
    pub echues: u32,
    pub taux: Option<f64>,
    /// Jours entre le dimanche et la clôture ; zéro ou moins = à l'heure.
    pub delai_moyen_jours: Option<f64>,
}

#[derive(Debug, Serialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct MoisDto {
    /// `AAAA-MM`
    pub mois: String,
    pub libelle: String,
    pub en_retard: u32,
    pub justifiees: u32,
    pub faites: u32,
}

#[derive(Debug, Serialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct QuantiteDto {
    pub id: Uuid,
    pub nom: String,
    pub valeur: f64,
}

#[derive(Debug, Serialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct BilanDto {
    pub annee: i32,
    pub totaux: TotauxDto,
    pub ponctualite: PonctualiteDto,
    pub par_mois: Vec<MoisDto>,
    /// Nombre de semaines justifiées par motif.
    pub par_motif: Vec<QuantiteDto>,
    /// Heures par mission.
    pub par_mission: Vec<QuantiteDto>,
    /// Heures par compétence : la durée entière d'une ligne compte pour
    /// chacune de ses compétences, la somme peut donc dépasser le total.
    pub par_competence: Vec<QuantiteDto>,
    pub total_heures: f64,
}

#[utoipa::path(
    get,
    path = "/api/bilan",
    tag = "bilan",
    params(RequeteAnnee),
    responses((status = 200, body = BilanDto))
)]
pub async fn bilan(
    State(etat): State<Etat>,
    Parametres(requete): Parametres<RequeteAnnee>,
) -> Result<Json<BilanDto>, Probleme> {
    let a = annee(&etat, requete.annee)?;
    let (b, noms) = etat.app.bilan(a, requete.type_fiche.into()).await?;
    let c = b.compteurs;
    let mut par_mission: Vec<QuantiteDto> = b
        .par_mission
        .iter()
        .map(|(id, m)| QuantiteDto {
            id: id.uuid(),
            nom: noms.nom(*id),
            valeur: heures(*m),
        })
        .collect();
    let mut par_competence: Vec<QuantiteDto> = b
        .par_competence
        .iter()
        .map(|(id, m)| QuantiteDto {
            id: id.uuid(),
            nom: noms.nom(*id),
            valeur: heures(*m),
        })
        .collect();
    // Du plus grand au plus petit, comme les tableaux de la maquette.
    par_mission.sort_by(|x, y| y.valeur.total_cmp(&x.valeur));
    par_competence.sort_by(|x, y| y.valeur.total_cmp(&x.valeur));
    Ok(Json(BilanDto {
        annee: a.valeur(),
        totaux: TotauxDto {
            attendues: c.attendues,
            faites: c.faites,
            justifiees: c.justifiees,
            en_retard: c.en_retard,
            a_faire: c.a_faire,
            taux_faites: c.taux_faites,
        },
        ponctualite: PonctualiteDto {
            ponctuelles: b.ponctuelles,
            echues: b.echues,
            taux: b.taux_ponctualite,
            delai_moyen_jours: b.delai_moyen_jours,
        },
        par_mois: b
            .par_mois
            .iter()
            .map(|m| MoisDto {
                mois: format!("{}-{:02}", m.annee, m.mois),
                libelle: format!("{} {}", nom_du_mois(m.mois), m.annee),
                en_retard: m.en_retard,
                justifiees: m.justifiees,
                faites: m.faites,
            })
            .collect(),
        par_motif: b
            .par_motif
            .iter()
            .map(|(id, n)| QuantiteDto {
                id: id.uuid(),
                nom: noms.nom(*id),
                valeur: f64::from(*n),
            })
            .collect(),
        par_mission,
        par_competence,
        total_heures: heures(b.total_minutes),
    }))
}

#[utoipa::path(
    get,
    path = "/api/bilan/{fichier}",
    tag = "bilan",
    params(
        ("fichier" = String, Path, description = "totaux.csv, retards-par-mois.csv, justifications-par-motif.csv, temps-par-mission.csv ou temps-par-competence.csv"),
        RequeteAnnee
    ),
    responses(
        (status = 200, description = "Tableau en CSV", content_type = "text/csv"),
        (status = 404, body = Probleme, content_type = "application/problem+json")
    )
)]
pub async fn bilan_csv(
    State(etat): State<Etat>,
    Path(fichier): Path<String>,
    Parametres(requete): Parametres<RequeteAnnee>,
) -> Result<Response, Probleme> {
    let tableau = fichier
        .strip_suffix(".csv")
        .and_then(TableauBilan::depuis_code)
        .ok_or_else(|| Probleme::introuvable("Tableau du bilan inconnu."))?;
    let a = annee(&etat, requete.annee)?;
    let f = etat
        .app
        .exporter_bilan(tableau, a, requete.type_fiche.into())
        .await?;
    Ok(reponse_fichier(f, false))
}

#[derive(Debug, Clone, Copy, Deserialize, ToSchema)]
#[serde(rename_all = "snake_case")]
pub enum PeriodeDto {
    Mois,
    Annee,
}

#[derive(Debug, Deserialize, IntoParams)]
#[into_params(parameter_in = Query)]
pub struct RequeteLot {
    #[param(inline)]
    pub periode: PeriodeDto,
    /// `2026-10` pour un mois, `2026` pour une année scolaire.
    pub valeur: String,
    #[serde(rename = "type", default)]
    #[param(rename = "type", inline)]
    pub type_fiche: FiltreTypeDto,
    /// `docx` ou `pdf` (`html` accepté).
    #[param(inline)]
    pub format: FormatFicheDto,
}

fn periode(requete: &RequeteLot) -> Option<Periode> {
    match requete.periode {
        PeriodeDto::Mois => {
            let (a, m) = requete.valeur.split_once('-')?;
            let mois: u32 = m.parse().ok()?;
            (1..=12).contains(&mois).then_some(Periode::Mois {
                annee: a.parse().ok()?,
                mois,
            })
        }
        PeriodeDto::Annee => AnneeScolaire::new(requete.valeur.parse().ok()?)
            .ok()
            .map(Periode::Annee),
    }
}

#[utoipa::path(
    get,
    path = "/api/export/lot",
    tag = "exports",
    params(RequeteLot),
    responses(
        (status = 200, description = "Une fiche par page (PDF) ou par section (Word)", content_type = "application/octet-stream"),
        (status = 404, description = "Aucune fiche pour cette période", body = Probleme, content_type = "application/problem+json")
    )
)]
pub async fn lot(
    State(etat): State<Etat>,
    Parametres(requete): Parametres<RequeteLot>,
) -> Result<Response, Probleme> {
    let p = periode(&requete)
        .ok_or_else(|| Probleme::requete_invalide("Période invalide : AAAA-MM ou AAAA."))?;
    let f = etat
        .app
        .exporter_lot(p, requete.type_fiche.into(), requete.format.into())
        .await?;
    Ok(reponse_fichier(f, false))
}

#[derive(Debug, Deserialize, IntoParams)]
#[into_params(parameter_in = Query)]
pub struct RequeteComplet {
    #[param(inline)]
    pub format: FormatDonneesDto,
}

#[utoipa::path(
    get,
    path = "/api/export/complet",
    tag = "exports",
    params(RequeteComplet),
    responses((status = 200, description = "JSON unique ou archive zip de CSV", content_type = "application/octet-stream"))
)]
pub async fn complet(
    State(etat): State<Etat>,
    Parametres(requete): Parametres<RequeteComplet>,
) -> Result<Response, Probleme> {
    let f = etat.app.exporter_complet(requete.format.into()).await?;
    Ok(reponse_fichier(f, false))
}
