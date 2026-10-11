//! SQLite en mémoire : S1 à S9, JRN2, JRN3.

#![allow(clippy::unwrap_used, clippy::expect_used)]

use adapter_memory::{DepotMemoire, EtatMemoire, jeu_demo};
use adapter_store::DepotSqlite;
use application::ports::{Depot, EcritureFiche, ErreurDepot, Lot};
use domain::journal::FiltreJournal;
use domain::{NomReferentiel, StatutFiche};
use pretty_assertions::assert_eq;

/// Le jeu de démo sous forme d'un seul lot, pour remplir une base.
fn lot_de(etat: &EtatMemoire) -> Lot {
    let mut referentiels = Vec::new();
    for (nom, liste) in [
        (NomReferentiel::Missions, &etat.missions),
        (NomReferentiel::Competences, &etat.competences),
        (NomReferentiel::Motifs, &etat.motifs),
    ] {
        referentiels.extend(liste.iter().cloned().map(|e| (nom, e)));
    }
    Lot {
        fiches: etat
            .fiches
            .values()
            .map(|s| EcritureFiche::Creer(s.fiche.clone()))
            .collect(),
        plan: etat.plan.values().cloned().collect(),
        justifications: etat.justifications.values().cloned().collect(),
        referentiels,
        evenements: etat.journal.clone(),
    }
}

async fn base_demo() -> DepotSqlite {
    let depot = DepotSqlite::en_memoire().await.unwrap();
    depot.valider(lot_de(&jeu_demo())).await.unwrap();
    depot
}

#[tokio::test]
async fn migrations_s_appliquent_sur_une_base_vide() {
    let depot = DepotSqlite::en_memoire().await.unwrap();
    assert!(depot.fiches().await.unwrap().is_empty());
    assert!(depot.plan().await.unwrap().is_empty());
}

#[tokio::test]
async fn sauvegarde_puis_chargement_donne_la_meme_fiche() {
    let depot = base_demo().await;
    let jeu = jeu_demo();
    for attendue in jeu.fiches.values() {
        let lue = depot.fiche(attendue.fiche.id).await.unwrap().unwrap();
        assert_eq!(lue.fiche, attendue.fiche);
        assert_eq!(lue.version, 1);
    }
}

#[tokio::test]
async fn l_ordre_des_lignes_et_des_competences_est_preserve() {
    let depot = base_demo().await;
    let s38 = jeu_demo()
        .fiches
        .values()
        .find(|s| s.fiche.lundi.numero() == 38)
        .unwrap()
        .fiche
        .clone();
    let mut inversee = s38.clone();
    inversee.lignes.reverse();
    inversee.lignes[1].competences.reverse();
    depot
        .valider(Lot {
            fiches: vec![EcritureFiche::Remplacer {
                fiche: inversee.clone(),
                version_attendue: 1,
            }],
            ..Lot::default()
        })
        .await
        .unwrap();
    let lue = depot.fiche(s38.id).await.unwrap().unwrap();
    assert_eq!(lue.fiche.lignes, inversee.lignes);
    assert_eq!(lue.version, 2);
}

#[tokio::test]
async fn une_version_perimee_est_refusee() {
    let depot = base_demo().await;
    let s38 = jeu_demo()
        .fiches
        .values()
        .find(|s| s.fiche.lundi.numero() == 38)
        .unwrap()
        .fiche
        .clone();
    let erreur = depot
        .valider(Lot {
            fiches: vec![EcritureFiche::Remplacer {
                fiche: s38,
                version_attendue: 7,
            }],
            ..Lot::default()
        })
        .await
        .unwrap_err();
    assert_eq!(
        erreur,
        ErreurDepot::Conflit {
            version_courante: 1
        }
    );
}

#[tokio::test]
async fn suppression_d_une_fiche_supprime_ses_lignes() {
    let depot = base_demo().await;
    let s36 = jeu_demo()
        .fiches
        .values()
        .find(|s| s.fiche.lundi.numero() == 36)
        .unwrap()
        .fiche
        .id;
    depot
        .valider(Lot {
            fiches: vec![EcritureFiche::Supprimer { id: s36 }],
            ..Lot::default()
        })
        .await
        .unwrap();
    let restantes: i64 = sqlx::query_scalar("SELECT count(*) FROM ligne WHERE fiche_id = ?")
        .bind(s36.to_string())
        .fetch_one(depot.pool())
        .await
        .unwrap();
    assert_eq!(restantes, 0);
}

#[tokio::test]
async fn type_de_fiche_invalide_est_rejete_par_la_contrainte() {
    let depot = DepotSqlite::en_memoire().await.unwrap();
    let resultat = sqlx::query(
        "INSERT INTO fiche (id, lundi, type_fiche, created_at, updated_at) VALUES ('x', '2026-10-05', 'les_deux', 'a', 'a')",
    )
    .execute(depot.pool())
    .await;
    assert!(resultat.is_err());
}

#[tokio::test]
async fn l_unicite_lundi_type_est_imposee_par_la_base() {
    let depot = base_demo().await;
    let resultat = sqlx::query(
        "INSERT INTO fiche (id, lundi, type_fiche, created_at, updated_at) VALUES ('y', '2026-09-14', 'entreprise', 'a', 'a')",
    )
    .execute(depot.pool())
    .await;
    assert!(resultat.is_err());
}

