//! Cas d'usage sur le dépôt en mémoire : scénarios d'acceptation de 20.5,
//! plan (PLN), fiches (FIC), justification (JUS), journal (JRN), référentiels (REF).

#![allow(clippy::unwrap_used, clippy::expect_used)]

mod commun;

use application::ErreurApp;
use application::exports::{FormatDocument, Periode};
use application::fiches::FiltreFiches;
use application::referentiels::ModificationEntree;
use application::{EcritureFiche, Lot};
use commun::*;
use domain::journal::{FiltreJournal, TypeEvenement};
use domain::{LigneSaisie, NomReferentiel, StatutFiche, StatutSemaine, TypeAttendu, TypeFiche};

async fn s(ctx: &Contexte, l: domain::Lundi) -> domain::statut::SemaineCalculee {
    ctx.app
        .semaines(a2026(), TOUTES)
        .await
        .unwrap()
        .into_iter()
        .find(|x| x.lundi == l)
        .unwrap()
}

async fn nb_journal(ctx: &Contexte) -> usize {
    ctx.app
        .journal(&FiltreJournal::default(), 1, None)
        .await
        .unwrap()
        .0
        .total
}

#[tokio::test]
async fn justifier_une_semaine_en_retard_la_passe_en_justifiee() {
    let ctx = demo();
    let conges = entree(&ctx.app, NomReferentiel::Motifs, "Congés").await;

    ctx.app
        .justifier_semaine(lundi(2026, 9, 21), TypeFiche::Entreprise, conges, None)
        .await
        .unwrap();

    let tdb = ctx.app.tableau_de_bord(a2026(), TOUTES).await.unwrap();
    assert_eq!(
        s(&ctx, lundi(2026, 9, 21)).await.statut,
        StatutSemaine::Justifiee
    );
    assert_eq!(tdb.compteurs.en_retard, 0);
    assert_eq!(tdb.compteurs.taux_faites, Some(0.75));
}

#[tokio::test]
async fn lever_restaure_le_statut_calcule_et_conserve_la_ligne() {
    let ctx = demo();
    let s40 = s(&ctx, lundi(2026, 9, 28)).await;
    let (id, _) = s40.justification.unwrap();

    ctx.app.lever_justification(id).await.unwrap();

    let s40 = s(&ctx, lundi(2026, 9, 28)).await;
    assert_eq!(s40.statut, StatutSemaine::EnRetard);
    assert_eq!(s40.jours_retard, Some(7));
    let tdb = ctx.app.tableau_de_bord(a2026(), TOUTES).await.unwrap();
    assert_eq!(tdb.compteurs.en_retard, 2);
    assert_eq!(tdb.compteurs.taux_faites, Some(0.5));
    assert!(
        ctx.depot
            .instantane()
            .await
            .justifications
            .contains_key(&id)
    );
}

#[tokio::test]
async fn motif_autre_sans_commentaire_est_refuse() {
    let ctx = demo();
    let autre = entree(&ctx.app, NomReferentiel::Motifs, "Autre").await;
    let erreur = ctx
        .app
        .justifier_semaine(lundi(2026, 9, 21), TypeFiche::Entreprise, autre, None)
        .await
        .unwrap_err();
    let ErreurApp::Validation(v) = erreur else {
        panic!("{erreur:?}")
    };
    assert_eq!(v.erreurs[0].champ, "commentaire");
}

#[tokio::test]
async fn justifier_une_semaine_a_venir_ou_non_attendue_est_refuse() {
    let ctx = demo();
    let conges = entree(&ctx.app, NomReferentiel::Motifs, "Congés").await;
    for (l, t) in [
        (lundi(2026, 10, 12), TypeFiche::Entreprise),
        (lundi(2026, 9, 21), TypeFiche::Formation),
    ] {
        let erreur = ctx
            .app
            .justifier_semaine(l, t, conges, None)
            .await
            .unwrap_err();
        assert!(matches!(
            erreur,
            ErreurApp::Regle {
                code: "semaine-non-justifiable",
                ..
            }
        ));
    }
}

#[tokio::test]
async fn une_seule_justification_active_par_semaine_et_type() {
    let ctx = demo();
    let conges = entree(&ctx.app, NomReferentiel::Motifs, "Congés").await;
    let erreur = ctx
        .app
        .justifier_semaine(lundi(2026, 9, 28), TypeFiche::Formation, conges, None)
        .await
        .unwrap_err();
    assert!(matches!(erreur, ErreurApp::Regle { .. }));
}

