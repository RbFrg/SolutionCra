//! Référentiels : missions, compétences, motifs (3.7).

use serde::{Deserialize, Serialize};
use unicode_normalization::UnicodeNormalization;
use unicode_normalization::char::is_combining_mark;

use crate::erreur::{ErreurChamp, ErreurValidation};
use crate::ids::EntreeId;

pub const NOM_MAX: usize = 300;
pub const CODE_MAX: usize = 50;
pub const ENTREES_MAX: usize = 2000;
/// Code réservé du motif « Autre » : commentaire obligatoire (J-04).
pub const CODE_AUTRE: &str = "autre";

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum NomReferentiel {
    Missions,
    Competences,
    Motifs,
}

impl NomReferentiel {
    pub const TOUS: [NomReferentiel; 3] = [
        NomReferentiel::Missions,
        NomReferentiel::Competences,
        NomReferentiel::Motifs,
    ];

    pub fn code(self) -> &'static str {
        match self {
            NomReferentiel::Missions => "missions",
            NomReferentiel::Competences => "competences",
            NomReferentiel::Motifs => "motifs",
        }
    }

    pub fn depuis_code(code: &str) -> Option<Self> {
        Self::TOUS.into_iter().find(|n| n.code() == code)
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct EntreeReferentiel {
    pub id: EntreeId,
    pub nom: String,
    pub code: Option<String>,
    pub archivee: bool,
}

/// Clé d'unicité : sans casse ni accents, espaces de bordure retirés (REF1).
pub fn cle_de_nom(nom: &str) -> String {
    nom.trim()
        .nfd()
        .filter(|c| !is_combining_mark(*c))
        .collect::<String>()
        .to_lowercase()
}

/// Nom normalisé et validé (REF2) : non vide, 300 caractères au plus.
pub fn normaliser_nom(nom: &str) -> Result<String, ErreurChamp> {
    let nom = nom.trim();
    if nom.is_empty() {
        return Err(ErreurChamp::new("nom", "Le nom est obligatoire."));
    }
    if nom.chars().count() > NOM_MAX {
        return Err(ErreurChamp::new(
            "nom",
            format!("Le nom ne doit pas dépasser {NOM_MAX} caractères."),
        ));
    }
    Ok(nom.to_owned())
}

/// Avertissement sans blocage : une compétence se formule « être capable de
/// <verbe à l'infinitif> » (REF6). Heuristique sur la terminaison du premier mot.
pub fn avertissement_competence(nom: &str) -> Option<String> {
    let premier = nom.split_whitespace().next()?.to_lowercase();
    let infinitif = ["er", "ir", "re", "oir"]
        .iter()
        .any(|fin| premier.ends_with(fin));
    (!infinitif).then(|| {
        "Une compétence commence par un verbe à l'infinitif (« Concevoir », « Tester »…)."
            .to_owned()
    })
}

/// Valide une nouvelle entrée ou un renommage contre la liste existante.
pub fn valider_entree(
    existantes: &[EntreeReferentiel],
    nom_referentiel: NomReferentiel,
    id: EntreeId,
    nom: &str,
    code: Option<&str>,
) -> Result<(String, Option<String>), ErreurValidation> {
    let mut erreurs = ErreurValidation::default();
    let nom = match normaliser_nom(nom) {
        Ok(n) => Some(n),
        Err(e) => {
            erreurs.erreurs.push(e);
            None
        }
    };
    if let Some(n) = &nom {
        let cle = cle_de_nom(n);
        if existantes
            .iter()
            .any(|e| e.id != id && cle_de_nom(&e.nom) == cle)
        {
            erreurs.ajouter("nom", format!("« {n} » existe déjà dans ce référentiel."));
        }
    }
    let code = code
        .map(str::trim)
        .filter(|c| !c.is_empty())
        .map(str::to_owned);
    if let Some(c) = &code {
        if c.chars().count() > CODE_MAX {
            erreurs.ajouter(
                "code",
                format!("Le code ne doit pas dépasser {CODE_MAX} caractères."),
            );
        }
        if nom_referentiel == NomReferentiel::Motifs
            && existantes
                .iter()
                .any(|e| e.id != id && e.code.as_deref() == Some(c))
        {
            erreurs.ajouter("code", "Ce code est déjà utilisé.");
        }
    }
    let ancienne = existantes.iter().find(|e| e.id == id);
    if nom_referentiel == NomReferentiel::Motifs
        && ancienne.is_some_and(|e| e.code.as_deref() == Some(CODE_AUTRE))
        && code.as_deref() != Some(CODE_AUTRE)
    {
        erreurs.ajouter("code", "Le motif « Autre » garde son code réservé.");
    }
    if ancienne.is_none() && existantes.len() >= ENTREES_MAX {
        erreurs.ajouter(
            "nom",
            format!("Un référentiel compte {ENTREES_MAX} entrées au plus."),
        );
    }
    erreurs.resultat()?;
    Ok((nom.unwrap_or_default(), code))
}

/// Le motif « Autre » ne s'archive pas (JUS10).
pub fn peut_archiver(entree: &EntreeReferentiel, nom_referentiel: NomReferentiel) -> bool {
    !(nom_referentiel == NomReferentiel::Motifs && entree.code.as_deref() == Some(CODE_AUTRE))
}

/// Motifs amorcés au premier démarrage (18.2) : (nom, code).
pub const MOTIFS_PAR_DEFAUT: [(&str, Option<&str>); 5] = [
    ("Congés", None),
    ("Férié", None),
    ("Maladie", None),
    ("Déplacement", None),
    ("Autre", Some(CODE_AUTRE)),
];

#[cfg(test)]
mod tests {
    use super::*;
    use uuid::Uuid;

    fn entree(n: u128, nom: &str, code: Option<&str>) -> EntreeReferentiel {
        EntreeReferentiel {
            id: EntreeId(Uuid::from_u128(n)),
            nom: nom.to_owned(),
            code: code.map(str::to_owned),
            archivee: false,
        }
    }

    #[test]
    fn nom_unique_insensible_a_la_casse_et_aux_accents() {
        let existantes = vec![entree(1, "Déployer et exploiter", None)];
        let erreur = valider_entree(
            &existantes,
            NomReferentiel::Competences,
            EntreeId(Uuid::from_u128(2)),
            "  deployer ET exploiter ",
            None,
        )
        .unwrap_err();
        assert_eq!(erreur.erreurs[0].champ, "nom");
    }

    #[test]
    fn renommer_une_entree_sans_changer_sa_cle_est_accepte() {
        let existantes = vec![entree(1, "documenter", None)];
        let (nom, _) = valider_entree(
            &existantes,
            NomReferentiel::Competences,
            EntreeId(Uuid::from_u128(1)),
            "Documenter",
            None,
        )
        .unwrap();
        assert_eq!(nom, "Documenter");
    }

    #[test]
    fn nom_vide_est_refuse() {
        let erreur = valider_entree(
            &[],
            NomReferentiel::Missions,
            EntreeId(Uuid::from_u128(1)),
            "   ",
            None,
        )
        .unwrap_err();
        assert_eq!(erreur.erreurs[0].message, "Le nom est obligatoire.");
    }

    #[test]
    fn competence_sans_verbe_a_l_infinitif_est_signalee_sans_bloquer() {
        assert!(avertissement_competence("Concevoir une architecture").is_none());
        assert!(avertissement_competence("Tester et valider").is_none());
        assert!(avertissement_competence("Rédaction de documents").is_some());
    }

    #[test]
    fn le_motif_autre_ne_peut_etre_ni_archive_ni_changer_de_code() {
        let autre = entree(5, "Autre", Some(CODE_AUTRE));
        assert!(!peut_archiver(&autre, NomReferentiel::Motifs));
        let erreur = valider_entree(
            std::slice::from_ref(&autre),
            NomReferentiel::Motifs,
            autre.id,
            "Autre motif",
            Some("divers"),
        )
        .unwrap_err();
        assert_eq!(erreur.erreurs[0].champ, "code");
    }
}