#[tokio::test]
async fn un_seul_index_actif_par_semaine_et_type_pour_les_justifications() {
    let depot = base_demo().await;
    let mut seconde = jeu_demo().justifications.values().next().unwrap().clone();
    seconde.id = domain::JustificationId(uuid::Uuid::from_u128(77));
    let erreur = depot
        .valider(Lot {
            justifications: vec![seconde],
            ..Lot::default()
        })
        .await
        .unwrap_err();
    assert_eq!(erreur, ErreurDepot::JustificationActiveExistante);
}

#[tokio::test]
async fn une_duree_hors_quart_d_heure_est_rejetee_par_la_contrainte() {
    let depot = base_demo().await;
    let resultat = sqlx::query("UPDATE ligne SET duree_minutes = 20")
        .execute(depot.pool())
        .await;
    assert!(resultat.is_err());
}

#[tokio::test]
async fn update_sur_evenement_est_refuse_par_le_declencheur() {
    let depot = base_demo().await;
    let erreur = sqlx::query("UPDATE evenement SET resume = 'falsifié'")
        .execute(depot.pool())
        .await
        .unwrap_err();
    assert!(erreur.to_string().contains("journal en ajout seul"));
}

#[tokio::test]
async fn delete_sur_evenement_est_refuse_par_le_declencheur() {
    let depot = base_demo().await;
    let erreur = sqlx::query("DELETE FROM evenement")
        .execute(depot.pool())
        .await
        .unwrap_err();
    assert!(erreur.to_string().contains("journal en ajout seul"));
}

#[tokio::test]
async fn changement_metier_et_entree_de_journal_sont_atomiques() {
    let depot = base_demo().await;
    let avant = depot
        .journal(&FiltreJournal::default(), 1, 100)
        .await
        .unwrap()
        .total;
    let mut s38 = jeu_demo()
        .fiches
        .values()
        .find(|s| s.fiche.lundi.numero() == 38)
        .unwrap()
        .fiche
        .clone();
    s38.statut = StatutFiche::Validee;
    let mut evenement = jeu_demo().journal[0].clone();
    evenement.id = domain::EvenementId(uuid::Uuid::from_u128(99));
    // L'entrée de journal passerait, mais la fiche est périmée : rien n'est écrit.
    let lot = Lot {
        evenements: vec![evenement],
        fiches: vec![EcritureFiche::Remplacer {
            fiche: s38.clone(),
            version_attendue: 9,
        }],
        ..Lot::default()
    };
    assert!(depot.valider(lot).await.is_err());
    assert_eq!(
        depot
            .journal(&FiltreJournal::default(), 1, 100)
            .await
            .unwrap()
            .total,
        avant
    );
    assert_eq!(
        depot.fiche(s38.id).await.unwrap().unwrap().fiche.statut,
        StatutFiche::Transmise
    );
}

#[tokio::test]
async fn adapter_memory_et_adapter_store_passent_la_meme_suite_de_contrat() {
    let sqlite = base_demo().await;
    let memoire = DepotMemoire::new(jeu_demo());

    let mut plan_s = sqlite.plan().await.unwrap();
    let mut plan_m = memoire.plan().await.unwrap();
    plan_s.sort_by_key(|p| p.lundi);
    plan_m.sort_by_key(|p| p.lundi);
    assert_eq!(plan_s, plan_m);

    for nom in NomReferentiel::TOUS {
        let mut s = sqlite.referentiel(nom).await.unwrap();
        let mut m = memoire.referentiel(nom).await.unwrap();
        s.sort_by_key(|e| e.id);
        m.sort_by_key(|e| e.id);
        assert_eq!(s, m);
    }

    let mut fiches_s: Vec<_> = sqlite
        .fiches()
        .await
        .unwrap()
        .into_iter()
        .map(|s| s.fiche)
        .collect();
    let mut fiches_m: Vec<_> = memoire
        .fiches()
        .await
        .unwrap()
        .into_iter()
        .map(|s| s.fiche)
        .collect();
    fiches_s.sort_by_key(|f| f.id);
    fiches_m.sort_by_key(|f| f.id);
    assert_eq!(fiches_s, fiches_m);

    assert_eq!(
        sqlite.justifications().await.unwrap(),
        memoire.justifications().await.unwrap()
    );

    let filtre = FiltreJournal::default();
    assert_eq!(
        sqlite.journal(&filtre, 2, 10).await.unwrap(),
        memoire.journal(&filtre, 2, 10).await.unwrap()
    );

    let doublon = Lot {
        fiches: vec![EcritureFiche::Creer(fiches_m[0].clone())],
        ..Lot::default()
    };
    assert_eq!(
        sqlite.valider(doublon.clone()).await.unwrap_err(),
        memoire.valider(doublon).await.unwrap_err()
    );
}

#[tokio::test]
async fn une_base_sur_disque_survit_a_la_reouverture() {
    let dossier = tempfile::tempdir().unwrap();
    let chemin = dossier.path().join("cra.db");
    let depot = DepotSqlite::ouvrir(&chemin).await.unwrap();
    depot.valider(lot_de(&jeu_demo())).await.unwrap();
    depot.fermer().await;

    let rouverte = DepotSqlite::ouvrir(&chemin).await.unwrap();
    assert_eq!(rouverte.fiches().await.unwrap().len(), 3);
    rouverte.fermer().await;
}
