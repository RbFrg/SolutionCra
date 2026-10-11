//! Fiche hebdomadaire, lignes et cycle de vie (3.3, 3.5).

use std::collections::HashSet;

use chrono::{DateTime, NaiveDate, Utc};
use serde::{Deserialize, Serialize};

use crate::duree::Duree;
use crate::erreur::{ErreurChamp, ErreurValidation};
use crate::ids::{CompetenceId, FicheId, LigneId, MissionId};
use crate::plan::TypeFiche;
use crate::referentiel::EntreeReferentiel;
use crate::semaine::{Lundi, date_paris, semaine_courante};

pub const LIGNES_MAX: usize = 200;
pub const TEXTE_MAX: usize = 5000;
pub const COMPETENCES_MAX: usize = 20;

/// Cycle Brouillon → Terminée → Transmise → Validée, dans l'ordre croissant.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum StatutFiche {
    Brouillon,
    Terminee,
    Transmise,
    Validee,
}

impl StatutFiche {
    pub const TOUS: [StatutFiche; 4] = [
        StatutFiche::Brouillon,
        StatutFiche::Terminee,
        StatutFiche::Transmise,
        StatutFiche::Validee,
    ];

    /// « Faite » = Terminée, Transmise ou Validée (3.4).
    pub fn est_faite(self) -> bool {
        self >= StatutFiche::Terminee
    }

    pub fn suivant(self) -> Option<Self> {
        match self {
            StatutFiche::Brouillon => Some(StatutFiche::Terminee),
            StatutFiche::Terminee => Some(StatutFiche::Transmise),
            StatutFiche::Transmise => Some(StatutFiche::Validee),
            StatutFiche::Validee => None,
        }
    }

