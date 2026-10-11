//! Durée d'une ligne : minutes entières, au quart d'heure, de 15 min à 12 h (3.8).

use std::fmt;

use serde::{Deserialize, Serialize};

pub const MINUTES_MIN: u16 = 15;
pub const MINUTES_MAX: u16 = 720;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(try_from = "u16", into = "u16")]
pub struct Duree(u16);

#[derive(Debug, Clone, Copy, PartialEq, Eq, thiserror::Error)]
pub enum ErreurDuree {
    #[error("La durée doit être un multiple de 15 minutes.")]
    PasUnQuartDHeure,
    #[error("La durée doit être comprise entre 0,25 h et 12 h.")]
    HorsBornes,
}

impl Duree {
    pub const QUART_D_HEURE: Duree = Duree(MINUTES_MIN);

    pub fn minutes(minutes: u16) -> Result<Self, ErreurDuree> {
        if !(MINUTES_MIN..=MINUTES_MAX).contains(&minutes) {
            return Err(ErreurDuree::HorsBornes);
        }
        if !minutes.is_multiple_of(15) {
            return Err(ErreurDuree::PasUnQuartDHeure);
        }
        Ok(Self(minutes))
    }

    /// Conversion des heures décimales de l'API : arrondi à la minute, puis
    /// refus de tout ce qui n'est pas un quart d'heure (18.3).
    pub fn depuis_heures(heures: f64) -> Result<Self, ErreurDuree> {
        if !heures.is_finite() {
            return Err(ErreurDuree::HorsBornes);
        }
        let minutes = (heures * 60.0).round();
        if !(f64::from(MINUTES_MIN)..=f64::from(MINUTES_MAX)).contains(&minutes) {
            return Err(ErreurDuree::HorsBornes);
        }
        // Borne vérifiée juste au-dessus : la conversion est exacte.
        #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
        Self::minutes(minutes as u16)
    }

    pub fn en_minutes(self) -> u16 {
        self.0
    }

    pub fn en_heures(self) -> f64 {
        f64::from(self.0) / 60.0
    }
}

impl TryFrom<u16> for Duree {
    type Error = ErreurDuree;
    fn try_from(minutes: u16) -> Result<Self, Self::Error> {
        Self::minutes(minutes)
    }
}

impl From<Duree> for u16 {
    fn from(duree: Duree) -> Self {
        duree.0
    }
}

/// Heures décimales en français : `8,5 h`.
pub fn formater_heures(minutes: u32) -> String {
    let heures = f64::from(minutes) / 60.0;
    let texte = if minutes.is_multiple_of(60) {
        format!("{}", minutes / 60)
    } else {
        format!("{heures}")
    };
    format!("{} h", texte.replace('.', ","))
}

impl fmt::Display for Duree {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&formater_heures(u32::from(self.0)))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use proptest::prelude::*;
    use rstest::rstest;

    #[rstest]
    #[case(15)]
    #[case(210)]
    #[case(720)]
    fn duree_multiple_de_15_minutes_est_acceptee(#[case] minutes: u16) {
        assert_eq!(Duree::minutes(minutes).unwrap().en_minutes(), minutes);
    }

    #[test]
    fn duree_hors_quart_d_heure_est_refusee_avec_le_champ_concerne() {
        assert_eq!(
            Duree::depuis_heures(1.1),
            Err(ErreurDuree::PasUnQuartDHeure)
        );
        assert_eq!(Duree::minutes(20), Err(ErreurDuree::PasUnQuartDHeure));
    }

    #[test]
    fn les_heures_decimales_se_convertissent_en_minutes_sans_derive() {
        assert_eq!(Duree::depuis_heures(0.25).unwrap().en_minutes(), 15);
        assert_eq!(Duree::depuis_heures(3.5).unwrap().en_minutes(), 210);
        assert_eq!(Duree::depuis_heures(7.75).unwrap().en_minutes(), 465);
    }

    #[rstest]
    #[case(0.0)]
    #[case(12.25)]
    #[case(-1.0)]
    #[case(f64::NAN)]
    fn duree_hors_des_bornes_15_minutes_12_heures_est_refusee(#[case] heures: f64) {
        assert_eq!(Duree::depuis_heures(heures), Err(ErreurDuree::HorsBornes));
    }

    #[test]
    fn les_heures_s_affichent_avec_une_virgule() {
        assert_eq!(formater_heures(510), "8,5 h");
        assert_eq!(formater_heures(600), "10 h");
        assert_eq!(formater_heures(15), "0,25 h");
    }

    proptest! {
        #[test]
        fn minutes_vers_heures_vers_minutes_est_l_identite(quarts in 1u16..=48) {
            let duree = Duree::minutes(quarts * 15).unwrap();
            prop_assert_eq!(Duree::depuis_heures(duree.en_heures()).unwrap(), duree);
        }
    }
}
