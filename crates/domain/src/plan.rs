//! Plan d'alternance saisi semaine par semaine (3.2). Pas de semaine mixte.

use serde::{Deserialize, Serialize};

use crate::erreur::{ErreurChamp, ErreurValidation};
use crate::semaine::Lundi;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TypeFiche {
    Entreprise,
    Formation,
}

impl TypeFiche {
    pub const TOUS: [TypeFiche; 2] = [TypeFiche::Entreprise, TypeFiche::Formation];

    pub fn code(self) -> &'static str {
        match self {
            TypeFiche::Entreprise => "entreprise",
            TypeFiche::Formation => "formation",
        }
    }

    pub fn libelle(self) -> &'static str {
        match self {
            TypeFiche::Entreprise => "entreprise",
            TypeFiche::Formation => "formation",
        }
    }

    pub fn depuis_code(code: &str) -> Option<Self> {
        match code {
            "entreprise" => Some(TypeFiche::Entreprise),
            "formation" => Some(TypeFiche::Formation),
            _ => None,
        }
    }
}

/// Type attendu d'une semaine. « Les deux » n'existe pas (décision du 11/10/2026).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TypeAttendu {
    Entreprise,
    Formation,
    Aucune,
}

impl TypeAttendu {
    pub fn type_fiche(self) -> Option<TypeFiche> {
        match self {
            TypeAttendu::Entreprise => Some(TypeFiche::Entreprise),
            TypeAttendu::Formation => Some(TypeFiche::Formation),
            TypeAttendu::Aucune => None,
        }
    }

    pub fn code(self) -> &'static str {
        match self {
            TypeAttendu::Entreprise => "entreprise",
            TypeAttendu::Formation => "formation",
            TypeAttendu::Aucune => "aucune",
        }
    }

    pub fn depuis_code(code: &str) -> Option<Self> {
        match code {
            "entreprise" => Some(TypeAttendu::Entreprise),
            "formation" => Some(TypeAttendu::Formation),
            "aucune" => Some(TypeAttendu::Aucune),
            _ => None,
        }
    }
}

/// Une semaine sans ligne de plan vaut Entreprise (P-04).
pub const TYPE_PAR_DEFAUT: TypeAttendu = TypeAttendu::Entreprise;

pub const LIBELLE_MAX: usize = 100;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SemainePlan {
    pub lundi: Lundi,
    pub type_attendu: TypeAttendu,
    pub libelle: Option<String>,
}

impl SemainePlan {
    pub fn par_defaut(lundi: Lundi) -> Self {
        Self {
            lundi,
            type_attendu: TYPE_PAR_DEFAUT,
            libelle: None,
        }
    }
}

/// Type attendu pour `lundi` d'après les lignes du plan.
pub fn type_attendu(plan: &[SemainePlan], lundi: Lundi) -> TypeAttendu {
    plan.iter()
        .find(|s| s.lundi == lundi)
        .map_or(TYPE_PAR_DEFAUT, |s| s.type_attendu)
}

/// Libellé normalisé : espaces de bordure retirés, vide devient absent, 100 caractères au plus (P-03).
pub fn normaliser_libelle(libelle: Option<&str>) -> Result<Option<String>, ErreurValidation> {
    let Some(texte) = libelle.map(str::trim).filter(|t| !t.is_empty()) else {
        return Ok(None);
    };
    if texte.chars().count() > LIBELLE_MAX {
        return Err(ErreurValidation::une(ErreurChamp::new(
            "libelle",
            format!("Le libellé ne doit pas dépasser {LIBELLE_MAX} caractères."),
        )));
    }
    Ok(Some(texte.to_owned()))
}

/// Applique un type à une plage de semaines : une ligne par semaine (P-05).
pub fn appliquer_plage(
    du: Lundi,
    au: Lundi,
    type_attendu: TypeAttendu,
    libelle: Option<&str>,
) -> Result<Vec<SemainePlan>, ErreurValidation> {
    if au < du {
        return Err(ErreurValidation::une(ErreurChamp::new(
            "au",
            "La fin de la plage précède son début.",
        )));
    }
    let libelle = normaliser_libelle(libelle)?;
    Ok(Lundi::toutes()
        .filter(|l| *l >= du && *l <= au)
        .map(|lundi| SemainePlan {
            lundi,
            type_attendu,
            libelle: libelle.clone(),
        })
        .collect())
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::NaiveDate;

    fn lundi(a: i32, m: u32, j: u32) -> Lundi {
        Lundi::new(NaiveDate::from_ymd_opt(a, m, j).unwrap()).unwrap()
    }

    #[test]
    fn le_type_les_deux_est_refuse_a_la_deserialisation() {
        assert!(serde_json::from_str::<TypeAttendu>("\"les_deux\"").is_err());
        assert_eq!(
            serde_json::from_str::<TypeAttendu>("\"aucune\"").unwrap(),
            TypeAttendu::Aucune
        );
    }

    #[test]
    fn semaine_sans_ligne_de_plan_vaut_entreprise() {
        assert_eq!(
            type_attendu(&[], lundi(2026, 10, 5)),
            TypeAttendu::Entreprise
        );
    }

    #[test]
    fn libelle_est_conserve_et_limite_a_100_caracteres() {
        assert_eq!(
            normaliser_libelle(Some("  Vacances scolaires ")).unwrap(),
            Some("Vacances scolaires".to_owned())
        );
        assert_eq!(normaliser_libelle(Some("   ")).unwrap(), None);
        assert!(normaliser_libelle(Some(&"é".repeat(100))).is_ok());
        let erreur = normaliser_libelle(Some(&"é".repeat(101))).unwrap_err();
        assert_eq!(erreur.erreurs[0].champ, "libelle");
    }

    #[test]
    fn appliquer_une_plage_produit_une_ligne_par_semaine() {
        let lignes = appliquer_plage(
            lundi(2026, 10, 12),
            lundi(2026, 10, 26),
            TypeAttendu::Formation,
            Some("Bloc école"),
        )
        .unwrap();
        assert_eq!(lignes.len(), 3);
        assert!(
            lignes
                .iter()
                .all(|l| l.type_attendu == TypeAttendu::Formation)
        );
        assert_eq!(lignes[2].lundi, lundi(2026, 10, 26));
        assert_eq!(lignes[0].libelle.as_deref(), Some("Bloc école"));
    }

    #[test]
    fn plage_inversee_est_refusee() {
        let erreur = appliquer_plage(
            lundi(2026, 10, 26),
            lundi(2026, 10, 12),
            TypeAttendu::Formation,
            None,
        )
        .unwrap_err();
        assert_eq!(erreur.erreurs[0].champ, "au");
    }
}
