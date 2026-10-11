//! Jeu de démonstration (20.5) : fictif, fixe, identique à chaque démarrage.
//! Il couvre toute l'année scolaire 2026-2027 et se lit à la date figée du
//! dimanche 11/10/2026 (semaine 41 « À faire »).

use chrono::{DateTime, NaiveDate, Utc};
use serde_json::json;

use application::ids::{IdGenerator, SequenceIds};
use application::ports::FicheStockee;
use domain::journal::{Evenement, TypeEvenement};
use domain::referentiel::CODE_AUTRE;
use domain::{
    AnneeScolaire, Duree, EntreeId, EntreeReferentiel, EvenementId, Fiche, FicheId, Justification,
    JustificationId, Ligne, LigneId, Lundi, SemainePlan, StatutFiche, TypeAttendu, TypeFiche,
};

use crate::depot::EtatMemoire;

/// Horodatage des identifiants du jeu (30/08/2026 16:00 UTC).
const MILLIS_JEU: u64 = 1_788_105_600_000;

fn d(m: u32, j: u32) -> NaiveDate {
    let annee = if m >= 8 { 2026 } else { 2027 };
    NaiveDate::from_ymd_opt(annee, m, j).unwrap_or_default()
}

fn utc(texte: &str) -> DateTime<Utc> {
    texte.parse().unwrap_or_default()
}

fn lundi(m: u32, j: u32) -> Lundi {
    Lundi::new(d(m, j)).unwrap_or(domain::semaine::PREMIER_LUNDI)
}

struct Fabrique {
    ids: SequenceIds,
}

impl Fabrique {
    fn uuid(&self) -> uuid::Uuid {
        self.ids.generer()
    }

    fn entree(&self, nom: &str, code: Option<&str>, archivee: bool) -> EntreeReferentiel {
        EntreeReferentiel {
            id: EntreeId(self.uuid()),
            nom: nom.to_owned(),
            code: code.map(str::to_owned),
            archivee,
        }
    }

    fn ligne(
        &self,
        date: NaiveDate,
        mission: EntreeId,
        competences: &[EntreeId],
        minutes: u16,
        etapes: &str,
        echanges: Option<&str>,
    ) -> Ligne {
        Ligne {
            id: LigneId(self.uuid()),
            date,
            mission,
            competences: competences.to_vec(),
            duree: Duree::minutes(minutes).unwrap_or(Duree::QUART_D_HEURE),
            etapes: etapes.to_owned(),
            echanges_tuteur: echanges.map(str::to_owned),
        }
    }

    #[allow(clippy::too_many_arguments)]
    fn evenement(
        &self,
        horodatage: &str,
        type_evenement: TypeEvenement,
        cible: (&str, String),
        lundi: Option<Lundi>,
        resume: &str,
        avant: Option<serde_json::Value>,
        apres: Option<serde_json::Value>,
    ) -> Evenement {
        Evenement {
            id: EvenementId(self.uuid()),
            horodatage: utc(horodatage),
            type_evenement,
            cible_type: cible.0.to_owned(),
            cible_id: cible.1,
            lundi,
            resume: resume.to_owned(),
            avant,
            apres,
        }
    }
}

/// Plan d'alternance 2026-2027 : 52 semaines (DEM10).
fn plan() -> Vec<SemainePlan> {
    use TypeAttendu::{Aucune, Entreprise, Formation};
    let debut = [
        Entreprise, Formation, Entreprise, Entreprise, Formation, Entreprise, // S36 à S41
        Entreprise, Entreprise, Formation, Aucune, Entreprise, Entreprise, // S42 à S47
    ];
    let vacances = [
        (d(12, 21), "Vacances de Noël"),
        (d(12, 28), "Vacances de Noël"),
        (d(2, 15), "Vacances d'hiver"),
        (d(4, 19), "Vacances de printemps"),
        (d(8, 2), "Congés d'été"),
        (d(8, 9), "Congés d'été"),
        (d(8, 16), "Congés d'été"),
    ];
    AnneeScolaire::new(2026)
        .map(|a| a.semaines().collect::<Vec<_>>())
        .unwrap_or_default()
        .into_iter()
        .enumerate()
        .map(|(i, l)| {
            if let Some((_, libelle)) = vacances.iter().find(|(v, _)| *v == l.date()) {
                return SemainePlan {
                    lundi: l,
                    type_attendu: Aucune,
                    libelle: Some((*libelle).to_owned()),
                };
            }
            // Au-delà de S47 : une semaine de formation toutes les quatre.
            let type_attendu = debut.get(i).copied().unwrap_or_else(|| {
                if (i - debut.len()).is_multiple_of(4) {
                    Formation
                } else {
                    Entreprise
                }
            });
            SemainePlan {
                lundi: l,
                type_attendu,
                libelle: (type_attendu == Aucune).then(|| "Congés".to_owned()),
            }
        })
        .collect()
}

