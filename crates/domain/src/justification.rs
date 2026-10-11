//! Justification d'une semaine attendue sans fiche faite (3.6).

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use crate::erreur::{ErreurChamp, ErreurValidation};
use crate::ids::{JustificationId, MotifId};
use crate::plan::TypeFiche;
use crate::referentiel::{CODE_AUTRE, EntreeReferentiel};
use crate::semaine::Lundi;
use crate::statut::StatutSemaine;

pub const COMMENTAIRE_MAX: usize = 1000;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Justification {
    pub id: JustificationId,
    pub lundi: Lundi,
    pub type_fiche: TypeFiche,
    pub motif: MotifId,
    pub commentaire: Option<String>,
    pub creee_le: DateTime<Utc>,
    pub levee_le: Option<DateTime<Utc>>,
}

#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum ErreurJustification {
    #[error("seule une semaine à faire ou en retard se justifie (statut actuel : {0:?})")]
    SemaineNonJustifiable(StatutSemaine),
    #[error("la justification est déjà levée")]
    DejaLevee,
    #[error("{0}")]
    Validation(ErreurValidation),
}

impl Justification {
    pub fn est_active(&self) -> bool {
        self.levee_le.is_none()
    }

    /// Justifie une semaine **attendue, à faire ou en retard** (J-01) ; une
    /// justification active la rend « Justifiée », d'où l'unicité (JUS5).
    pub fn creer(
        id: JustificationId,
        lundi: Lundi,
        type_fiche: TypeFiche,
        motif: &EntreeReferentiel,
        commentaire: Option<&str>,
        statut_actuel: StatutSemaine,
        instant: DateTime<Utc>,
    ) -> Result<Self, ErreurJustification> {
        if !matches!(
            statut_actuel,
            StatutSemaine::AFaire | StatutSemaine::EnRetard
        ) {
            return Err(ErreurJustification::SemaineNonJustifiable(statut_actuel));
        }
        let commentaire = commentaire.map(str::trim).filter(|c| !c.is_empty());
        let mut erreurs = ErreurValidation::default();
        if motif.archivee {
            erreurs.ajouter("motifId", "Ce motif est archivé.");
        }
        if motif.code.as_deref() == Some(CODE_AUTRE) && commentaire.is_none() {
            erreurs.ajouter(
                "commentaire",
                "Précisez le motif : un commentaire est obligatoire pour « Autre ».",
            );
        }
        if commentaire.is_some_and(|c| c.chars().count() > COMMENTAIRE_MAX) {
            erreurs.ajouter(
                "commentaire",
                format!("Le commentaire ne doit pas dépasser {COMMENTAIRE_MAX} caractères."),
            );
        }
        erreurs
            .resultat()
            .map_err(ErreurJustification::Validation)?;
        Ok(Self {
            id,
            lundi,
            type_fiche,
            motif: motif.id,
            commentaire: commentaire.map(str::to_owned),
            creee_le: instant,
            levee_le: None,
        })
    }

    /// Lève la justification : la ligne reste, `levee_le` est renseigné (J-06).
    pub fn lever(&mut self, instant: DateTime<Utc>) -> Result<(), ErreurJustification> {
        if self.levee_le.is_some() {
            return Err(ErreurJustification::DejaLevee);
        }
        self.levee_le = Some(instant);
        Ok(())
    }
}

impl From<ErreurChamp> for ErreurJustification {
    fn from(erreur: ErreurChamp) -> Self {
        Self::Validation(ErreurValidation::une(erreur))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ids::EntreeId;
    use chrono::NaiveDate;
    use uuid::Uuid;

    fn motif(nom: &str, code: Option<&str>) -> EntreeReferentiel {
        EntreeReferentiel {
            id: EntreeId(Uuid::from_u128(1)),
            nom: nom.to_owned(),
            code: code.map(str::to_owned),
            archivee: false,
        }
    }

    fn s39() -> Lundi {
        Lundi::new(NaiveDate::from_ymd_opt(2026, 9, 21).unwrap()).unwrap()
    }

    fn instant() -> DateTime<Utc> {
        "2026-10-11T10:00:00Z".parse().unwrap()
    }

    fn justifier(
        motif: &EntreeReferentiel,
        commentaire: Option<&str>,
        statut: StatutSemaine,
    ) -> Result<Justification, ErreurJustification> {
        Justification::creer(
            JustificationId(Uuid::from_u128(9)),
            s39(),
            TypeFiche::Entreprise,
            motif,
            commentaire,
            statut,
            instant(),
        )
    }

    #[test]
    fn justifier_une_semaine_a_faire_est_accepte() {
        let j = justifier(&motif("Congés", None), None, StatutSemaine::AFaire).unwrap();
        assert!(j.est_active());
        assert_eq!(j.creee_le, instant());
    }

    #[test]
    fn justifier_une_semaine_a_venir_ou_non_attendue_est_refuse() {
        for statut in [StatutSemaine::AVenir, StatutSemaine::NonAttendue] {
            assert_eq!(
                justifier(&motif("Congés", None), None, statut),
                Err(ErreurJustification::SemaineNonJustifiable(statut))
            );
        }
    }

    #[test]
    fn justifier_une_semaine_deja_faite_ou_justifiee_est_refuse() {
        for statut in [StatutSemaine::Faite, StatutSemaine::Justifiee] {
            assert!(justifier(&motif("Congés", None), None, statut).is_err());
        }
    }

    #[test]
    fn motif_autre_sans_commentaire_est_refuse() {
        let erreur = justifier(
            &motif("Autre", Some("autre")),
            Some("  "),
            StatutSemaine::EnRetard,
        )
        .unwrap_err();
        let ErreurJustification::Validation(v) = erreur else {
            panic!("erreur de validation attendue");
        };
        assert_eq!(v.erreurs[0].champ, "commentaire");
    }

    #[test]
    fn motif_autre_avec_commentaire_est_accepte() {
        let j = justifier(
            &motif("Autre", Some("autre")),
            Some(" Panne de train "),
            StatutSemaine::EnRetard,
        )
        .unwrap();
        assert_eq!(j.commentaire.as_deref(), Some("Panne de train"));
    }

    #[test]
    fn lever_conserve_la_justification_et_renseigne_la_date() {
        let mut j = justifier(&motif("Congés", None), None, StatutSemaine::EnRetard).unwrap();
        j.lever(instant()).unwrap();
        assert_eq!(j.levee_le, Some(instant()));
        assert!(!j.est_active());
        assert_eq!(j.lever(instant()), Err(ErreurJustification::DejaLevee));
    }
}
