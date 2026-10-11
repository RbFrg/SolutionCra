#![allow(dead_code, clippy::unwrap_used, clippy::expect_used)]

use std::sync::Arc;

use adapter_memory::{DepotMemoire, EtatMemoire, jeu_demo};
use application::exports::{DocumentFiche, Exporteur, FormatDocument, Table};
use application::mode::instant_demo;
use application::{App, FixedClock, Mode, SequenceIds};
use domain::{AnneeScolaire, EntreeId, FiltreType, Lundi, NomReferentiel};

/// Exporteur factice : les cas d'usage n'ont pas à connaître les formats.
pub struct FauxExporteur;

impl Exporteur for FauxExporteur {
    fn fiche(&self, document: &DocumentFiche, format: FormatDocument) -> Result<Vec<u8>, String> {
        Ok(format!(
            "{:?} {} {}",
            format,
            document.titre(),
            document.lignes.len()
        )
        .into_bytes())
    }
    fn lot(
        &self,
        titre: &str,
        documents: &[DocumentFiche],
        _: FormatDocument,
    ) -> Result<Vec<u8>, String> {
        Ok(format!("{titre} {}", documents.len()).into_bytes())
    }
    fn csv(&self, table: &Table) -> Vec<u8> {
        format!("{} {}", table.nom, table.lignes.len()).into_bytes()
    }
    fn archive_csv(&self, tables: &[Table], _: &str) -> Result<Vec<u8>, String> {
        Ok(tables
            .iter()
            .map(|t| t.nom.clone())
            .collect::<Vec<_>>()
            .join(",")
            .into_bytes())
    }
}

pub struct Contexte {
    pub app: App,
    pub depot: DepotMemoire,
    pub horloge: Arc<FixedClock>,
}

pub fn contexte_avec(etat: EtatMemoire, mode: Mode) -> Contexte {
    let depot = DepotMemoire::new(etat);
    let horloge = Arc::new(FixedClock::new(instant_demo()));
    let app = App::new(
        Arc::new(depot.clone()),
        horloge.clone(),
        Arc::new(SequenceIds::new(1_800_000_000_000)),
        Arc::new(FauxExporteur),
        mode,
    )
    .avec_reinitialisation(Arc::new(depot.clone()));
    Contexte {
        app,
        depot,
        horloge,
    }
}

/// L'application en mode démo, au 11/10/2026.
pub fn demo() -> Contexte {
    contexte_avec(jeu_demo(), Mode::Demo)
}

pub fn lundi(a: i32, m: u32, j: u32) -> Lundi {
    Lundi::new(chrono::NaiveDate::from_ymd_opt(a, m, j).unwrap()).unwrap()
}

pub fn a2026() -> AnneeScolaire {
    AnneeScolaire::new(2026).unwrap()
}

pub const TOUTES: FiltreType = FiltreType::Toutes;

pub async fn entree(app: &App, referentiel: NomReferentiel, nom: &str) -> EntreeId {
    app.referentiel(referentiel, true)
        .await
        .unwrap()
        .into_iter()
        .find(|e| e.nom == nom)
        .unwrap()
        .id
}
