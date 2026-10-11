//! Dépôt en mémoire : tests et mode démo (4.5). L'unité de travail applique
//! un lot sur une copie, puis l'échange : tout ou rien, comme SQLite.

use std::collections::BTreeMap;
use std::sync::Arc;

use async_trait::async_trait;
use tokio::sync::RwLock;

use application::ports::{
    Depot, EcritureFiche, ErreurDepot, FicheStockee, Lot, PageJournal, Reinitialisable,
};
use domain::journal::{Evenement, FiltreJournal, trier_recent_d_abord};
use domain::referentiel::cle_de_nom;
use domain::{
    EntreeReferentiel, FicheId, Justification, JustificationId, Lundi, NomReferentiel, SemainePlan,
};

#[derive(Debug, Clone, Default, PartialEq)]
pub struct EtatMemoire {
    pub plan: BTreeMap<Lundi, SemainePlan>,
    pub fiches: BTreeMap<FicheId, FicheStockee>,
    pub justifications: BTreeMap<JustificationId, Justification>,
    pub missions: Vec<EntreeReferentiel>,
    pub competences: Vec<EntreeReferentiel>,
    pub motifs: Vec<EntreeReferentiel>,
    pub journal: Vec<Evenement>,
}

impl EtatMemoire {
    fn referentiel_mut(&mut self, nom: NomReferentiel) -> &mut Vec<EntreeReferentiel> {
        match nom {
            NomReferentiel::Missions => &mut self.missions,
            NomReferentiel::Competences => &mut self.competences,
            NomReferentiel::Motifs => &mut self.motifs,
        }
    }

    fn referentiel(&self, nom: NomReferentiel) -> &[EntreeReferentiel] {
        match nom {
            NomReferentiel::Missions => &self.missions,
            NomReferentiel::Competences => &self.competences,
            NomReferentiel::Motifs => &self.motifs,
        }
    }

    /// Applique un lot en place ; l'appelant travaille sur une copie.
    pub fn appliquer(&mut self, lot: Lot) -> Result<(), ErreurDepot> {
        for ecriture in lot.fiches {
            match ecriture {
                EcritureFiche::Creer(fiche) => {
                    if let Some(existante) = self.fiches.values().find(|s| {
                        s.fiche.lundi == fiche.lundi && s.fiche.type_fiche == fiche.type_fiche
                    }) {
                        return Err(ErreurDepot::FicheExistante {
                            fiche_id: existante.fiche.id,
                        });
                    }
                    let instant = horodatage(&lot.evenements);
                    self.fiches.insert(
                        fiche.id,
                        FicheStockee {
                            fiche,
                            version: 1,
                            creee_le: instant,
                            modifiee_le: instant,
                        },
                    );
                }
                EcritureFiche::Remplacer {
                    fiche,
                    version_attendue,
                } => {
                    let instant = horodatage(&lot.evenements);
                    let stockee = self
                        .fiches
                        .get_mut(&fiche.id)
                        .ok_or(ErreurDepot::Introuvable)?;
                    if stockee.version != version_attendue {
                        return Err(ErreurDepot::Conflit {
                            version_courante: stockee.version,
                        });
                    }
                    stockee.fiche = fiche;
                    stockee.version += 1;
                    stockee.modifiee_le = instant.max(stockee.modifiee_le);
                }
                EcritureFiche::Supprimer { id } => {
                    self.fiches.remove(&id).ok_or(ErreurDepot::Introuvable)?;
                }
            }
        }
        for ligne in lot.plan {
            self.plan.insert(ligne.lundi, ligne);
        }
        for justification in lot.justifications {
            let doublon = justification.est_active()
                && self.justifications.values().any(|j| {
                    j.id != justification.id
                        && j.est_active()
                        && j.lundi == justification.lundi
                        && j.type_fiche == justification.type_fiche
                });
            if doublon {
                return Err(ErreurDepot::JustificationActiveExistante);
            }
            self.justifications.insert(justification.id, justification);
        }
        for (nom, entree) in lot.referentiels {
            let liste = self.referentiel_mut(nom);
            let cle = cle_de_nom(&entree.nom);
            if liste
                .iter()
                .any(|e| e.id != entree.id && cle_de_nom(&e.nom) == cle)
            {
                return Err(ErreurDepot::NomEnDouble);
            }
            match liste.iter_mut().find(|e| e.id == entree.id) {
                Some(existante) => *existante = entree,
                None => liste.push(entree),
            }
        }
        self.journal.extend(lot.evenements);
        Ok(())
    }
}

fn horodatage(evenements: &[Evenement]) -> chrono::DateTime<chrono::Utc> {
    evenements
        .first()
        .map_or_else(chrono::Utc::now, |e| e.horodatage)
}

/// Dépôt partagé ; `depart` est l'état restauré par `reinitialiser` (M-05).
#[derive(Debug, Clone, Default)]
pub struct DepotMemoire {
    etat: Arc<RwLock<EtatMemoire>>,
    depart: Arc<EtatMemoire>,
}

impl DepotMemoire {
    pub fn new(depart: EtatMemoire) -> Self {
        Self {
            etat: Arc::new(RwLock::new(depart.clone())),
            depart: Arc::new(depart),
        }
    }

    pub async fn instantane(&self) -> EtatMemoire {
        self.etat.read().await.clone()
    }
}

#[async_trait]
impl Depot for DepotMemoire {
    async fn plan(&self) -> Result<Vec<SemainePlan>, ErreurDepot> {
        Ok(self.etat.read().await.plan.values().cloned().collect())
    }

    async fn fiches(&self) -> Result<Vec<FicheStockee>, ErreurDepot> {
        Ok(self.etat.read().await.fiches.values().cloned().collect())
    }

    async fn fiche(&self, id: FicheId) -> Result<Option<FicheStockee>, ErreurDepot> {
        Ok(self.etat.read().await.fiches.get(&id).cloned())
    }

    async fn justifications(&self) -> Result<Vec<Justification>, ErreurDepot> {
        Ok(self
            .etat
            .read()
            .await
            .justifications
            .values()
            .cloned()
            .collect())
    }

    async fn referentiel(
        &self,
        nom: NomReferentiel,
    ) -> Result<Vec<EntreeReferentiel>, ErreurDepot> {
        Ok(self.etat.read().await.referentiel(nom).to_vec())
    }

    async fn journal(
        &self,
        filtre: &FiltreJournal,
        page: usize,
        taille: usize,
    ) -> Result<PageJournal, ErreurDepot> {
        let mut entrees: Vec<Evenement> = self
            .etat
            .read()
            .await
            .journal
            .iter()
            .filter(|e| filtre.accepte(e))
            .cloned()
            .collect();
        trier_recent_d_abord(&mut entrees);
        let total = entrees.len();
        let debut = page.saturating_sub(1).saturating_mul(taille);
        Ok(PageJournal {
            entrees: entrees.into_iter().skip(debut).take(taille).collect(),
            total,
        })
    }

    async fn valider(&self, lot: Lot) -> Result<(), ErreurDepot> {
        let mut etat = self.etat.write().await;
        let mut copie = etat.clone();
        copie.appliquer(lot)?;
        *etat = copie;
        Ok(())
    }
}

#[async_trait]
impl Reinitialisable for DepotMemoire {
    async fn reinitialiser(&self) {
        *self.etat.write().await = (*self.depart).clone();
    }
}
