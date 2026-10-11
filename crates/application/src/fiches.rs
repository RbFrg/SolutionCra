//! Fiches : création, lecture, sauvegarde, statut, suppression (tranches 2 et 3).

use chrono::NaiveDate;
use serde::Serialize;

use domain::fiche::{ErreurFiche, Referentiels, valider_lignes, verifier_creation};
use domain::journal::TypeEvenement;
use domain::{
    AnneeScolaire, Fiche, FicheId, FiltreType, LigneId, LigneSaisie, Lundi, NomReferentiel,
    StatutFiche, TypeFiche,
};

use crate::app::{App, json};
use crate::erreur::{ErreurApp, Resultat};
use crate::ports::{EcritureFiche, FicheStockee, Lot};

#[derive(Debug, Clone, Default, PartialEq)]
pub struct FiltreFiches {
    pub annee: Option<AnneeScolaire>,
    pub type_fiche: FiltreType,
    pub statut: Option<StatutFiche>,
}

fn erreur_fiche(erreur: ErreurFiche) -> ErreurApp {
    match erreur {
        ErreurFiche::Validation(v) => ErreurApp::Validation(v),
        ErreurFiche::TermineeSansLigne => {
            ErreurApp::regle("terminee-sans-ligne", erreur.to_string())
        }
        ErreurFiche::SautDeStatut { .. } => ErreurApp::regle("saut-de-statut", erreur.to_string()),
        ErreurFiche::TropLoinDansLeFutur => ErreurApp::regle("trop-loin", erreur.to_string()),
    }
}

/// Résumé d'une fiche pour le journal.
#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct EtatFiche {
    statut: StatutFiche,
    lignes: usize,
    total_minutes: u32,
}

impl EtatFiche {
    fn de(fiche: &Fiche) -> Self {
        Self {
            statut: fiche.statut,
            lignes: fiche.lignes.len(),
            total_minutes: fiche.total_minutes(),
        }
    }
}

fn libelle_semaine(fiche: &Fiche) -> String {
    format!("S{} {}", fiche.lundi.numero(), fiche.type_fiche.libelle())
}

impl App {
    /// Liste par lundi décroissant (A3).
    pub async fn lister_fiches(&self, filtre: &FiltreFiches) -> Resultat<Vec<FicheStockee>> {
        let mut fiches: Vec<FicheStockee> = self
            .depot
            .fiches()
            .await?
            .into_iter()
            .filter(|s| {
                filtre
                    .annee
                    .is_none_or(|a| s.fiche.lundi.annee_scolaire() == a)
                    && filtre.type_fiche.accepte(s.fiche.type_fiche)
                    && filtre.statut.is_none_or(|st| s.fiche.statut == st)
            })
            .collect();
        fiches.sort_by(|a, b| {
            b.fiche
                .lundi
                .cmp(&a.fiche.lundi)
                .then(b.fiche.id.cmp(&a.fiche.id))
        });
        Ok(fiches)
    }

    pub async fn lire_fiche(&self, id: FicheId) -> Resultat<FicheStockee> {
        self.depot
            .fiche(id)
            .await?
            .ok_or(ErreurApp::Introuvable("Fiche introuvable."))
    }

    /// Crée la fiche d'un (lundi, type) ; refuse un doublon (R-11, L-10).
    pub async fn creer_fiche(&self, lundi: Lundi, type_fiche: TypeFiche) -> Resultat<FicheStockee> {
        let instant = self.maintenant();
        verifier_creation(lundi, instant).map_err(erreur_fiche)?;
        if let Some(existante) = self
            .depot
            .fiches()
            .await?
            .into_iter()
            .find(|s| s.fiche.lundi == lundi && s.fiche.type_fiche == type_fiche)
        {
            return Err(ErreurApp::FicheExistante {
                fiche_id: existante.fiche.id,
                message: format!(
                    "Une fiche {} existe déjà pour la semaine {}.",
                    type_fiche.libelle(),
                    lundi.numero()
                ),
            });
        }
        let fiche = Fiche::nouvelle(FicheId(self.nouvel_id()), lundi, type_fiche);
        let evenement = self.evenement(
            TypeEvenement::FicheCreee,
            ("fiche", fiche.id.to_string()),
            Some(lundi),
            format!("Fiche créée, {}", libelle_semaine(&fiche)),
            None,
            json(&EtatFiche::de(&fiche)),
        );
        self.depot
            .valider(Lot {
                fiches: vec![EcritureFiche::Creer(fiche.clone())],
                evenements: vec![evenement],
                ..Lot::default()
            })
            .await?;
        self.lire_fiche(fiche.id).await
    }

