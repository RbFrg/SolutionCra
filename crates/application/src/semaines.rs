//! Tableau de bord, semaines et plan d'alternance (tranche 1).

use chrono::{DateTime, NaiveDate, Utc};
use serde::Serialize;

use domain::journal::TypeEvenement;
use domain::plan::{appliquer_plage, type_attendu};
use domain::semaine::{date_paris, semaine_courante};
use domain::statut::{Compteurs, SemaineCalculee, calculer_semaines, compter};
use domain::{
    AnneeScolaire, Fiche, FiltreType, Lundi, SemainePlan, StatutSemaine, TypeAttendu, TypeFiche,
};

use crate::app::{App, json};
use crate::erreur::{ErreurApp, Resultat};
use crate::ports::Lot;

#[derive(Debug, Clone, PartialEq)]
pub struct AFaireCetteSemaine {
    pub lundi: Lundi,
    pub type_fiche: TypeFiche,
    pub echeance: DateTime<Utc>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct TableauDeBord {
    pub aujourdhui: NaiveDate,
    pub semaine_courante: Option<Lundi>,
    pub compteurs: Compteurs,
    pub a_faire_cette_semaine: Option<AFaireCetteSemaine>,
    /// Semaines en retard, de la plus ancienne à la plus récente (T-05).
    pub retards: Vec<SemaineCalculee>,
    pub semaines: Vec<SemaineCalculee>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Alertes {
    pub genere_le: DateTime<Utc>,
    pub aujourdhui: NaiveDate,
    pub en_retard: Vec<SemaineCalculee>,
    pub a_faire: Vec<SemaineCalculee>,
}

impl App {
    pub(crate) async fn fiches_seules(&self) -> Resultat<Vec<Fiche>> {
        Ok(self
            .depot
            .fiches()
            .await?
            .into_iter()
            .map(|s| s.fiche)
            .collect())
    }

    /// Statuts calculés des semaines d'une année (3.4).
    pub async fn semaines(
        &self,
        annee: AnneeScolaire,
        filtre: FiltreType,
    ) -> Resultat<Vec<SemaineCalculee>> {
        let plan = self.depot.plan().await?;
        let fiches = self.fiches_seules().await?;
        let justifications = self.depot.justifications().await?;
        Ok(calculer_semaines(
            annee.semaines(),
            &plan,
            &fiches,
            &justifications,
            filtre,
            self.maintenant(),
        ))
    }

    pub async fn tableau_de_bord(
        &self,
        annee: AnneeScolaire,
        filtre: FiltreType,
    ) -> Resultat<TableauDeBord> {
        let instant = self.maintenant();
        let semaines = self.semaines(annee, filtre).await?;
        let compteurs = compter(&semaines, instant);
        let courante = semaine_courante(instant);
        let a_faire_cette_semaine = semaines
            .iter()
            .find(|s| Some(s.lundi) == courante && s.statut == StatutSemaine::AFaire)
            .and_then(|s| {
                s.type_evalue.map(|t| AFaireCetteSemaine {
                    lundi: s.lundi,
                    type_fiche: t,
                    echeance: s.lundi.echeance(),
                })
            });
        let retards = semaines
            .iter()
            .filter(|s| s.statut == StatutSemaine::EnRetard)
            .cloned()
            .collect();
        Ok(TableauDeBord {
            aujourdhui: date_paris(instant),
            semaine_courante: courante,
            compteurs,
            a_faire_cette_semaine,
            retards,
            semaines,
        })
    }

    /// Alertes pour n8n (4.6) : même calcul que le tableau de bord, sur tout le suivi.
    pub async fn alertes(&self) -> Resultat<Alertes> {
        let instant = self.maintenant();
        let mut en_retard = Vec::new();
        let mut a_faire = Vec::new();
        for annee in [2026, 2027] {
            let annee =
                AnneeScolaire::new(annee).map_err(|e| ErreurApp::Technique(e.to_string()))?;
            for s in self.semaines(annee, FiltreType::Toutes).await? {
                match s.statut {
                    StatutSemaine::EnRetard => en_retard.push(s),
                    StatutSemaine::AFaire => a_faire.push(s),
                    _ => {}
                }
            }
        }
        Ok(Alertes {
            genere_le: instant,
            aujourdhui: date_paris(instant),
            en_retard,
            a_faire,
        })
    }

    /// Applique un type à une plage de semaines (P-05). Si des semaines déjà
    /// commencées changent de type, la confirmation est exigée (P-06).
    pub async fn appliquer_plan(
        &self,
        du: Lundi,
        au: Lundi,
        type_attendu_voulu: TypeAttendu,
        libelle: Option<&str>,
        confirmer: bool,
    ) -> Resultat<Vec<SemainePlan>> {
        let instant = self.maintenant();
        let nouvelles = appliquer_plage(du, au, type_attendu_voulu, libelle)?;
        let plan = self.depot.plan().await?;
        let avant: Vec<SemainePlan> = nouvelles
            .iter()
            .map(|n| {
                plan.iter()
                    .find(|p| p.lundi == n.lundi)
                    .cloned()
                    .unwrap_or_else(|| SemainePlan::par_defaut(n.lundi))
            })
            .collect();
        let passees: Vec<Lundi> = nouvelles
            .iter()
            .filter(|n| {
                n.lundi.debut() <= instant && type_attendu(&plan, n.lundi) != n.type_attendu
            })
            .map(|n| n.lundi)
            .collect();
        if !passees.is_empty() && !confirmer {
            return Err(ErreurApp::ConfirmationRequise { semaines: passees });
        }
        let resume = if du == au {
            format!("S{} : {}", du.numero(), type_attendu_voulu.code())
        } else {
            format!(
                "S{} à S{} : {}",
                du.numero(),
                au.numero(),
                type_attendu_voulu.code()
            )
        };
        let evenement = self.evenement(
            TypeEvenement::CalendrierModifie,
            ("semaine", format!("{du}..{au}")),
            Some(du),
            resume,
            json(&PlanJournal::de(&avant)),
            json(&PlanJournal::de(&nouvelles)),
        );
        self.depot
            .valider(Lot {
                plan: nouvelles.clone(),
                evenements: vec![evenement],
                ..Lot::default()
            })
            .await?;
        Ok(nouvelles)
    }
}

/// Forme stable des lignes de plan dans le journal (avant / après).
#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct PlanJournal {
    lundi: Lundi,
    type_attendu: TypeAttendu,
    libelle: Option<String>,
}

impl PlanJournal {
    fn de(lignes: &[SemainePlan]) -> Vec<Self> {
        lignes
            .iter()
            .map(|l| Self {
                lundi: l.lundi,
                type_attendu: l.type_attendu,
                libelle: l.libelle.clone(),
            })
            .collect()
    }
}