    pub fn code(self) -> &'static str {
        match self {
            StatutFiche::Brouillon => "brouillon",
            StatutFiche::Terminee => "terminee",
            StatutFiche::Transmise => "transmise",
            StatutFiche::Validee => "validee",
        }
    }

    pub fn libelle(self) -> &'static str {
        match self {
            StatutFiche::Brouillon => "Brouillon",
            StatutFiche::Terminee => "Terminée",
            StatutFiche::Transmise => "Transmise",
            StatutFiche::Validee => "Validée",
        }
    }

    pub fn depuis_code(code: &str) -> Option<Self> {
        Self::TOUS.into_iter().find(|s| s.code() == code)
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Ligne {
    pub id: LigneId,
    pub date: NaiveDate,
    pub mission: MissionId,
    pub competences: Vec<CompetenceId>,
    pub duree: Duree,
    pub etapes: String,
    pub echanges_tuteur: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Fiche {
    pub id: FicheId,
    pub lundi: Lundi,
    pub type_fiche: TypeFiche,
    pub statut: StatutFiche,
    /// Instant : la ponctualité le compare au lundi suivant 00h00 de Paris (3.8).
    pub terminee_le: Option<DateTime<Utc>>,
    /// Dates seules, de Paris (F-05).
    pub transmise_le: Option<NaiveDate>,
    pub validee_le: Option<NaiveDate>,
    /// Instant de la dernière modification de contenu faite alors que la fiche
    /// était Transmise ou Validée (F-07) ; absent sinon.
    pub modifiee_apres_transmission_le: Option<DateTime<Utc>>,
    pub lignes: Vec<Ligne>,
}

/// Ligne telle que saisie dans l'éditeur, avant validation.
#[derive(Debug, Clone, PartialEq)]
pub struct LigneSaisie {
    /// Absent pour une nouvelle ligne : l'identifiant est attribué à la sauvegarde.
    pub id: Option<LigneId>,
    pub date: NaiveDate,
    pub mission: MissionId,
    pub competences: Vec<CompetenceId>,
    pub duree_heures: f64,
    pub etapes: String,
    pub echanges_tuteur: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum ErreurFiche {
    #[error("Ajoutez au moins une ligne avant de terminer la fiche.")]
    TermineeSansLigne,
    #[error("On avance d'un statut à la fois (de {de:?} vers {vers:?} refusé).")]
    SautDeStatut { de: StatutFiche, vers: StatutFiche },
    #[error("Une fiche ne se crée pas au-delà de la semaine suivante.")]
    TropLoinDansLeFutur,
    #[error("{0}")]
    Validation(ErreurValidation),
}

impl Fiche {
    /// Une fiche naît en Brouillon (F-01).
    pub fn nouvelle(id: FicheId, lundi: Lundi, type_fiche: TypeFiche) -> Self {
        Self {
            id,
            lundi,
            type_fiche,
            statut: StatutFiche::Brouillon,
            terminee_le: None,
            transmise_le: None,
            validee_le: None,
            modifiee_apres_transmission_le: None,
            lignes: Vec::new(),
        }
    }

    pub fn modifiee_apres_transmission(&self) -> bool {
        self.modifiee_apres_transmission_le.is_some()
    }

    /// Somme des minutes des lignes (DUR6).
    pub fn total_minutes(&self) -> u32 {
        self.lignes
            .iter()
            .map(|l| u32::from(l.duree.en_minutes()))
            .sum()
    }

    /// Remplace le contenu ; le statut ne change pas (F-06). Renvoie `true` si
    /// la modification touche une fiche Transmise ou Validée (F-07).
    pub fn remplacer_lignes(&mut self, lignes: Vec<Ligne>, instant: DateTime<Utc>) -> bool {
        if lignes == self.lignes {
            return false;
        }
        self.lignes = lignes;
        if self.statut >= StatutFiche::Transmise {
            self.modifiee_apres_transmission_le = Some(instant);
            return true;
        }
        false
    }

    /// Avance d'un cran ou revient à n'importe quel statut inférieur (F-03).
    /// Chaque passage enregistre sa date ; revenir efface celles des statuts
    /// quittés (F-05). Renvoie `false` si le statut demandé est déjà le bon.
    pub fn changer_statut(
        &mut self,
        nouveau: StatutFiche,
        instant: DateTime<Utc>,
        date: Option<NaiveDate>,
    ) -> Result<bool, ErreurFiche> {
        let actuel = self.statut;
        if nouveau == actuel {
            return Ok(false);
        }
        if nouveau > actuel {
            if Some(nouveau) != actuel.suivant() {
                return Err(ErreurFiche::SautDeStatut {
                    de: actuel,
                    vers: nouveau,
                });
            }
            if nouveau == StatutFiche::Terminee && self.lignes.is_empty() {
                return Err(ErreurFiche::TermineeSansLigne);
            }
            let jour = date.unwrap_or_else(|| date_paris(instant));
            match nouveau {
                StatutFiche::Terminee => self.terminee_le = Some(instant),
                StatutFiche::Transmise => self.transmise_le = Some(jour),
                StatutFiche::Validee => self.validee_le = Some(jour),
                StatutFiche::Brouillon => {}
            }
        } else {
            if nouveau < StatutFiche::Validee {
                self.validee_le = None;
            }
            if nouveau < StatutFiche::Transmise {
                self.transmise_le = None;
                self.modifiee_apres_transmission_le = None;
            }
            if nouveau < StatutFiche::Terminee {
                self.terminee_le = None;
            }
        }
        self.statut = nouveau;
        Ok(true)
    }
}

/// Création : pas au-delà de la semaine suivant la semaine en cours (R-13).
pub fn verifier_creation(lundi: Lundi, instant: DateTime<Utc>) -> Result<(), ErreurFiche> {
    let limite = semaine_courante(instant).and_then(Lundi::suivant);
    match limite {
        Some(limite) if lundi > limite => Err(ErreurFiche::TropLoinDansLeFutur),
        _ => Ok(()),
    }
}

/// Référentiels connus au moment de la saisie.
pub struct Referentiels<'a> {
    pub missions: &'a [EntreeReferentiel],
    pub competences: &'a [EntreeReferentiel],
}

/// Valide les lignes saisies pour `fiche` (3.10). Une entrée archivée reste
/// acceptée si la fiche l'utilisait déjà (G-05, FIC11).
pub fn valider_lignes(
    fiche: &Fiche,
    saisies: &[LigneSaisie],
    referentiels: &Referentiels<'_>,
    mut nouvel_id: impl FnMut() -> LigneId,
) -> Result<Vec<Ligne>, ErreurValidation> {
    let mut erreurs = ErreurValidation::default();
    if saisies.len() > LIGNES_MAX {
        erreurs.ajouter(
            "lignes",
            format!("Une fiche compte {LIGNES_MAX} lignes au plus."),
        );
        return Err(erreurs);
    }
    let missions_utilisees: HashSet<MissionId> = fiche.lignes.iter().map(|l| l.mission).collect();
    let competences_utilisees: HashSet<CompetenceId> = fiche
        .lignes
        .iter()
        .flat_map(|l| l.competences.iter().copied())
        .collect();
    let ids_existants: HashSet<LigneId> = fiche.lignes.iter().map(|l| l.id).collect();
    let mut ids_vus = HashSet::new();

    let mut lignes = Vec::with_capacity(saisies.len());
    for (i, saisie) in saisies.iter().enumerate() {
        let champ = |nom: &str| format!("lignes[{i}].{nom}");
        if !fiche.lundi.contient(saisie.date) {
            erreurs.ajouter(
                champ("date"),
                format!(
                    "La date doit être dans la semaine du {} au {}.",
                    fiche.lundi.date().format("%d/%m/%Y"),
                    fiche.lundi.dimanche().format("%d/%m/%Y")
                ),
            );
        }
        match referentiels
            .missions
            .iter()
            .find(|m| m.id == saisie.mission)
        {
            None => erreurs.ajouter(champ("missionId"), "Choisissez une mission du référentiel."),
            Some(m) if m.archivee && !missions_utilisees.contains(&m.id) => {
                erreurs.ajouter(champ("missionId"), "Cette mission est archivée.");
            }
            Some(_) => {}
        }
        let distinctes: HashSet<CompetenceId> = saisie.competences.iter().copied().collect();
        if distinctes.len() != saisie.competences.len() {
            erreurs.ajouter(champ("competenceIds"), "Une compétence est en double.");
        }
        if saisie.competences.len() > COMPETENCES_MAX {
            erreurs.ajouter(
                champ("competenceIds"),
                format!("Une ligne porte {COMPETENCES_MAX} compétences au plus."),
            );
        }
        for c in &saisie.competences {
            match referentiels.competences.iter().find(|e| e.id == *c) {
                None => erreurs.ajouter(champ("competenceIds"), "Compétence inconnue."),
                Some(e) if e.archivee && !competences_utilisees.contains(&e.id) => {
                    erreurs.ajouter(
                        champ("competenceIds"),
                        format!("« {} » est archivée.", e.nom),
                    );
                }
                Some(_) => {}
            }
        }
        let duree = Duree::depuis_heures(saisie.duree_heures)
            .map_err(|e| erreurs.ajouter(champ("dureeHeures"), e.to_string()))
            .ok();
        let etapes = saisie.etapes.trim().to_owned();
        if etapes.chars().count() > TEXTE_MAX {
            erreurs.ajouter(champ("etapes"), format!("{TEXTE_MAX} caractères au plus."));
        }
        let echanges = saisie
            .echanges_tuteur
            .as_deref()
            .map(str::trim)
            .filter(|t| !t.is_empty())
            .map(str::to_owned);
        if echanges
            .as_ref()
            .is_some_and(|t| t.chars().count() > TEXTE_MAX)
        {
            erreurs.ajouter(
                champ("echangesTuteur"),
                format!("{TEXTE_MAX} caractères au plus."),
            );
        }
        // Un identifiant inconnu ou répété est remplacé : seules les lignes
        // déjà sauvegardées gardent le leur (18.1).
        let id = match saisie.id {
            Some(id) if ids_existants.contains(&id) && ids_vus.insert(id) => id,
            _ => nouvel_id(),
        };
        if let Some(duree) = duree {
            lignes.push(Ligne {
                id,
                date: saisie.date,
                mission: saisie.mission,
                competences: saisie.competences.clone(),
                duree,
                etapes,
                echanges_tuteur: echanges,
            });
        }
    }
    erreurs.resultat()?;
    Ok(lignes)
}

impl From<ErreurValidation> for ErreurFiche {
    fn from(erreur: ErreurValidation) -> Self {
        Self::Validation(erreur)
    }
}

impl From<ErreurChamp> for ErreurFiche {
    fn from(erreur: ErreurChamp) -> Self {
        Self::Validation(ErreurValidation::une(erreur))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ids::EntreeId;
    use pretty_assertions::assert_eq;
    use uuid::Uuid;

    fn d(a: i32, m: u32, j: u32) -> NaiveDate {
        NaiveDate::from_ymd_opt(a, m, j).unwrap()
    }

    fn utc(texte: &str) -> DateTime<Utc> {
        texte.parse().unwrap()
    }

    fn entree(n: u128, nom: &str, archivee: bool) -> EntreeReferentiel {
        EntreeReferentiel {
            id: EntreeId(Uuid::from_u128(n)),
            nom: nom.to_owned(),
            code: None,
            archivee,
        }
    }

    fn missions() -> Vec<EntreeReferentiel> {
        vec![
            entree(1, "Pipeline CI", false),
            entree(2, "Ancienne mission", true),
        ]
    }

    fn competences() -> Vec<EntreeReferentiel> {
        vec![
            entree(10, "Tester et valider", false),
            entree(11, "Documenter", false),
        ]
    }

    fn fiche_s38() -> Fiche {
        Fiche::nouvelle(
            FicheId(Uuid::from_u128(100)),
            Lundi::new(d(2026, 9, 14)).unwrap(),
            TypeFiche::Entreprise,
        )
    }

    fn saisie(date: NaiveDate, heures: f64) -> LigneSaisie {
        LigneSaisie {
            id: None,
            date,
            mission: EntreeId(Uuid::from_u128(1)),
            competences: vec![EntreeId(Uuid::from_u128(10))],
            duree_heures: heures,
            etapes: "  Écrire le workflow  ".to_owned(),
            echanges_tuteur: Some("   ".to_owned()),
        }
    }

    fn valider(fiche: &Fiche, saisies: &[LigneSaisie]) -> Result<Vec<Ligne>, ErreurValidation> {
        static COMPTEUR: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(1000);
        let missions = missions();
        let competences = competences();
        valider_lignes(
            fiche,
            saisies,
            &Referentiels {
                missions: &missions,
                competences: &competences,
            },
            || {
                let n = COMPTEUR.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
                LigneId(Uuid::from_u128(u128::from(n)))
            },
        )
    }

    fn fiche_avec_une_ligne() -> Fiche {
        let mut fiche = fiche_s38();
        let lignes = valider(&fiche, &[saisie(d(2026, 9, 15), 4.0)]).unwrap();
        fiche.remplacer_lignes(lignes, utc("2026-09-15T10:00:00Z"));
        fiche
    }

    #[test]
    fn une_fiche_nait_en_brouillon() {
        assert_eq!(fiche_s38().statut, StatutFiche::Brouillon);
    }

    #[test]
    fn ligne_valide_est_normalisee() {
        let lignes = valider(&fiche_s38(), &[saisie(d(2026, 9, 15), 3.5)]).unwrap();
        assert_eq!(lignes[0].etapes, "Écrire le workflow");
        assert_eq!(lignes[0].echanges_tuteur, None);
        assert_eq!(lignes[0].duree.en_minutes(), 210);
    }

    #[test]
    fn ligne_dont_la_date_sort_de_la_semaine_est_refusee() {
        let erreur = valider(&fiche_s38(), &[saisie(d(2026, 9, 21), 1.0)]).unwrap_err();
        assert_eq!(erreur.erreurs[0].champ, "lignes[0].date");
    }

    #[test]
    fn duree_hors_quart_d_heure_porte_le_champ_de_la_ligne() {
        let erreur = valider(
            &fiche_s38(),
            &[saisie(d(2026, 9, 15), 1.0), saisie(d(2026, 9, 16), 1.1)],
        )
        .unwrap_err();
        assert_eq!(erreur.erreurs[0].champ, "lignes[1].dureeHeures");
    }

    #[test]
    fn competences_en_double_sont_refusees() {
        let mut s = saisie(d(2026, 9, 15), 1.0);
        s.competences = vec![EntreeId(Uuid::from_u128(10)), EntreeId(Uuid::from_u128(10))];
        let erreur = valider(&fiche_s38(), &[s]).unwrap_err();
        assert_eq!(erreur.erreurs[0].champ, "lignes[0].competenceIds");
    }

    #[test]
    fn mission_archivee_d_une_ligne_existante_reste_acceptee_mais_pas_pour_une_nouvelle() {
        let mut s = saisie(d(2026, 9, 15), 1.0);
        s.mission = EntreeId(Uuid::from_u128(2));
        assert!(valider(&fiche_s38(), std::slice::from_ref(&s)).is_err());

        let mut fiche = fiche_s38();
        fiche.lignes = vec![Ligne {
            id: LigneId(Uuid::from_u128(500)),
            date: d(2026, 9, 15),
            mission: EntreeId(Uuid::from_u128(2)),
            competences: vec![],
            duree: Duree::minutes(60).unwrap(),
            etapes: String::new(),
            echanges_tuteur: None,
        }];
        assert!(valider(&fiche, &[s]).is_ok());
    }

    #[test]
    fn plus_de_200_lignes_est_refuse() {
        let saisies = vec![saisie(d(2026, 9, 15), 0.25); 201];
        let erreur = valider(&fiche_s38(), &saisies).unwrap_err();
        assert_eq!(erreur.erreurs[0].champ, "lignes");
    }

    #[test]
    fn champ_trop_long_est_refuse_avec_le_champ_concerne() {
        let mut s = saisie(d(2026, 9, 15), 1.0);
        s.etapes = "é".repeat(TEXTE_MAX + 1);
        let erreur = valider(&fiche_s38(), &[s]).unwrap_err();
        assert_eq!(erreur.erreurs[0].champ, "lignes[0].etapes");
    }

    #[test]
    fn une_ligne_existante_garde_son_identifiant() {
        let fiche = fiche_avec_une_ligne();
        let mut s = saisie(d(2026, 9, 15), 2.0);
        s.id = Some(fiche.lignes[0].id);
        let lignes = valider(&fiche, &[s, saisie(d(2026, 9, 16), 1.0)]).unwrap();
        assert_eq!(lignes[0].id, fiche.lignes[0].id);
        assert_ne!(lignes[1].id, fiche.lignes[0].id);
    }

    #[test]
    fn le_total_d_une_fiche_est_la_somme_des_minutes() {
        let mut fiche = fiche_s38();
        let lignes = valider(
            &fiche,
            &[saisie(d(2026, 9, 15), 4.0), saisie(d(2026, 9, 16), 2.5)],
        )
        .unwrap();
        fiche.remplacer_lignes(lignes, utc("2026-09-16T10:00:00Z"));
        assert_eq!(fiche.total_minutes(), 390);
    }

    #[test]
    fn terminee_sans_ligne_est_refusee_avec_un_message() {
        let mut fiche = fiche_s38();
        let erreur = fiche
            .changer_statut(StatutFiche::Terminee, utc("2026-09-20T10:00:00Z"), None)
            .unwrap_err();
        assert_eq!(erreur, ErreurFiche::TermineeSansLigne);
        assert_eq!(
            erreur.to_string(),
            "Ajoutez au moins une ligne avant de terminer la fiche."
        );
    }

    #[test]
    fn on_avance_d_un_cran_ou_on_revient_a_n_importe_quel_statut_inferieur() {
        let mut fiche = fiche_avec_une_ligne();
        let instant = utc("2026-09-20T10:00:00Z");
        assert!(matches!(
            fiche.changer_statut(StatutFiche::Transmise, instant, None),
            Err(ErreurFiche::SautDeStatut { .. })
        ));
        for statut in [
            StatutFiche::Terminee,
            StatutFiche::Transmise,
            StatutFiche::Validee,
        ] {
            assert_eq!(fiche.changer_statut(statut, instant, None), Ok(true));
        }
        assert_eq!(
            fiche.changer_statut(StatutFiche::Brouillon, instant, None),
            Ok(true)
        );
        assert_eq!(fiche.statut, StatutFiche::Brouillon);
    }

    #[test]
    fn chaque_passage_enregistre_sa_date_et_revenir_efface_celle_des_statuts_quittes() {
        let mut fiche = fiche_avec_une_ligne();
        let termine = utc("2026-09-20T19:10:00Z");
        fiche
            .changer_statut(StatutFiche::Terminee, termine, None)
            .unwrap();
        fiche
            .changer_statut(
                StatutFiche::Transmise,
                utc("2026-09-21T08:00:00Z"),
                Some(d(2026, 9, 21)),
            )
            .unwrap();
        fiche
            .changer_statut(StatutFiche::Validee, utc("2026-09-25T08:00:00Z"), None)
            .unwrap();
        assert_eq!(fiche.terminee_le, Some(termine));
        assert_eq!(fiche.transmise_le, Some(d(2026, 9, 21)));
        assert_eq!(fiche.validee_le, Some(d(2026, 9, 25)));

        fiche
            .changer_statut(StatutFiche::Terminee, utc("2026-09-26T08:00:00Z"), None)
            .unwrap();
        assert_eq!(fiche.terminee_le, Some(termine));
        assert_eq!(fiche.transmise_le, None);
        assert_eq!(fiche.validee_le, None);
    }

    #[test]
    fn la_transmission_est_une_date_seule_sans_destinataire() {
        let mut fiche = fiche_avec_une_ligne();
        let instant = utc("2026-10-11T22:30:00Z"); // lundi 12/10 à Paris
        fiche
            .changer_statut(StatutFiche::Terminee, instant, None)
            .unwrap();
        fiche
            .changer_statut(StatutFiche::Transmise, instant, None)
            .unwrap();
        assert_eq!(fiche.transmise_le, Some(d(2026, 10, 12)));
    }

    #[test]
    fn modifier_une_fiche_transmise_ne_change_pas_son_statut_et_pose_l_indicateur() {
        let mut fiche = fiche_avec_une_ligne();
        let instant = utc("2026-09-20T10:00:00Z");
        fiche
            .changer_statut(StatutFiche::Terminee, instant, None)
            .unwrap();
        fiche
            .changer_statut(StatutFiche::Transmise, instant, None)
            .unwrap();

        let lignes = valider(&fiche, &[saisie(d(2026, 9, 17), 2.0)]).unwrap();
        let modifie = utc("2026-10-11T12:40:00Z");
        assert!(fiche.remplacer_lignes(lignes, modifie));
        assert_eq!(fiche.statut, StatutFiche::Transmise);
        assert_eq!(fiche.modifiee_apres_transmission_le, Some(modifie));
    }

    #[test]
    fn une_sauvegarde_identique_ne_pose_pas_l_indicateur() {
        let mut fiche = fiche_avec_une_ligne();
        let instant = utc("2026-09-20T10:00:00Z");
        fiche
            .changer_statut(StatutFiche::Terminee, instant, None)
            .unwrap();
        fiche
            .changer_statut(StatutFiche::Transmise, instant, None)
            .unwrap();
        let memes = fiche.lignes.clone();
        assert!(!fiche.remplacer_lignes(memes, instant));
        assert!(!fiche.modifiee_apres_transmission());
    }

    #[test]
    fn l_indicateur_retombe_quand_la_fiche_redescend_sous_transmise() {
        let mut fiche = fiche_avec_une_ligne();
        let instant = utc("2026-09-20T10:00:00Z");
        fiche
            .changer_statut(StatutFiche::Terminee, instant, None)
            .unwrap();
        fiche
            .changer_statut(StatutFiche::Transmise, instant, None)
            .unwrap();
        fiche.modifiee_apres_transmission_le = Some(instant);
        fiche
            .changer_statut(StatutFiche::Terminee, instant, None)
            .unwrap();
        assert!(!fiche.modifiee_apres_transmission());
    }

    #[test]
    fn creation_au_dela_de_la_semaine_suivante_est_refusee() {
        let instant = utc("2026-10-11T10:00:00Z"); // semaine 41
        assert!(verifier_creation(Lundi::new(d(2026, 10, 12)).unwrap(), instant).is_ok());
        assert_eq!(
            verifier_creation(Lundi::new(d(2026, 10, 19)).unwrap(), instant),
            Err(ErreurFiche::TropLoinDansLeFutur)
        );
    }
}