#[tokio::test]
async fn une_fiche_faite_l_emporte_sur_la_justification_et_la_justification_reste_au_journal() {
    let ctx = demo();
    let mission = entree(
        &ctx.app,
        NomReferentiel::Missions,
        "Architecture hexagonale en Rust",
    )
    .await;
    let fiche = ctx
        .app
        .creer_fiche(lundi(2026, 9, 28), TypeFiche::Formation)
        .await
        .unwrap();
    let fiche = ctx
        .app
        .sauvegarder_fiche(
            fiche.fiche.id,
            1,
            &[saisie(
                chrono::NaiveDate::from_ymd_opt(2026, 9, 29).unwrap(),
                mission,
            )],
        )
        .await
        .unwrap();
    ctx.app
        .changer_statut(fiche.fiche.id, fiche.version, StatutFiche::Terminee, None)
        .await
        .unwrap();

    assert_eq!(
        s(&ctx, lundi(2026, 9, 28)).await.statut,
        StatutSemaine::Faite
    );
    let justifiees = ctx
        .app
        .journal(
            &FiltreJournal {
                type_evenement: Some(TypeEvenement::SemaineJustifiee),
                ..FiltreJournal::default()
            },
            1,
            None,
        )
        .await
        .unwrap()
        .0;
    assert_eq!(justifiees.total, 1);
}

fn saisie(date: chrono::NaiveDate, mission: domain::EntreeId) -> LigneSaisie {
    LigneSaisie {
        id: None,
        date,
        mission,
        competences: vec![],
        duree_heures: 2.5,
        etapes: "Travail".to_owned(),
        echanges_tuteur: None,
    }
}

#[tokio::test]
async fn marquer_terminee_sans_ligne_est_refuse_avec_le_message_du_cdc() {
    let ctx = demo();
    let fiche = ctx
        .app
        .creer_fiche(lundi(2026, 10, 5), TypeFiche::Entreprise)
        .await
        .unwrap();
    let erreur = ctx
        .app
        .changer_statut(fiche.fiche.id, fiche.version, StatutFiche::Terminee, None)
        .await
        .unwrap_err();
    assert_eq!(
        erreur,
        ErreurApp::regle(
            "terminee-sans-ligne",
            "Ajoutez au moins une ligne avant de terminer la fiche."
        )
    );
}

#[tokio::test]
async fn seconde_fiche_meme_lundi_et_meme_type_est_refusee() {
    let ctx = demo();
    let erreur = ctx
        .app
        .creer_fiche(lundi(2026, 9, 14), TypeFiche::Entreprise)
        .await
        .unwrap_err();
    let ErreurApp::FicheExistante { message, .. } = erreur else {
        panic!("{erreur:?}")
    };
    assert_eq!(
        message,
        "Une fiche entreprise existe déjà pour la semaine 38."
    );
}

#[tokio::test]
async fn creer_fiche_persiste_ecrit_le_journal_et_retourne_la_version_1() {
    let ctx = demo();
    let avant = nb_journal(&ctx).await;
    let fiche = ctx
        .app
        .creer_fiche(lundi(2026, 10, 5), TypeFiche::Entreprise)
        .await
        .unwrap();
    assert_eq!(fiche.version, 1);
    assert_eq!(fiche.fiche.statut, StatutFiche::Brouillon);
    assert_eq!(ctx.app.lire_fiche(fiche.fiche.id).await.unwrap(), fiche);
    assert_eq!(nb_journal(&ctx).await, avant + 1);
}

#[tokio::test]
async fn lister_les_fiches_par_lundi_decroissant() {
    let ctx = demo();
    let fiches = ctx
        .app
        .lister_fiches(&FiltreFiches::default())
        .await
        .unwrap();
    let numeros: Vec<u32> = fiches.iter().map(|f| f.fiche.lundi.numero()).collect();
    assert_eq!(numeros, [38, 37, 36]);
    let transmises = ctx
        .app
        .lister_fiches(&FiltreFiches {
            statut: Some(StatutFiche::Transmise),
            ..FiltreFiches::default()
        })
        .await
        .unwrap();
    assert_eq!(transmises.len(), 2);
}

#[tokio::test]
async fn modifier_apres_transmission_pose_l_indicateur_et_ecrit_une_entree_au_journal() {
    let ctx = demo();
    let s38 = ctx
        .app
        .lister_fiches(&FiltreFiches::default())
        .await
        .unwrap()
        .remove(0);
    let mission = s38.fiche.lignes[0].mission;
    let avant = nb_journal(&ctx).await;

    let apres = ctx
        .app
        .sauvegarder_fiche(
            s38.fiche.id,
            s38.version,
            &[saisie(
                chrono::NaiveDate::from_ymd_opt(2026, 9, 17).unwrap(),
                mission,
            )],
        )
        .await
        .unwrap();

    assert_eq!(apres.fiche.statut, StatutFiche::Transmise);
    assert!(apres.fiche.modifiee_apres_transmission());
    assert_eq!(apres.version, s38.version + 1);
    assert_eq!(nb_journal(&ctx).await, avant + 1);
}

