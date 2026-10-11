//! Bilan : totaux, ponctualité, délai moyen, retards par mois, motifs, temps
//! par mission et par compétence (3.8). Fonctions pures sur données chargées.

use std::collections::BTreeMap;

use chrono::{DateTime, Datelike, Utc};

use crate::fiche::Fiche;
use crate::ids::{CompetenceId, MissionId, MotifId};
use crate::justification::Justification;
use crate::plan::SemainePlan;
use crate::semaine::{AnneeScolaire, date_paris};
use crate::statut::{Compteurs, FiltreType, StatutSemaine, calculer_semaines, compter};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct LigneMois {
    pub annee: i32,
    pub mois: u32,
    pub en_retard: u32,
    pub justifiees: u32,
    pub faites: u32,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Bilan {
    pub compteurs: Compteurs,
    /// Fiches ponctuelles parmi les semaines échues, attendues, non justifiées.
    pub ponctuelles: u32,
    pub echues: u32,
    pub taux_ponctualite: Option<f64>,
    /// Jours entre le dimanche et la clôture ; zéro ou moins = à l'heure (B-04).
    pub delai_moyen_jours: Option<f64>,
    pub par_mois: Vec<LigneMois>,
    pub par_motif: Vec<(MotifId, u32)>,
    pub par_mission: Vec<(MissionId, u32)>,
    pub par_competence: Vec<(CompetenceId, u32)>,
    pub total_minutes: u32,
}

pub fn calculer_bilan(
    annee: AnneeScolaire,
    filtre: FiltreType,
    plan: &[SemainePlan],
    fiches: &[Fiche],
    justifications: &[Justification],
    instant: DateTime<Utc>,
) -> Bilan {
    let semaines = calculer_semaines(
        annee.semaines(),
        plan,
        fiches,
        justifications,
        filtre,
        instant,
    );
    let compteurs = compter(&semaines, instant);

    // Ponctualité : semaines échues (lundi suivant 00h00 passé), Faites ou En retard.
    let mut ponctuelles = 0;
    let mut echues = 0;
    let mut delais = Vec::new();
    for s in &semaines {
        let fiche = s
            .fiche
            .and_then(|(id, _)| fiches.iter().find(|f| f.id == id));
        if s.statut == StatutSemaine::Faite
            && let Some(termine) = fiche.and_then(|f| f.terminee_le)
        {
            delais.push((date_paris(termine) - s.lundi.dimanche()).num_days());
        }
        if s.lundi.fin() > instant {
            continue;
        }
        match s.statut {
            StatutSemaine::Faite => {
                echues += 1;
                if fiche
                    .and_then(|f| f.terminee_le)
                    .is_some_and(|t| t < s.lundi.fin())
                {
                    ponctuelles += 1;
                }
            }
            StatutSemaine::EnRetard => echues += 1,
            _ => {}
        }
    }
    let taux_ponctualite = (echues > 0).then(|| f64::from(ponctuelles) / f64::from(echues));
    #[allow(clippy::cast_precision_loss)]
    let delai_moyen_jours =
        (!delais.is_empty()).then(|| delais.iter().sum::<i64>() as f64 / delais.len() as f64);

    // Retards, justifications et fiches faites par mois du lundi.
    let mut mois: BTreeMap<(i32, u32), LigneMois> = BTreeMap::new();
    let aujourdhui = date_paris(instant);
    for s in semaines.iter().filter(|s| s.lundi.date() <= aujourdhui) {
        let d = s.lundi.date();
        let ligne = mois.entry((d.year(), d.month())).or_insert(LigneMois {
            annee: d.year(),
            mois: d.month(),
            en_retard: 0,
            justifiees: 0,
            faites: 0,
        });
        match s.statut {
            StatutSemaine::EnRetard => ligne.en_retard += 1,
            StatutSemaine::Justifiee => ligne.justifiees += 1,
            StatutSemaine::Faite => ligne.faites += 1,
            _ => {}
        }
    }

    let mut par_motif: BTreeMap<MotifId, u32> = BTreeMap::new();
    for s in &semaines {
        if let (StatutSemaine::Justifiee, Some((_, motif))) = (s.statut, s.justification) {
            *par_motif.entry(motif).or_default() += 1;
        }
    }

    // Temps : toutes les fiches de l'année et du type, quel que soit leur statut.
    let mut par_mission: BTreeMap<MissionId, u32> = BTreeMap::new();
    let mut par_competence: BTreeMap<CompetenceId, u32> = BTreeMap::new();
    let mut total_minutes = 0;
    for fiche in fiches
        .iter()
        .filter(|f| f.lundi.annee_scolaire() == annee && filtre.accepte(f.type_fiche))
    {
        for ligne in &fiche.lignes {
            let minutes = u32::from(ligne.duree.en_minutes());
            total_minutes += minutes;
            *par_mission.entry(ligne.mission).or_default() += minutes;
            // La durée entière compte pour chaque compétence de la ligne (B-06).
            for c in &ligne.competences {
                *par_competence.entry(*c).or_default() += minutes;
            }
        }
    }

    Bilan {
        compteurs,
        ponctuelles,
        echues,
        taux_ponctualite,
        delai_moyen_jours,
        par_mois: mois.into_values().collect(),
        par_motif: par_motif.into_iter().collect(),
        par_mission: par_mission.into_iter().collect(),
        par_competence: par_competence.into_iter().collect(),
        total_minutes,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::duree::Duree;
    use crate::fiche::{Ligne, StatutFiche};
    use crate::ids::{EntreeId, FicheId, JustificationId, LigneId};
    use crate::plan::{TypeAttendu, TypeFiche};
    use crate::semaine::Lundi;
    use chrono::NaiveDate;
    use uuid::Uuid;

    fn lundi(a: i32, m: u32, j: u32) -> Lundi {
        Lundi::new(NaiveDate::from_ymd_opt(a, m, j).unwrap()).unwrap()
    }

    fn utc(texte: &str) -> DateTime<Utc> {
        texte.parse().unwrap()
    }

    fn id(n: u128) -> EntreeId {
        EntreeId(Uuid::from_u128(n))
    }

    fn ligne(date: NaiveDate, mission: u128, competences: &[u128], minutes: u16) -> Ligne {
        Ligne {
            id: LigneId(Uuid::from_u128(u128::from(minutes) * 1000 + mission)),
            date,
            mission: id(mission),
            competences: competences.iter().map(|c| id(*c)).collect(),
            duree: Duree::minutes(minutes).unwrap(),
            etapes: String::new(),
            echanges_tuteur: None,
        }
    }

    fn fiche(n: u128, l: Lundi, t: TypeFiche, termine: Option<&str>, lignes: Vec<Ligne>) -> Fiche {
        Fiche {
            id: FicheId(Uuid::from_u128(n)),
            lundi: l,
            type_fiche: t,
            statut: if termine.is_some() {
                StatutFiche::Terminee
            } else {
                StatutFiche::Brouillon
            },
            terminee_le: termine.map(utc),
            transmise_le: None,
            validee_le: None,
            modifiee_apres_transmission_le: None,
            lignes,
        }
    }

    fn plan_formation(l: Lundi) -> SemainePlan {
        SemainePlan {
            lundi: l,
            type_attendu: TypeAttendu::Formation,
            libelle: None,
        }
    }

    /// Le jeu de la démo (20.5), réduit à ce qui compte pour le bilan.
    fn bilan_demo(filtre: FiltreType) -> Bilan {
        let d = |m, j| NaiveDate::from_ymd_opt(2026, m, j).unwrap();
        let plan = vec![
            plan_formation(lundi(2026, 9, 7)),
            plan_formation(lundi(2026, 9, 28)),
        ];
        let fiches = vec![
            fiche(
                1,
                lundi(2026, 8, 31),
                TypeFiche::Entreprise,
                Some("2026-09-06T18:30:00Z"),
                vec![
                    ligne(d(9, 1), 1, &[13], 240),
                    ligne(d(9, 2), 1, &[11], 180),
                    ligne(d(9, 3), 1, &[10, 11], 180),
                ],
            ),
            fiche(
                2,
                lundi(2026, 9, 7),
                TypeFiche::Formation,
                Some("2026-09-13T19:00:00Z"),
                vec![
                    ligne(d(9, 8), 2, &[10], 210),
                    ligne(d(9, 9), 2, &[10, 12], 180),
                    ligne(d(9, 10), 2, &[10], 120),
                ],
            ),
            fiche(
                3,
                lundi(2026, 9, 14),
                TypeFiche::Entreprise,
                Some("2026-09-22T17:10:00Z"),
                vec![
                    ligne(d(9, 15), 3, &[11, 12], 240),
                    ligne(d(9, 16), 3, &[11], 150),
                ],
            ),
        ];
        let justifications = vec![Justification {
            id: JustificationId(Uuid::from_u128(40)),
            lundi: lundi(2026, 9, 28),
            type_fiche: TypeFiche::Formation,
            motif: id(20),
            commentaire: Some("Congés posés".to_owned()),
            creee_le: utc("2026-10-11T07:12:00Z"),
            levee_le: None,
        }];
        calculer_bilan(
            AnneeScolaire::new(2026).unwrap(),
            filtre,
            &plan,
            &fiches,
            &justifications,
            utc("2026-10-11T16:00:00Z"),
        )
    }

    #[test]
    fn totaux_attendues_faites_justifiees_en_retard() {
        let c = bilan_demo(FiltreType::Toutes).compteurs;
        assert_eq!(
            (c.attendues, c.faites, c.justifiees, c.en_retard),
            (6, 3, 1, 1)
        );
    }

    #[test]
    fn ponctualite_du_jeu_de_demo_est_de_50_pour_cent() {
        let b = bilan_demo(FiltreType::Toutes);
        assert_eq!((b.ponctuelles, b.echues), (2, 4));
        assert_eq!(b.taux_ponctualite, Some(0.5));
    }

    #[test]
    fn delai_moyen_de_cloture_zero_ou_moins_signifie_a_l_heure() {
        let b = bilan_demo(FiltreType::Toutes);
        let delai = b.delai_moyen_jours.unwrap();
        assert!((delai - 2.0 / 3.0).abs() < 1e-9);
    }

    #[test]
    fn retards_par_mois() {
        let b = bilan_demo(FiltreType::Toutes);
        let septembre = b.par_mois.iter().find(|m| m.mois == 9).unwrap();
        let octobre = b.par_mois.iter().find(|m| m.mois == 10).unwrap();
        assert_eq!((septembre.en_retard, septembre.justifiees), (1, 1));
        assert_eq!((octobre.en_retard, octobre.justifiees), (0, 0));
    }

    #[test]
    fn justifications_par_motif() {
        assert_eq!(bilan_demo(FiltreType::Toutes).par_motif, vec![(id(20), 1)]);
    }

    #[test]
    fn temps_par_mission_et_par_competence_en_heures() {
        let b = bilan_demo(FiltreType::Toutes);
        assert_eq!(b.total_minutes, 25 * 60);
        assert_eq!(
            b.par_mission,
            vec![(id(1), 600), (id(2), 510), (id(3), 390)]
        );
        assert_eq!(
            b.par_competence,
            vec![(id(10), 690), (id(11), 750), (id(12), 420), (id(13), 240)]
        );
    }

    #[test]
    fn fiche_terminee_le_dimanche_23h59m59_est_ponctuelle() {
        let s = lundi(2026, 10, 5);
        let fiches = vec![fiche(
            1,
            s,
            TypeFiche::Entreprise,
            Some("2026-10-11T21:59:59Z"),
            vec![],
        )];
        let b = calculer_bilan(
            AnneeScolaire::new(2026).unwrap(),
            FiltreType::Entreprise,
            &[],
            &fiches,
            &[],
            utc("2026-10-20T10:00:00Z"),
        );
        assert_eq!(b.ponctuelles, 1);
    }

    #[test]
    fn fiche_terminee_le_lundi_00h00_n_est_pas_ponctuelle() {
        let s = lundi(2026, 10, 5);
        let fiches = vec![fiche(
            1,
            s,
            TypeFiche::Entreprise,
            Some("2026-10-11T22:00:00Z"),
            vec![],
        )];
        let b = calculer_bilan(
            AnneeScolaire::new(2026).unwrap(),
            FiltreType::Entreprise,
            &[],
            &fiches,
            &[],
            utc("2026-10-20T10:00:00Z"),
        );
        assert_eq!(b.ponctuelles, 0);
    }

    #[test]
    fn ponctualite_au_changement_d_heure_du_25_10_2026() {
        let s = lundi(2026, 10, 19);
        // Dimanche 25/10 à 23h30, heure d'hiver (UTC+1) : encore dans les temps.
        let fiches = vec![fiche(
            1,
            s,
            TypeFiche::Entreprise,
            Some("2026-10-25T22:30:00Z"),
            vec![],
        )];
        let b = calculer_bilan(
            AnneeScolaire::new(2026).unwrap(),
            FiltreType::Entreprise,
            &[],
            &fiches,
            &[],
            utc("2026-11-01T10:00:00Z"),
        );
        assert_eq!(b.ponctuelles, 1);
    }

    #[test]
    fn la_semaine_en_cours_n_est_pas_jugee_dans_la_ponctualite() {
        let b = bilan_demo(FiltreType::Toutes);
        // S41 est à faire au 11/10 : elle n'entre pas dans les échues.
        assert_eq!(b.echues, 4);
    }

    #[test]
    fn un_bilan_sans_semaine_attendue_ne_divise_pas_par_zero() {
        let b = calculer_bilan(
            AnneeScolaire::new(2026).unwrap(),
            FiltreType::Toutes,
            &[],
            &[],
            &[],
            utc("2026-08-01T10:00:00Z"),
        );
        assert_eq!(b.taux_ponctualite, None);
        assert_eq!(b.delai_moyen_jours, None);
        assert_eq!(b.compteurs.taux_faites, None);
    }

    #[test]
    fn le_filtre_de_type_et_l_annee_cadrent_le_bilan() {
        let b = bilan_demo(FiltreType::Formation);
        assert_eq!(b.total_minutes, 510);
        assert_eq!(b.compteurs.attendues, 2);
        let b2027 = calculer_bilan(
            AnneeScolaire::new(2027).unwrap(),
            FiltreType::Toutes,
            &[],
            &[],
            &[],
            utc("2026-10-11T16:00:00Z"),
        );
        assert_eq!(b2027.compteurs.attendues, 0);
    }
}
