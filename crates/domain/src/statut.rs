//! Statut de semaine **calculé**, jamais stocké (3.4), et compteurs (J-05).

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use crate::fiche::{Fiche, StatutFiche};
use crate::ids::{FicheId, JustificationId, MotifId};
use crate::justification::Justification;
use crate::plan::{SemainePlan, TypeAttendu, TypeFiche};
use crate::semaine::{Lundi, date_paris};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum StatutSemaine {
    Faite,
    AFaire,
    EnRetard,
    AVenir,
    NonAttendue,
    Justifiee,
}

/// Statut d'un couple (semaine, type) à un instant donné. Ordre d'évaluation
/// du tableau 3.4, du plus prioritaire au moins prioritaire.
pub fn statut_semaine(
    lundi: Lundi,
    type_fiche: TypeFiche,
    type_attendu: TypeAttendu,
    statut_fiche: Option<StatutFiche>,
    justification_active: bool,
    instant: DateTime<Utc>,
) -> StatutSemaine {
    if type_attendu.type_fiche() != Some(type_fiche) {
        return StatutSemaine::NonAttendue;
    }
    if statut_fiche.is_some_and(StatutFiche::est_faite) {
        return StatutSemaine::Faite;
    }
    if justification_active {
        return StatutSemaine::Justifiee;
    }
    if instant < lundi.debut() {
        StatutSemaine::AVenir
    } else if instant < lundi.fin() {
        StatutSemaine::AFaire
    } else {
        StatutSemaine::EnRetard
    }
}

/// Jours de retard : date de Paris d'aujourd'hui moins le dimanche de la
/// semaine ; le lundi suivant vaut 1 (T-05).
pub fn jours_retard(lundi: Lundi, instant: DateTime<Utc>) -> i64 {
    (date_paris(instant) - lundi.dimanche()).num_days()
}

/// Filtre Toutes / Entreprise / Formation (T-10) : il restreint l'ensemble sommé.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum FiltreType {
    #[default]
    Toutes,
    Entreprise,
    Formation,
}

impl FiltreType {
    pub fn type_fiche(self) -> Option<TypeFiche> {
        match self {
            FiltreType::Toutes => None,
            FiltreType::Entreprise => Some(TypeFiche::Entreprise),
            FiltreType::Formation => Some(TypeFiche::Formation),
        }
    }

    pub fn accepte(self, type_fiche: TypeFiche) -> bool {
        self.type_fiche().is_none_or(|t| t == type_fiche)
    }
}

/// Une semaine avec son statut calculé, telle que l'affichent le calendrier,
/// la frise et le calendrier d'alternance.
#[derive(Debug, Clone, PartialEq)]
pub struct SemaineCalculee {
    pub lundi: Lundi,
    pub type_attendu: TypeAttendu,
    pub libelle: Option<String>,
    /// Type évalué : celui du filtre, ou celui du plan pour « Toutes ».
    pub type_evalue: Option<TypeFiche>,
    pub statut: StatutSemaine,
    pub fiche: Option<(FicheId, StatutFiche)>,
    pub justification: Option<(JustificationId, MotifId)>,
    pub jours_retard: Option<i64>,
}

/// Calcule le statut de chaque semaine de `semaines` (3.4).
pub fn calculer_semaines(
    semaines: impl IntoIterator<Item = Lundi>,
    plan: &[SemainePlan],
    fiches: &[Fiche],
    justifications: &[Justification],
    filtre: FiltreType,
    instant: DateTime<Utc>,
) -> Vec<SemaineCalculee> {
    semaines
        .into_iter()
        .map(|lundi| {
            let ligne_plan = plan.iter().find(|s| s.lundi == lundi);
            let type_attendu = crate::plan::type_attendu(plan, lundi);
            let type_evalue = filtre.type_fiche().or(type_attendu.type_fiche());
            let fiche = type_evalue.and_then(|t| {
                fiches
                    .iter()
                    .find(|f| f.lundi == lundi && f.type_fiche == t)
                    .map(|f| (f.id, f.statut))
            });
            let justification = type_evalue.and_then(|t| {
                justifications
                    .iter()
                    .find(|j| j.lundi == lundi && j.type_fiche == t && j.est_active())
                    .map(|j| (j.id, j.motif))
            });
            let statut = match type_evalue {
                Some(t) => statut_semaine(
                    lundi,
                    t,
                    type_attendu,
                    fiche.map(|(_, s)| s),
                    justification.is_some(),
                    instant,
                ),
                None => StatutSemaine::NonAttendue,
            };
            SemaineCalculee {
                lundi,
                type_attendu,
                libelle: ligne_plan.and_then(|s| s.libelle.clone()),
                type_evalue,
                statut,
                fiche,
                justification,
                jours_retard: (statut == StatutSemaine::EnRetard)
                    .then(|| jours_retard(lundi, instant)),
            }
        })
        .collect()
}

