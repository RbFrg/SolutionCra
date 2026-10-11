//! Justification d'une semaine et levée (tranche 3, 3.6).

use domain::journal::TypeEvenement;
use domain::justification::{ErreurJustification, Justification};
use domain::plan::type_attendu;
use domain::statut::statut_semaine;
use domain::{JustificationId, Lundi, MotifId, NomReferentiel, StatutSemaine, TypeFiche};

use crate::app::{App, json};
use crate::erreur::{ErreurApp, Resultat};
use crate::ports::Lot;

fn erreur_justification(erreur: ErreurJustification) -> ErreurApp {
    match erreur {
        ErreurJustification::Validation(v) => ErreurApp::Validation(v),
        ErreurJustification::SemaineNonJustifiable(statut) => ErreurApp::regle(
            "semaine-non-justifiable",
            match statut {
                StatutSemaine::Faite => "Cette semaine a déjà une fiche faite.",
                StatutSemaine::Justifiee => "Cette semaine est déjà justifiée.",
                StatutSemaine::AVenir => "Une semaine à venir ne se justifie pas.",
                StatutSemaine::NonAttendue => "Aucune fiche n'est attendue cette semaine.",
                StatutSemaine::AFaire | StatutSemaine::EnRetard => "Semaine non justifiable.",
            },
        ),
        ErreurJustification::DejaLevee => {
            ErreurApp::regle("deja-levee", "Cette justification est déjà levée.")
        }
    }
}

impl App {
    pub async fn justifier_semaine(
        &self,
        lundi: Lundi,
        type_fiche: TypeFiche,
        motif_id: MotifId,
        commentaire: Option<&str>,
    ) -> Resultat<Justification> {
        let instant = self.maintenant();
        let motifs = self.depot.referentiel(NomReferentiel::Motifs).await?;
        let Some(motif) = motifs.iter().find(|m| m.id == motif_id) else {
            let mut v = domain::ErreurValidation::default();
            v.ajouter("motifId", "Choisissez un motif.");
            return Err(ErreurApp::Validation(v));
        };
        let plan = self.depot.plan().await?;
        let fiches = self.fiches_seules().await?;
        let justifications = self.depot.justifications().await?;
        let statut = statut_semaine(
            lundi,
            type_fiche,
            type_attendu(&plan, lundi),
            fiches
                .iter()
                .find(|f| f.lundi == lundi && f.type_fiche == type_fiche)
                .map(|f| f.statut),
            justifications
                .iter()
                .any(|j| j.lundi == lundi && j.type_fiche == type_fiche && j.est_active()),
            instant,
        );
        let justification = Justification::creer(
            JustificationId(self.nouvel_id()),
            lundi,
            type_fiche,
            motif,
            commentaire,
            statut,
            instant,
        )
        .map_err(erreur_justification)?;
        let evenement = self.evenement(
            TypeEvenement::SemaineJustifiee,
            ("justification", justification.id.to_string()),
            Some(lundi),
            format!(
                "Semaine justifiée, S{} {}, {}",
                lundi.numero(),
                type_fiche.libelle(),
                motif.nom
            ),
            None,
            json(&justification),
        );
        self.depot
            .valider(Lot {
                justifications: vec![justification.clone()],
                evenements: vec![evenement],
                ..Lot::default()
            })
            .await?;
        Ok(justification)
    }

    /// Lève une justification : la ligne reste, la semaine retrouve son statut (J-06).
    pub async fn lever_justification(&self, id: JustificationId) -> Resultat<Justification> {
        let mut justification = self
            .depot
            .justifications()
            .await?
            .into_iter()
            .find(|j| j.id == id)
            .ok_or(ErreurApp::Introuvable("Justification introuvable."))?;
        let avant = json(&justification);
        justification
            .lever(self.maintenant())
            .map_err(erreur_justification)?;
        let evenement = self.evenement(
            TypeEvenement::JustificationLevee,
            ("justification", id.to_string()),
            Some(justification.lundi),
            format!(
                "Justification levée, S{} {}",
                justification.lundi.numero(),
                justification.type_fiche.libelle()
            ),
            avant,
            json(&justification),
        );
        self.depot
            .valider(Lot {
                justifications: vec![justification.clone()],
                evenements: vec![evenement],
                ..Lot::default()
            })
            .await?;
        Ok(justification)
    }
}
