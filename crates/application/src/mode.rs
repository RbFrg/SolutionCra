//! Mode d'exécution (4.5) : choisi au démarrage par `APP_MODE`, jamais modifiable
//! depuis l'interface ni par l'API (M-01).

use chrono::{DateTime, NaiveDate, TimeZone, Utc};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Mode {
    Production,
    Demo,
}

impl Mode {
    pub fn date_figee(self) -> Option<NaiveDate> {
        match self {
            Mode::Production => None,
            Mode::Demo => Some(instant_demo().date_naive()),
        }
    }
}

/// Date figée de la démo : dimanche 11/10/2026 à 18 h, heure de Paris. Le
/// skill proposait midi (7.7), mais le journal du jeu (20.5) contient des
/// entrées à 14:40 et 14:52 ce jour-là : l'horloge doit leur être postérieure.
/// La semaine 41 reste « À faire » jusqu'à minuit.
pub fn instant_demo() -> DateTime<Utc> {
    Utc.with_ymd_and_hms(2026, 10, 11, 16, 0, 0)
        .single()
        .unwrap_or(DateTime::<Utc>::MIN_UTC)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn la_date_figee_de_la_demo_est_le_11_10_2026() {
        assert_eq!(
            Mode::Demo.date_figee(),
            NaiveDate::from_ymd_opt(2026, 10, 11)
        );
        assert_eq!(Mode::Production.date_figee(), None);
    }
}