#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct Compteurs {
    pub attendues: u32,
    pub faites: u32,
    pub justifiees: u32,
    pub en_retard: u32,
    pub a_faire: u32,
    /// Faite / (attendues − justifiées) ; absent si le dénominateur est nul.
    pub taux_faites: Option<f64>,
}

/// Attendues = Faite, À faire, En retard ou Justifiée dont le lundi est passé ;
/// À venir et Non attendue n'entrent dans aucun dénominateur (J-05, T-03).
pub fn compter(semaines: &[SemaineCalculee], instant: DateTime<Utc>) -> Compteurs {
    let mut c = Compteurs::default();
    for s in semaines.iter().filter(|s| s.lundi.debut() <= instant) {
        match s.statut {
            StatutSemaine::Faite => c.faites += 1,
            StatutSemaine::AFaire => c.a_faire += 1,
            StatutSemaine::EnRetard => c.en_retard += 1,
            StatutSemaine::Justifiee => c.justifiees += 1,
            StatutSemaine::AVenir | StatutSemaine::NonAttendue => continue,
        }
        c.attendues += 1;
    }
    let denominateur = c.attendues - c.justifiees;
    c.taux_faites = (denominateur > 0).then(|| f64::from(c.faites) / f64::from(denominateur));
    c
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::NaiveDate;
    use proptest::prelude::*;
    use rstest::rstest;

    fn lundi(a: i32, m: u32, j: u32) -> Lundi {
        Lundi::new(NaiveDate::from_ymd_opt(a, m, j).unwrap()).unwrap()
    }

    fn utc(texte: &str) -> DateTime<Utc> {
        texte.parse().unwrap()
    }

    fn statut_entreprise(lundi: Lundi, fiche: Option<StatutFiche>, instant: &str) -> StatutSemaine {
        statut_semaine(
            lundi,
            TypeFiche::Entreprise,
            TypeAttendu::Entreprise,
            fiche,
            false,
            utc(instant),
        )
    }

    #[test]
    fn dimanche_23h59m59_paris_est_encore_a_faire() {
        assert_eq!(
            statut_entreprise(lundi(2026, 10, 5), None, "2026-10-11T21:59:59Z"),
            StatutSemaine::AFaire
        );
    }

    #[test]
    fn lundi_00h00_paris_passe_en_retard_sans_delai_de_grace() {
        assert_eq!(
            statut_entreprise(lundi(2026, 10, 5), None, "2026-10-11T22:00:00Z"),
            StatutSemaine::EnRetard
        );
    }

    #[test]
    fn frontiere_de_retard_au_passage_a_l_heure_d_hiver_le_25_10_2026() {
        let s = lundi(2026, 10, 19);
        assert_eq!(
            statut_entreprise(s, None, "2026-10-25T22:59:59Z"),
            StatutSemaine::AFaire
        );
        assert_eq!(
            statut_entreprise(s, None, "2026-10-25T23:00:00Z"),
            StatutSemaine::EnRetard
        );
    }

    #[test]
    fn frontiere_de_retard_au_passage_a_l_heure_d_ete_le_28_03_2027() {
        let s = lundi(2027, 3, 22);
        assert_eq!(
            statut_entreprise(s, None, "2027-03-28T21:59:59Z"),
            StatutSemaine::AFaire
        );
        assert_eq!(
            statut_entreprise(s, None, "2027-03-28T22:00:00Z"),
            StatutSemaine::EnRetard
        );
    }

    #[test]
    fn lundi_00h00_paris_ouvre_la_semaine_et_la_seconde_d_avant_est_a_venir() {
        let s41 = lundi(2026, 10, 5);
        assert_eq!(
            statut_entreprise(s41, None, "2026-10-04T21:59:59Z"),
            StatutSemaine::AVenir
        );
        assert_eq!(
            statut_entreprise(s41, None, "2026-10-04T22:00:00Z"),
            StatutSemaine::AFaire
        );
    }

    #[test]
    fn plan_aucune_rend_la_semaine_non_attendue() {
        let statut = statut_semaine(
            lundi(2026, 11, 2),
            TypeFiche::Entreprise,
            TypeAttendu::Aucune,
            None,
            false,
            utc("2026-12-01T10:00:00Z"),
        );
        assert_eq!(statut, StatutSemaine::NonAttendue);
    }

    #[test]
    fn l_autre_type_que_celui_du_plan_est_non_attendu() {
        let statut = statut_semaine(
            lundi(2026, 9, 28),
            TypeFiche::Entreprise,
            TypeAttendu::Formation,
            None,
            false,
            utc("2026-12-01T10:00:00Z"),
        );
        assert_eq!(statut, StatutSemaine::NonAttendue);
    }

    #[rstest]
    #[case(StatutFiche::Terminee)]
    #[case(StatutFiche::Transmise)]
    #[case(StatutFiche::Validee)]
    fn fiche_terminee_transmise_ou_validee_donne_faite_meme_apres_l_echeance(
        #[case] statut: StatutFiche,
    ) {
        assert_eq!(
            statut_entreprise(lundi(2026, 9, 21), Some(statut), "2026-12-01T10:00:00Z"),
            StatutSemaine::Faite
        );
    }

    #[test]
    fn fiche_brouillon_est_a_faire_puis_en_retard() {
        let s41 = lundi(2026, 10, 5);
        let brouillon = Some(StatutFiche::Brouillon);
        assert_eq!(
            statut_entreprise(s41, brouillon, "2026-10-08T10:00:00Z"),
            StatutSemaine::AFaire
        );
        assert_eq!(
            statut_entreprise(s41, brouillon, "2026-10-12T10:00:00Z"),
            StatutSemaine::EnRetard
        );
    }

    #[test]
    fn une_fiche_faite_l_emporte_sur_la_justification() {
        let statut = statut_semaine(
            lundi(2026, 9, 21),
            TypeFiche::Entreprise,
            TypeAttendu::Entreprise,
            Some(StatutFiche::Terminee),
            true,
            utc("2026-10-11T10:00:00Z"),
        );
        assert_eq!(statut, StatutSemaine::Faite);
    }

    #[test]
    fn une_justification_active_donne_justifiee() {
        let statut = statut_semaine(
            lundi(2026, 9, 21),
            TypeFiche::Entreprise,
            TypeAttendu::Entreprise,
            Some(StatutFiche::Brouillon),
            true,
            utc("2026-10-11T10:00:00Z"),
        );
        assert_eq!(statut, StatutSemaine::Justifiee);
    }

    #[test]
    fn jours_de_retard_sont_comptes_depuis_le_dimanche_et_le_lundi_suivant_vaut_un() {
        let s39 = lundi(2026, 9, 21);
        assert_eq!(jours_retard(s39, utc("2026-09-27T22:00:00Z")), 1);
        assert_eq!(jours_retard(s39, utc("2026-10-11T10:00:00Z")), 14);
    }

    fn semaine(lundi: Lundi, statut: StatutSemaine) -> SemaineCalculee {
        SemaineCalculee {
            lundi,
            type_attendu: TypeAttendu::Entreprise,
            libelle: None,
            type_evalue: Some(TypeFiche::Entreprise),
            statut,
            fiche: None,
            justification: None,
            jours_retard: None,
        }
    }

    #[test]
    fn compteurs_excluent_a_venir_et_non_attendue_et_retirent_les_justifiees_du_denominateur() {
        use StatutSemaine::*;
        let semaines = vec![
            semaine(lundi(2026, 8, 31), Faite),
            semaine(lundi(2026, 9, 7), Faite),
            semaine(lundi(2026, 9, 14), Faite),
            semaine(lundi(2026, 9, 21), EnRetard),
            semaine(lundi(2026, 9, 28), Justifiee),
            semaine(lundi(2026, 10, 5), AFaire),
            semaine(lundi(2026, 10, 12), AVenir),
            semaine(lundi(2026, 11, 2), NonAttendue),
        ];
        let c = compter(&semaines, utc("2026-10-11T10:00:00Z"));
        assert_eq!(
            (c.attendues, c.faites, c.justifiees, c.en_retard, c.a_faire),
            (6, 3, 1, 1, 1)
        );
        assert_eq!(c.taux_faites, Some(0.6));
    }

    #[test]
    fn sans_semaine_attendue_le_taux_est_absent() {
        let c = compter(&[], utc("2026-10-11T10:00:00Z"));
        assert_eq!(c.taux_faites, None);
    }

    proptest! {
        #[test]
        fn statut_de_semaine_est_total_pour_tout_lundi_et_tout_instant(
            index in 0usize..105,
            secondes in 1_780_000_000i64..1_850_000_000i64,
            fiche in proptest::option::of(0u8..4),
            justifiee: bool,
            attendu in 0u8..3,
        ) {
            let lundi = Lundi::toutes().nth(index).unwrap();
            let instant = DateTime::<Utc>::from_timestamp(secondes, 0).unwrap();
            let statut_fiche = fiche.map(|n| match n {
                0 => StatutFiche::Brouillon,
                1 => StatutFiche::Terminee,
                2 => StatutFiche::Transmise,
                _ => StatutFiche::Validee,
            });
            let type_attendu = match attendu {
                0 => TypeAttendu::Entreprise,
                1 => TypeAttendu::Formation,
                _ => TypeAttendu::Aucune,
            };
            let statut = statut_semaine(lundi, TypeFiche::Entreprise, type_attendu, statut_fiche, justifiee, instant);
            if statut == StatutSemaine::EnRetard {
                prop_assert!(instant >= lundi.fin());
            }
            if statut == StatutSemaine::AVenir {
                prop_assert!(instant < lundi.debut());
            }
        }
    }
}