#[tokio::test]
async fn put_avec_version_perimee_retourne_un_conflit() {
    let ctx = demo();
    let s38 = ctx
        .app
        .lister_fiches(&FiltreFiches::default())
        .await
        .unwrap()
        .remove(0);
    let erreur = ctx
        .app
        .sauvegarder_fiche(s38.fiche.id, s38.version - 1, &[])
        .await
        .unwrap_err();
    assert_eq!(
        erreur,
        ErreurApp::Conflit {
            version_courante: s38.version
        }
    );
}

#[tokio::test]
async fn ligne_invalide_ne_modifie_pas_le_depot() {
    let ctx = demo();
    let s38 = ctx
        .app
        .lister_fiches(&FiltreFiches::default())
        .await
        .unwrap()
        .remove(0);
    let avant = ctx.depot.instantane().await;
    let mut mauvaise = saisie(
        chrono::NaiveDate::from_ymd_opt(2026, 9, 30).unwrap(),
        s38.fiche.lignes[0].mission,
    );
    mauvaise.duree_heures = 1.1;
    let erreur = ctx
        .app
        .sauvegarder_fiche(s38.fiche.id, s38.version, &[mauvaise])
        .await
        .unwrap_err();
    let ErreurApp::Validation(v) = erreur else {
        panic!()
    };
    let champs: Vec<&str> = v.erreurs.iter().map(|e| e.champ.as_str()).collect();
    assert_eq!(champs, ["lignes[0].date", "lignes[0].dureeHeures"]);
    assert_eq!(ctx.depot.instantane().await, avant);
}

#[tokio::test]
async fn changement_metier_et_entree_de_journal_sont_atomiques() {
    let ctx = demo();
    let s38 = ctx
        .app
        .lister_fiches(&FiltreFiches::default())
        .await
        .unwrap()
        .remove(0);
    let avant = ctx.depot.instantane().await;
    let evenement = avant.journal[0].clone();
    // Le plan serait valide, mais la fiche est périmée : rien ne doit passer.
    let lot = Lot {
        plan: vec![domain::SemainePlan::par_defaut(lundi(2026, 11, 2))],
        fiches: vec![EcritureFiche::Remplacer {
            fiche: s38.fiche.clone(),
            version_attendue: 1,
        }],
        evenements: vec![evenement],
        ..Lot::default()
    };
    use application::Depot;
    assert!(ctx.depot.valider(lot).await.is_err());
    assert_eq!(ctx.depot.instantane().await, avant);
}

#[tokio::test]
async fn les_entrees_d_une_fiche_supprimee_restent() {
    let ctx = demo();
    let s36 = ctx
        .app
        .lister_fiches(&FiltreFiches::default())
        .await
        .unwrap()
        .remove(2);
    let avant = nb_journal(&ctx).await;
    ctx.app.supprimer_fiche(s36.fiche.id).await.unwrap();
    assert!(ctx.app.lire_fiche(s36.fiche.id).await.is_err());
    let du_s36 = ctx
        .app
        .journal(
            &FiltreJournal {
                lundi: Some(lundi(2026, 8, 31)),
                ..FiltreJournal::default()
            },
            1,
            None,
        )
        .await
        .unwrap()
        .0;
    assert!(
        du_s36
            .entrees
            .iter()
            .any(|e| e.type_evenement == TypeEvenement::FicheSupprimee)
    );
    assert_eq!(nb_journal(&ctx).await, avant + 1);
}

#[tokio::test]
async fn passer_s42_a_s44_en_formation_ne_demande_pas_de_confirmation() {
    let ctx = demo();
    let lignes = ctx
        .app
        .appliquer_plan(
            lundi(2026, 10, 12),
            lundi(2026, 10, 26),
            TypeAttendu::Formation,
            None,
            false,
        )
        .await
        .unwrap();
    assert_eq!(lignes.len(), 3);
    assert_eq!(
        s(&ctx, lundi(2026, 10, 19)).await.type_attendu,
        TypeAttendu::Formation
    );
    let journal = ctx
        .app
        .journal(&FiltreJournal::default(), 1, None)
        .await
        .unwrap()
        .0;
    assert_eq!(
        journal.entrees[0].type_evenement,
        TypeEvenement::CalendrierModifie
    );
}

