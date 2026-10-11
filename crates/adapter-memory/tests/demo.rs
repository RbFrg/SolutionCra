//! Mode démo : jeu de départ, indicateurs figés, réinitialisation (DEM1 à DEM4, DEM10).

#![allow(clippy::unwrap_used, clippy::expect_used)]

mod commun;

use adapter_memory::jeu_demo;
use commun::*;
use domain::{StatutFiche, StatutSemaine, TypeAttendu, TypeFiche};

#[tokio::test]
async fn le_tableau_de_bord_de_la_demo_donne_toujours_les_memes_indicateurs() {
    let ctx = demo();
    let tdb = ctx.app.tableau_de_bord(a2026(), TOUTES).await.unwrap();
    let c = tdb.compteurs;

    assert_eq!(tdb.aujourdhui.to_string(), "2026-10-11");
    assert_eq!(
        (c.attendues, c.faites, c.justifiees, c.en_retard, c.a_faire),
        (6, 3, 1, 1, 1)
    );
    assert_eq!(c.taux_faites, Some(0.6));
    assert_eq!(tdb.retards.len(), 1);
    assert_eq!(tdb.retards[0].lundi, lundi(2026, 9, 21));
    assert_eq!(tdb.retards[0].jours_retard, Some(14));
    let a_faire = tdb.a_faire_cette_semaine.unwrap();
    assert_eq!(a_faire.lundi, lundi(2026, 10, 5));
    assert_eq!(a_faire.type_fiche, TypeFiche::Entreprise);
    assert_eq!(a_faire.echeance.to_rfc3339(), "2026-10-11T21:59:59+00:00");
}

#[tokio::test]
async fn les_statuts_des_semaines_suivent_le_jeu_de_donnees() {
    let ctx = demo();
    let semaines = ctx.app.semaines(a2026(), TOUTES).await.unwrap();
    let statuts: Vec<StatutSemaine> = semaines.iter().take(12).map(|s| s.statut).collect();
    use StatutSemaine::*;
    assert_eq!(
        statuts,
        [
            Faite,
            Faite,
            Faite,
            EnRetard,
            Justifiee,
            AFaire,
            AVenir,
            AVenir,
            AVenir,
            NonAttendue,
            AVenir,
            AVenir
        ]
    );
}

#[test]
fn le_jeu_de_demo_couvre_toute_l_annee_2026_2027() {
    let jeu = jeu_demo();
    assert_eq!(jeu.plan.len(), 52);
    assert!(
        jeu.plan
            .values()
            .any(|p| p.type_attendu == TypeAttendu::Aucune && p.libelle.is_some())
    );
    let statuts: Vec<StatutFiche> = jeu.fiches.values().map(|s| s.fiche.statut).collect();
    assert!(statuts.contains(&StatutFiche::Validee));
    assert!(statuts.contains(&StatutFiche::Transmise));
    assert_eq!(jeu.journal.len(), 24);
    let total: u32 = jeu.fiches.values().map(|s| s.fiche.total_minutes()).sum();
    assert_eq!(total, 25 * 60);
}

#[test]
fn le_jeu_de_demo_est_identique_a_chaque_appel() {
    assert_eq!(jeu_demo(), jeu_demo());
}

#[tokio::test]
async fn reinitialiser_restaure_le_jeu_de_depart() {
    let ctx = demo();
    let conges = entree(&ctx.app, domain::NomReferentiel::Motifs, "Congés").await;
    ctx.app
        .justifier_semaine(lundi(2026, 9, 21), TypeFiche::Entreprise, conges, None)
        .await
        .unwrap();
    assert_ne!(ctx.depot.instantane().await, jeu_demo());

    assert!(ctx.app.reinitialiser_demo().await);

    assert_eq!(ctx.depot.instantane().await, jeu_demo());
}

#[tokio::test]
async fn la_date_est_figee_au_11_10_2026_quelle_que_soit_l_heure_reelle() {
    let ctx = demo();
    assert_eq!(ctx.app.maintenant(), application::mode::instant_demo());
    assert_eq!(
        domain::semaine::date_paris(ctx.app.maintenant()).to_string(),
        "2026-10-11"
    );
}

#[tokio::test]
async fn le_bilan_de_la_demo_reprend_les_valeurs_des_maquettes() {
    let ctx = demo();
    let (b, noms) = ctx.app.bilan(a2026(), TOUTES).await.unwrap();
    assert_eq!(b.taux_ponctualite, Some(0.5));
    assert_eq!(format!("{:.1}", b.delai_moyen_jours.unwrap()), "0.7");
    let missions: Vec<(String, u32)> = b
        .par_mission
        .iter()
        .map(|(id, m)| (noms.nom(*id), *m))
        .collect();
    assert!(missions.contains(&("Prise en main de l'environnement Oracle".to_owned(), 600)));
    assert!(missions.contains(&("Architecture hexagonale en Rust".to_owned(), 510)));
    assert!(missions.contains(&(
        "Mise en place du pipeline d'intégration continue".to_owned(),
        390
    )));
    let competences: Vec<(String, u32)> = b
        .par_competence
        .iter()
        .map(|(id, m)| (noms.nom(*id), *m))
        .collect();
    for attendu in [
        ("Concevoir une architecture", 690),
        ("Déployer et exploiter", 750),
        ("Tester et valider", 420),
        ("Documenter", 240),
    ] {
        assert!(
            competences.contains(&(attendu.0.to_owned(), attendu.1)),
            "{attendu:?}"
        );
    }
    assert_eq!(b.par_motif.len(), 1);
}
