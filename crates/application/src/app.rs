//! Point d'entrée des cas d'usage : les ports injectés et l'instant courant.

use std::collections::HashMap;
use std::sync::Arc;

use chrono::{DateTime, Utc};
use serde_json::Value;

use domain::journal::{Evenement, TypeEvenement};
use domain::{EntreeId, EntreeReferentiel, EvenementId, Lundi, NomReferentiel};

use crate::erreur::Resultat;
use crate::exports::Exporteur;
use crate::horloge::Clock;
use crate::ids::IdGenerator;
use crate::mode::Mode;
use crate::ports::{Depot, Reinitialisable};

pub struct App {
    pub(crate) depot: Arc<dyn Depot>,
    pub(crate) horloge: Arc<dyn Clock>,
    pub(crate) ids: Arc<dyn IdGenerator>,
    pub(crate) exporteur: Arc<dyn Exporteur>,
    pub(crate) mode: Mode,
    pub(crate) reinitialisable: Option<Arc<dyn Reinitialisable>>,
}

impl App {
    pub fn new(
        depot: Arc<dyn Depot>,
        horloge: Arc<dyn Clock>,
        ids: Arc<dyn IdGenerator>,
        exporteur: Arc<dyn Exporteur>,
        mode: Mode,
    ) -> Self {
        Self {
            depot,
            horloge,
            ids,
            exporteur,
            mode,
            reinitialisable: None,
        }
    }

    /// Démo seulement : la réinitialisation du jeu de départ (M-05).
    pub fn avec_reinitialisation(mut self, cible: Arc<dyn Reinitialisable>) -> Self {
        self.reinitialisable = Some(cible);
        self
    }

    pub fn mode(&self) -> Mode {
        self.mode
    }

    /// Le serveur est la seule source de « maintenant » (4.1).
    pub fn maintenant(&self) -> DateTime<Utc> {
        self.horloge.now()
    }

    pub(crate) fn nouvel_id(&self) -> uuid::Uuid {
        self.ids.generer()
    }

    pub(crate) fn evenement(
        &self,
        type_evenement: TypeEvenement,
        cible: (&str, String),
        lundi: Option<Lundi>,
        resume: String,
        avant: Option<Value>,
        apres: Option<Value>,
    ) -> Evenement {
        Evenement {
            id: EvenementId(self.nouvel_id()),
            horodatage: self.maintenant(),
            type_evenement,
            cible_type: cible.0.to_owned(),
            cible_id: cible.1,
            lundi,
            resume,
            avant,
            apres,
        }
    }

    /// Noms des entrées des trois référentiels, archivées comprises.
    pub async fn noms(&self) -> Resultat<Noms> {
        let mut noms = HashMap::new();
        for nom in NomReferentiel::TOUS {
            for e in self.depot.referentiel(nom).await? {
                noms.insert(e.id, e);
            }
        }
        Ok(Noms(noms))
    }

    /// Réinitialise la démo ; sans effet hors démo (la route n'existe pas).
    pub async fn reinitialiser_demo(&self) -> bool {
        match &self.reinitialisable {
            Some(cible) => {
                cible.reinitialiser().await;
                true
            }
            None => false,
        }
    }
}

/// Résolution identifiant -> entrée de référentiel.
#[derive(Debug, Clone, Default)]
pub struct Noms(HashMap<EntreeId, EntreeReferentiel>);

impl Noms {
    pub fn entree(&self, id: EntreeId) -> Option<&EntreeReferentiel> {
        self.0.get(&id)
    }

    pub fn nom(&self, id: EntreeId) -> String {
        self.0
            .get(&id)
            .map_or_else(|| "(entrée inconnue)".to_owned(), |e| e.nom.clone())
    }
}

pub(crate) fn json<T: serde::Serialize>(valeur: &T) -> Option<Value> {
    serde_json::to_value(valeur).ok()
}