#[tokio::test]
async fn modifier_une_semaine_passee_sans_confirmation_renvoie_les_semaines_touchees() {
    let ctx = demo();
    let avant = ctx.depot.instantane().await;
    let erreur = ctx
        .app
        .appliquer_plan(
            lundi(2026, 9, 14),
            lundi(2026, 10, 12),
            TypeAttendu::Aucune,
            None,
            false,
        )
        .await
        .unwrap_err();
    assert_eq!(
        erreur,
        ErreurApp::ConfirmationRequise {
            semaines: vec![
                lundi(2026, 9, 14),
                lundi(2026, 9, 21),
                lundi(2026, 9, 28),
                lundi(2026, 10, 5)
            ]
        }
    );
    assert_eq!(ctx.depot.instantane().await, avant);
}

#[tokio::test]
async fn changer_le_type_recalcule_les_statuts_des_semaines_passees() {
    let ctx = demo();
    let avant = nb_journal(&ctx).await;
    ctx.app
        .appliquer_plan(
            lundi(2026, 9, 21),
            lundi(2026, 9, 21),
            TypeAttendu::Aucune,
            Some("Congés"),
            true,
        )
        .await
        .unwrap();
    assert_eq!(
        s(&ctx, lundi(2026, 9, 21)).await.statut,
        StatutSemaine::NonAttendue
    );
    assert_eq!(nb_journal(&ctx).await, avant + 1);
}

#[tokio::test]
async fn le_journal_est_pagine_du_plus_recent_au_plus_ancien_et_se_filtre() {
    let ctx = demo();
    let (page, numero, taille) = ctx
        .app
        .journal(&FiltreJournal::default(), 1, Some(10))
        .await
        .unwrap();
    assert_eq!(
        (page.total, numero, taille, page.entrees.len()),
        (24, 1, 10, 10)
    );
    assert!(page.entrees[0].horodatage >= page.entrees[1].horodatage);
    assert_eq!(page.entrees[0].type_evenement, TypeEvenement::StatutModifie);
    let statuts = ctx
        .app
        .journal(
            &FiltreJournal {
                type_evenement: Some(TypeEvenement::StatutModifie),
                ..FiltreJournal::default()
            },
            1,
            None,
        )
        .await
        .unwrap()
        .0;
    assert_eq!(statuts.total, 7);
}

#[tokio::test]
async fn renommer_une_entree_la_renomme_dans_les_lignes_existantes() {
    let ctx = demo();
    let id = entree(
        &ctx.app,
        NomReferentiel::Missions,
        "Architecture hexagonale en Rust",
    )
    .await;
    ctx.app
        .modifier_entree(
            NomReferentiel::Missions,
            id,
            ModificationEntree {
                nom: Some("Architecture hexagonale".to_owned()),
                ..ModificationEntree::default()
            },
        )
        .await
        .unwrap();
    let noms = ctx.app.noms().await.unwrap();
    assert_eq!(noms.nom(id), "Architecture hexagonale");
}

#[tokio::test]
async fn la_saisie_ne_propose_que_les_entrees_actives() {
    let ctx = demo();
    let actifs = ctx
        .app
        .referentiel(NomReferentiel::Motifs, false)
        .await
        .unwrap();
    let tous = ctx
        .app
        .referentiel(NomReferentiel::Motifs, true)
        .await
        .unwrap();
    assert_eq!((actifs.len(), tous.len()), (5, 6));
}

#[tokio::test]
async fn le_motif_autre_ne_peut_pas_etre_archive() {
    let ctx = demo();
    let autre = entree(&ctx.app, NomReferentiel::Motifs, "Autre").await;
    let erreur = ctx
        .app
        .modifier_entree(
            NomReferentiel::Motifs,
            autre,
            ModificationEntree {
                archivee: Some(true),
                ..ModificationEntree::default()
            },
        )
        .await
        .unwrap_err();
    assert!(matches!(
        erreur,
        ErreurApp::Regle {
            code: "archivage-interdit",
            ..
        }
    ));
}

#[tokio::test]
async fn les_motifs_par_defaut_sont_amorces_une_seule_fois_au_premier_demarrage() {
    let ctx = contexte_avec(
        adapter_memory::EtatMemoire::default(),
        application::Mode::Production,
    );
    ctx.app.amorcer().await.unwrap();
    ctx.app.amorcer().await.unwrap();
    let motifs = ctx
        .app
        .referentiel(NomReferentiel::Motifs, true)
        .await
        .unwrap();
    assert_eq!(motifs.len(), 5);
    assert!(motifs.iter().any(|m| m.code.as_deref() == Some("autre")));
}

