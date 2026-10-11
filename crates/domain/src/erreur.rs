//! Erreurs de validation champ par champ (4.1 : `errors[].field`).

use std::fmt;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ErreurChamp {
    /// Chemin du champ, par exemple `lignes[2].dureeHeures`.
    pub champ: String,
    pub message: String,
}

impl ErreurChamp {
    pub fn new(champ: impl Into<String>, message: impl Into<String>) -> Self {
        Self {
            champ: champ.into(),
            message: message.into(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct ErreurValidation {
    pub erreurs: Vec<ErreurChamp>,
}

impl ErreurValidation {
    pub fn une(erreur: ErreurChamp) -> Self {
        Self {
            erreurs: vec![erreur],
        }
    }

    pub fn ajouter(&mut self, champ: impl Into<String>, message: impl Into<String>) {
        self.erreurs.push(ErreurChamp::new(champ, message));
    }

    pub fn est_vide(&self) -> bool {
        self.erreurs.is_empty()
    }

    /// `Ok(())` s'il n'y a aucune erreur.
    pub fn resultat(self) -> Result<(), ErreurValidation> {
        if self.est_vide() { Ok(()) } else { Err(self) }
    }
}

impl fmt::Display for ErreurValidation {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let textes: Vec<String> = self
            .erreurs
            .iter()
            .map(|e| format!("{} : {}", e.champ, e.message))
            .collect();
        write!(f, "{}", textes.join(" ; "))
    }
}

impl std::error::Error for ErreurValidation {}