    /// Sauvegarde automatique : remplace les lignes, avec la version connue (4.1).
    pub async fn sauvegarder_fiche(
        &self,
        id: FicheId,
        version: u32,
        saisies: &[LigneSaisie],
    ) -> Resultat<FicheStockee> {
        let stockee = self.lire_fiche(id).await?;
        if stockee.version != version {
            return Err(ErreurApp::Conflit {
                version_courante: stockee.version,
            });
        }
        let missions = self.depot.referentiel(NomReferentiel::Missions).await?;
        let competences = self.depot.referentiel(NomReferentiel::Competences).await?;
        let mut fiche = stockee.fiche.clone();
        let lignes = valider_lignes(
            &fiche,
            saisies,
            &Referentiels {
                missions: &missions,
                competences: &competences,
            },
            || LigneId(self.nouvel_id()),
        )?;
        let avant = EtatFiche::de(&fiche);
        let apres_transmission = fiche.remplacer_lignes(lignes, self.maintenant());
        let mut evenements = Vec::new();
        if apres_transmission {
            evenements.push(self.evenement(
                TypeEvenement::FicheModifieeApresTransmission,
                ("fiche", fiche.id.to_string()),
                Some(fiche.lundi),
                format!(
                    "Fiche modifiée après transmission, {}",
                    libelle_semaine(&fiche)
                ),
                json(&avant),
                json(&EtatFiche::de(&fiche)),
            ));
        }
        self.depot
            .valider(Lot {
                fiches: vec![EcritureFiche::Remplacer {
                    fiche,
                    version_attendue: version,
                }],
                evenements,
                ..Lot::default()
            })
            .await?;
        self.lire_fiche(id).await
    }

    /// Change le statut (3.5) ; `date` n'est lue que pour Transmise et Validée.
    pub async fn changer_statut(
        &self,
        id: FicheId,
        version: u32,
        statut: StatutFiche,
        date: Option<NaiveDate>,
    ) -> Resultat<FicheStockee> {
        let stockee = self.lire_fiche(id).await?;
        if stockee.version != version {
            return Err(ErreurApp::Conflit {
                version_courante: stockee.version,
            });
        }
        let mut fiche = stockee.fiche.clone();
        let ancien = fiche.statut;
        let change = fiche
            .changer_statut(statut, self.maintenant(), date)
            .map_err(erreur_fiche)?;
        if !change {
            return Ok(stockee);
        }
        let evenement = self.evenement(
            TypeEvenement::StatutModifie,
            ("fiche", fiche.id.to_string()),
            Some(fiche.lundi),
            format!(
                "Statut modifié, {} : {} vers {}",
                libelle_semaine(&fiche),
                ancien.libelle(),
                statut.libelle()
            ),
            json(&EtatFiche::de(&stockee.fiche)),
            json(&EtatFiche::de(&fiche)),
        );
        self.depot
            .valider(Lot {
                fiches: vec![EcritureFiche::Remplacer {
                    fiche,
                    version_attendue: version,
                }],
                evenements: vec![evenement],
                ..Lot::default()
            })
            .await?;
        self.lire_fiche(id).await
    }

    /// Supprime une fiche ; l'entrée `fiche_supprimee` reste au journal (V-06).
    pub async fn supprimer_fiche(&self, id: FicheId) -> Resultat<()> {
        let stockee = self.lire_fiche(id).await?;
        let evenement = self.evenement(
            TypeEvenement::FicheSupprimee,
            ("fiche", id.to_string()),
            Some(stockee.fiche.lundi),
            format!("Fiche supprimée, {}", libelle_semaine(&stockee.fiche)),
            json(&stockee.fiche),
            None,
        );
        self.depot
            .valider(Lot {
                fiches: vec![EcritureFiche::Supprimer { id }],
                evenements: vec![evenement],
                ..Lot::default()
            })
            .await?;
        Ok(())
    }
}