#[tokio::test]
async fn modifier_un_referentiel_ecrit_referentiel_modifie_au_journal() {
    let ctx = demo();
    let avant = nb_journal(&ctx).await;
    let (_, avertissement) = ctx
        .app
        .ajouter_entree(NomReferentiel::Competences, "Rédaction technique", None)
        .await
        .unwrap();
    assert!(avertissement.is_some());
    assert_eq!(nb_journal(&ctx).await, avant + 1);
}

#[tokio::test]
async fn valider_un_import_est_atomique_et_l_apercu_n_ecrit_rien() {
    let ctx = demo();
    let avant = ctx.depot.instantane().await;
    let apercu = ctx
        .app
        .importer_referentiel(
            NomReferentiel::Missions,
            "nom\nVeille technique\nAudit\n",
            false,
        )
        .await
        .unwrap();
    assert_eq!(apercu.a_creer(), 2);
    assert_eq!(ctx.depot.instantane().await, avant);

    let erreur = ctx
        .app
        .importer_referentiel(NomReferentiel::Missions, "nom\nVeille\n\"   \"\n", true)
        .await
        .unwrap_err();
    assert!(matches!(erreur, ErreurApp::Validation(_)));
    assert_eq!(ctx.depot.instantane().await, avant);

    ctx.app
        .importer_referentiel(
            NomReferentiel::Missions,
            "nom\nVeille technique\nAudit\n",
            true,
        )
        .await
        .unwrap();
    assert_eq!(
        ctx.app
            .referentiel(NomReferentiel::Missions, true)
            .await
            .unwrap()
            .len(),
        5
    );
}

#[tokio::test]
async fn un_export_ecrit_une_entree_export_au_journal() {
    let ctx = demo();
    let s38 = ctx
        .app
        .lister_fiches(&FiltreFiches::default())
        .await
        .unwrap()
        .remove(0);
    let avant = nb_journal(&ctx).await;
    let fichier = ctx
        .app
        .exporter_fiche(s38.fiche.id, FormatDocument::Docx)
        .await
        .unwrap();
    assert_eq!(fichier.nom, "fiche-cra-entreprise-2026-S38.docx");
    assert_eq!(nb_journal(&ctx).await, avant + 1);
}

#[tokio::test]
async fn le_lot_par_mois_regroupe_les_fiches_du_mois_de_leur_lundi() {
    let ctx = demo();
    let septembre = ctx
        .app
        .exporter_lot(
            Periode::Mois {
                annee: 2026,
                mois: 9,
            },
            TOUTES,
            FormatDocument::Pdf,
        )
        .await
        .unwrap();
    assert_eq!(
        String::from_utf8(septembre.contenu).unwrap(),
        "Fiches CRA · septembre 2026 2"
    );
    let aout = ctx
        .app
        .exporter_lot(
            Periode::Mois {
                annee: 2026,
                mois: 8,
            },
            TOUTES,
            FormatDocument::Pdf,
        )
        .await
        .unwrap();
    assert_eq!(
        String::from_utf8(aout.contenu).unwrap(),
        "Fiches CRA · août 2026 1"
    );
    let vide = ctx
        .app
        .exporter_lot(
            Periode::Mois {
                annee: 2026,
                mois: 11,
            },
            TOUTES,
            FormatDocument::Pdf,
        )
        .await;
    assert!(matches!(vide, Err(ErreurApp::Introuvable(_))));
}

#[tokio::test]
async fn alertes_et_tableau_de_bord_donnent_les_memes_semaines() {
    let ctx = demo();
    let alertes = ctx.app.alertes().await.unwrap();
    let tdb = ctx.app.tableau_de_bord(a2026(), TOUTES).await.unwrap();
    assert_eq!(alertes.en_retard, tdb.retards);
    assert_eq!(alertes.a_faire.len(), 1);
    assert_eq!(alertes.a_faire[0].lundi, lundi(2026, 10, 5));
}

#[tokio::test]
async fn le_passage_du_temps_fait_basculer_s41_en_retard() {
    let ctx = demo();
    ctx.horloge.avancer(chrono::Duration::hours(6));
    let tdb = ctx.app.tableau_de_bord(a2026(), TOUTES).await.unwrap();
    assert_eq!(tdb.compteurs.en_retard, 2);
    assert_eq!(
        tdb.a_faire_cette_semaine.unwrap().lundi,
        lundi(2026, 10, 12)
    );
}