/// Jeu de départ du mode démo, identique à chaque appel.
pub fn jeu_demo() -> EtatMemoire {
    let f = Fabrique {
        ids: SequenceIds::new(MILLIS_JEU),
    };

    let oracle = f.entree("Prise en main de l'environnement Oracle", None, false);
    let hexa = f.entree("Architecture hexagonale en Rust", None, false);
    let pipeline = f.entree(
        "Mise en place du pipeline d'intégration continue",
        None,
        false,
    );
    let concevoir = f.entree("Concevoir une architecture", None, false);
    let deployer = f.entree("Déployer et exploiter", None, false);
    let tester = f.entree("Tester et valider", None, false);
    let documenter = f.entree("Documenter", None, false);
    let conges = f.entree("Congés", None, false);
    let motifs = vec![
        conges.clone(),
        f.entree("Férié", None, false),
        f.entree("Maladie", None, false),
        f.entree("Déplacement", None, false),
        f.entree("Autre", Some(CODE_AUTRE), false),
        f.entree("Formation externe", None, true),
    ];
    let (o, h, p) = (oracle.id, hexa.id, pipeline.id);
    let (c, dp, t, doc) = (concevoir.id, deployer.id, tester.id, documenter.id);

    let s36 = Fiche {
        id: FicheId(f.uuid()),
        lundi: lundi(8, 31),
        type_fiche: TypeFiche::Entreprise,
        statut: StatutFiche::Validee,
        terminee_le: Some(utc("2026-09-06T18:30:00Z")),
        transmise_le: Some(d(9, 7)),
        validee_le: Some(d(9, 10)),
        modifiee_apres_transmission_le: None,
        lignes: vec![
            f.ligne(
                d(9, 1),
                o,
                &[doc],
                240,
                "Rédaction du runbook d'installation de la VM",
                Some("Relecture du runbook avec le tuteur."),
            ),
            f.ligne(
                d(9, 2),
                o,
                &[dp],
                180,
                "Création de la VM et ouverture des ports 80 et 443",
                None,
            ),
            f.ligne(
                d(9, 3),
                o,
                &[c, dp],
                180,
                "Organisation des conteneurs : Caddy et application",
                Some("Choix validé : un seul conteneur applicatif."),
            ),
        ],
    };
    let s37 = Fiche {
        id: FicheId(f.uuid()),
        lundi: lundi(9, 7),
        type_fiche: TypeFiche::Formation,
        statut: StatutFiche::Transmise,
        terminee_le: Some(utc("2026-09-13T19:00:00Z")),
        transmise_le: Some(d(9, 14)),
        validee_le: None,
        modifiee_apres_transmission_le: Some(utc("2026-10-11T12:40:00Z")),
        lignes: vec![
            f.ligne(
                d(9, 8),
                h,
                &[c],
                210,
                "Architecture hexagonale en Rust : ports et adaptateurs",
                None,
            ),
            f.ligne(
                d(9, 9),
                h,
                &[c, t],
                180,
                "Atelier : tests unitaires et doubles de test",
                Some("Échange sur les fakes en mémoire."),
            ),
            f.ligne(
                d(9, 10),
                h,
                &[c],
                120,
                "Mini-projet : adaptateur HTTP",
                None,
            ),
        ],
    };
    let s38 = Fiche {
        id: FicheId(f.uuid()),
        lundi: lundi(9, 14),
        type_fiche: TypeFiche::Entreprise,
        statut: StatutFiche::Transmise,
        terminee_le: Some(utc("2026-09-22T17:10:00Z")),
        transmise_le: Some(d(10, 11)),
        validee_le: None,
        modifiee_apres_transmission_le: None,
        lignes: vec![
            f.ligne(d(9, 15), p, &[dp, t], 240, "Mise en place du pipeline d'intégration continue\n1. Écrire le workflow\n2. Ajouter le cache Cargo", None),
            f.ligne(d(9, 16), p, &[dp], 150, "Écriture des tests du domaine et exécution en CI", Some("Point sur la couverture du domaine.")),
        ],
    };

    let s40 = Justification {
        id: JustificationId(f.uuid()),
        lundi: lundi(9, 28),
        type_fiche: TypeFiche::Formation,
        motif: conges.id,
        commentaire: Some("Congés posés".to_owned()),
        creee_le: utc("2026-10-11T07:12:00Z"),
        levee_le: None,
    };

    let statut = |s: StatutFiche| Some(json!({ "statut": s }));
    let fiche_cible = |fiche: &Fiche| ("fiche", fiche.id.to_string());
    let ajout = |e: &EntreeReferentiel| Some(json!({ "nom": e.nom }));
    let journal = vec![
        f.evenement("2026-08-30T16:00:00Z", TypeEvenement::ReferentielModifie, ("missions", oracle.id.to_string()), None, "Référentiel modifié (missions) : « Prise en main de l'environnement Oracle » ajoutée", None, ajout(&oracle)),
        f.evenement("2026-08-30T16:01:00Z", TypeEvenement::ReferentielModifie, ("missions", hexa.id.to_string()), None, "Référentiel modifié (missions) : « Architecture hexagonale en Rust » ajoutée", None, ajout(&hexa)),
        f.evenement("2026-08-30T16:02:00Z", TypeEvenement::ReferentielModifie, ("missions", pipeline.id.to_string()), None, "Référentiel modifié (missions) : « Mise en place du pipeline d'intégration continue » ajoutée", None, ajout(&pipeline)),
        f.evenement("2026-08-30T16:05:00Z", TypeEvenement::ReferentielModifie, ("competences", concevoir.id.to_string()), None, "Référentiel modifié (competences) : « Concevoir une architecture » ajoutée", None, ajout(&concevoir)),
        f.evenement("2026-08-30T16:06:00Z", TypeEvenement::ReferentielModifie, ("competences", deployer.id.to_string()), None, "Référentiel modifié (competences) : « Déployer et exploiter » ajoutée", None, ajout(&deployer)),
        f.evenement("2026-08-30T16:07:00Z", TypeEvenement::ReferentielModifie, ("competences", tester.id.to_string()), None, "Référentiel modifié (competences) : « Tester et valider » ajoutée", None, ajout(&tester)),
        f.evenement("2026-08-30T16:08:00Z", TypeEvenement::ReferentielModifie, ("competences", documenter.id.to_string()), None, "Référentiel modifié (competences) : « Documentation » ajoutée", None, Some(json!({ "nom": "Documentation" }))),
        f.evenement("2026-08-30T16:15:00Z", TypeEvenement::CalendrierModifie, ("semaine", "2026-08-31..2027-08-23".to_owned()), Some(lundi(8, 31)), "Calendrier modifié, S36 à S34 : plan de l'année 2026-2027", None, Some(json!({ "semaines": 52 }))),
        f.evenement("2026-08-31T06:30:00Z", TypeEvenement::FicheCreee, fiche_cible(&s36), Some(s36.lundi), "Fiche créée, S36 entreprise", None, statut(StatutFiche::Brouillon)),
        f.evenement("2026-09-06T18:30:00Z", TypeEvenement::StatutModifie, fiche_cible(&s36), Some(s36.lundi), "Statut modifié, S36 entreprise : Brouillon vers Terminée", statut(StatutFiche::Brouillon), statut(StatutFiche::Terminee)),
        f.evenement("2026-09-07T06:45:00Z", TypeEvenement::FicheCreee, fiche_cible(&s37), Some(s37.lundi), "Fiche créée, S37 formation", None, statut(StatutFiche::Brouillon)),
        f.evenement("2026-09-07T07:00:00Z", TypeEvenement::StatutModifie, fiche_cible(&s36), Some(s36.lundi), "Statut modifié, S36 entreprise : Terminée vers Transmise", statut(StatutFiche::Terminee), statut(StatutFiche::Transmise)),
        f.evenement("2026-09-10T15:00:00Z", TypeEvenement::StatutModifie, fiche_cible(&s36), Some(s36.lundi), "Statut modifié, S36 entreprise : Transmise vers Validée", statut(StatutFiche::Transmise), statut(StatutFiche::Validee)),
        f.evenement("2026-09-13T19:00:00Z", TypeEvenement::StatutModifie, fiche_cible(&s37), Some(s37.lundi), "Statut modifié, S37 formation : Brouillon vers Terminée", statut(StatutFiche::Brouillon), statut(StatutFiche::Terminee)),
        f.evenement("2026-09-14T06:30:00Z", TypeEvenement::FicheCreee, fiche_cible(&s38), Some(s38.lundi), "Fiche créée, S38 entreprise", None, statut(StatutFiche::Brouillon)),
        f.evenement("2026-09-14T08:20:00Z", TypeEvenement::StatutModifie, fiche_cible(&s37), Some(s37.lundi), "Statut modifié, S37 formation : Terminée vers Transmise", statut(StatutFiche::Terminee), statut(StatutFiche::Transmise)),
        f.evenement("2026-09-22T17:10:00Z", TypeEvenement::StatutModifie, fiche_cible(&s38), Some(s38.lundi), "Statut modifié, S38 entreprise : Brouillon vers Terminée", statut(StatutFiche::Brouillon), statut(StatutFiche::Terminee)),
        f.evenement("2026-09-23T07:00:00Z", TypeEvenement::Export, ("export", "pdf".to_owned()), Some(s36.lundi), "Export pdf de la fiche S36 entreprise", None, Some(json!({ "format": "pdf" }))),
        f.evenement("2026-10-09T06:50:00Z", TypeEvenement::ReferentielModifie, ("competences", documenter.id.to_string()), None, "Référentiel modifié (competences) : « Documentation » renommée « Documenter »", Some(json!({ "nom": "Documentation" })), Some(json!({ "nom": "Documenter" }))),
        f.evenement("2026-10-10T15:30:00Z", TypeEvenement::Import, ("fiche", s38.id.to_string()), Some(s38.lundi), "Import de 3 lignes vers S38", None, Some(json!({ "lignes": 3 }))),
        f.evenement("2026-10-10T16:05:00Z", TypeEvenement::CalendrierModifie, ("semaine", "2026-10-12..2026-10-26".to_owned()), Some(lundi(10, 12)), "Calendrier modifié, S42 à S44", Some(json!([{ "typeAttendu": "entreprise" }])), Some(json!([{ "typeAttendu": "entreprise" }, { "typeAttendu": "entreprise" }, { "typeAttendu": "formation" }]))),
        f.evenement("2026-10-11T07:12:00Z", TypeEvenement::SemaineJustifiee, ("justification", s40.id.to_string()), Some(s40.lundi), "Semaine justifiée, S40 formation, Congés", None, Some(json!({ "motif": "Congés", "commentaire": "Congés posés" }))),
        f.evenement("2026-10-11T12:40:00Z", TypeEvenement::FicheModifieeApresTransmission, fiche_cible(&s37), Some(s37.lundi), "Fiche modifiée après transmission, S37 formation (1 ligne modifiée)", Some(json!({ "lignes": 3, "totalMinutes": 480 })), Some(json!({ "lignes": 3, "totalMinutes": 510 }))),
        f.evenement("2026-10-11T12:52:00Z", TypeEvenement::StatutModifie, fiche_cible(&s38), Some(s38.lundi), "Statut modifié, S38 entreprise : Terminée vers Transmise", statut(StatutFiche::Terminee), statut(StatutFiche::Transmise)),
    ];

    let stockee = |fiche: Fiche, version: u32, creee: &str, modifiee: &str| {
        (
            fiche.id,
            FicheStockee {
                fiche,
                version,
                creee_le: utc(creee),
                modifiee_le: utc(modifiee),
            },
        )
    };

    EtatMemoire {
        plan: plan().into_iter().map(|p| (p.lundi, p)).collect(),
        fiches: [
            stockee(s36, 1, "2026-08-31T06:30:00Z", "2026-09-10T15:00:00Z"),
            stockee(s37, 5, "2026-09-07T06:45:00Z", "2026-10-11T12:40:00Z"),
            stockee(s38, 3, "2026-09-14T06:30:00Z", "2026-10-11T12:52:00Z"),
        ]
        .into_iter()
        .collect(),
        justifications: [(s40.id, s40)].into_iter().collect(),
        missions: vec![oracle, hexa, pipeline],
        competences: vec![concevoir, deployer, tester, documenter],
        motifs,
        journal,
    }
}
