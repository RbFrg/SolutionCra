//! Semaine (lundi au dimanche), année scolaire et heure de Paris (3.1).

use std::fmt;

use chrono::{DateTime, Datelike, Duration, NaiveDate, NaiveTime, TimeZone, Utc, Weekday};
use chrono_tz::Europe::Paris;
use serde::{Deserialize, Serialize};

const fn date(annee: i32, mois: u32, jour: u32) -> NaiveDate {
    match NaiveDate::from_ymd_opt(annee, mois, jour) {
        Some(d) => d,
        None => panic!("date constante invalide"),
    }
}

/// Premier lundi suivi : semaine du 31/08/2026 (S36).
pub const PREMIER_LUNDI: Lundi = Lundi(date(2026, 8, 31));
/// Dernier lundi suivi : semaine du 28/08/2028, qui finit le 03/09/2028.
pub const DERNIER_LUNDI: Lundi = Lundi(date(2028, 8, 28));
/// Premier lundi de l'année scolaire 2027-2028.
const PREMIER_LUNDI_2027: NaiveDate = date(2027, 8, 30);

#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum ErreurSemaine {
    #[error("le {0} n'est pas un lundi")]
    PasUnLundi(NaiveDate),
    #[error("la semaine du {0} est hors du suivi (31/08/2026 au 28/08/2028)")]
    HorsSuivi(NaiveDate),
    #[error("l'année scolaire {0} n'est pas suivie (2026 ou 2027)")]
    AnneeInconnue(i32),
}

/// Date du lundi d'une semaine suivie : clé de la semaine (plan, fiche,
/// justification). Invariant : un lundi, dans la plage du suivi.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(try_from = "NaiveDate", into = "NaiveDate")]
pub struct Lundi(NaiveDate);

impl Lundi {
    pub fn new(date: NaiveDate) -> Result<Self, ErreurSemaine> {
        if date.weekday() != Weekday::Mon {
            return Err(ErreurSemaine::PasUnLundi(date));
        }
        if date < PREMIER_LUNDI.0 || date > DERNIER_LUNDI.0 {
            return Err(ErreurSemaine::HorsSuivi(date));
        }
        Ok(Self(date))
    }

    /// Lundi de la semaine qui contient `date`.
    pub fn contenant(date: NaiveDate) -> Result<Self, ErreurSemaine> {
        let decalage = i64::from(date.weekday().num_days_from_monday());
        Self::new(date - Duration::days(decalage))
    }

    pub fn date(self) -> NaiveDate {
        self.0
    }

    pub fn dimanche(self) -> NaiveDate {
        self.0 + Duration::days(6)
    }

    /// Numéro de semaine ISO 8601 (le 05/10/2026 ouvre la semaine 41).
    pub fn numero(self) -> u32 {
        self.0.iso_week().week()
    }

    pub fn contient(self, jour: NaiveDate) -> bool {
        jour >= self.0 && jour <= self.dimanche()
    }

    pub fn suivant(self) -> Option<Self> {
        Self::new(self.0 + Duration::days(7)).ok()
    }

    pub fn precedent(self) -> Option<Self> {
        Self::new(self.0 - Duration::days(7)).ok()
    }

    /// Lundi 00h00, heure de Paris : début de la semaine.
    pub fn debut(self) -> DateTime<Utc> {
        minuit_paris(self.0)
    }

    /// Lundi suivant 00h00, heure de Paris : premier instant « en retard ».
    pub fn fin(self) -> DateTime<Utc> {
        minuit_paris(self.0 + Duration::days(7))
    }

    /// Échéance affichée : dimanche 23h59m59, heure de Paris.
    pub fn echeance(self) -> DateTime<Utc> {
        self.fin() - Duration::seconds(1)
    }

    pub fn annee_scolaire(self) -> AnneeScolaire {
        if self.0 < PREMIER_LUNDI_2027 {
            AnneeScolaire(2026)
        } else {
            AnneeScolaire(2027)
        }
    }

    /// Toutes les semaines suivies, dans l'ordre.
    pub fn toutes() -> impl Iterator<Item = Lundi> {
        std::iter::successors(Some(PREMIER_LUNDI), |l| l.suivant())
    }
}

impl From<Lundi> for NaiveDate {
    fn from(lundi: Lundi) -> Self {
        lundi.0
    }
}

impl TryFrom<NaiveDate> for Lundi {
    type Error = ErreurSemaine;
    fn try_from(date: NaiveDate) -> Result<Self, Self::Error> {
        Self::new(date)
    }
}

impl fmt::Display for Lundi {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0.format("%Y-%m-%d"))
    }
}

/// Année scolaire suivie, désignée par son année de début (2026 ou 2027).
/// Elle commence au lundi de la semaine qui contient le 1er septembre.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub struct AnneeScolaire(i32);

impl AnneeScolaire {
    pub fn new(annee: i32) -> Result<Self, ErreurSemaine> {
        match annee {
            2026 | 2027 => Ok(Self(annee)),
            autre => Err(ErreurSemaine::AnneeInconnue(autre)),
        }
    }

    pub fn valeur(self) -> i32 {
        self.0
    }

    pub fn semaines(self) -> impl Iterator<Item = Lundi> {
        Lundi::toutes().filter(move |l| l.annee_scolaire() == self)
    }

    /// Année scolaire de la date donnée, ramenée dans la plage suivie.
    pub fn de(date: NaiveDate) -> Self {
        match Lundi::contenant(date) {
            Ok(lundi) => lundi.annee_scolaire(),
            Err(_) if date < PREMIER_LUNDI.0 => Self(2026),
            Err(_) => Self(2027),
        }
    }
}

fn minuit_paris(jour: NaiveDate) -> DateTime<Utc> {
    let local = jour.and_time(NaiveTime::MIN);
    // Minuit existe toujours à Paris : les changements d'heure ont lieu à 2 h ou 3 h.
    match Paris.from_local_datetime(&local).earliest() {
        Some(instant) => instant.with_timezone(&Utc),
        None => Utc.from_utc_datetime(&local),
    }
}

/// Date de Paris d'un instant UTC (« aujourd'hui »).
pub fn date_paris(instant: DateTime<Utc>) -> NaiveDate {
    instant.with_timezone(&Paris).date_naive()
}

/// Lundi de la semaine en cours à Paris, s'il est dans la plage suivie.
pub fn semaine_courante(instant: DateTime<Utc>) -> Option<Lundi> {
    Lundi::contenant(date_paris(instant)).ok()
}

#[cfg(test)]
mod tests {
    use super::*;
    use pretty_assertions::assert_eq;

    fn d(a: i32, m: u32, j: u32) -> NaiveDate {
        NaiveDate::from_ymd_opt(a, m, j).unwrap()
    }

    fn utc(texte: &str) -> DateTime<Utc> {
        texte.parse().unwrap()
    }

    #[test]
    fn lundi_refuse_une_date_qui_n_est_pas_un_lundi() {
        assert_eq!(
            Lundi::new(d(2026, 10, 6)),
            Err(ErreurSemaine::PasUnLundi(d(2026, 10, 6)))
        );
        assert!(Lundi::new(d(2026, 10, 5)).is_ok());
    }

    #[test]
    fn semaine_hors_du_suivi_est_refusee() {
        assert_eq!(
            Lundi::new(d(2026, 8, 24)),
            Err(ErreurSemaine::HorsSuivi(d(2026, 8, 24)))
        );
        assert_eq!(
            Lundi::new(d(2028, 9, 4)),
            Err(ErreurSemaine::HorsSuivi(d(2028, 9, 4)))
        );
        assert!(Lundi::new(d(2026, 8, 31)).is_ok());
        assert!(Lundi::new(d(2028, 8, 28)).is_ok());
    }

    #[test]
    fn numero_iso_du_5_octobre_2026_est_41() {
        assert_eq!(Lundi::new(d(2026, 10, 5)).unwrap().numero(), 41);
        assert_eq!(PREMIER_LUNDI.numero(), 36);
    }

    #[test]
    fn le_suivi_compte_105_semaines_du_31_08_2026_au_28_08_2028() {
        let semaines: Vec<Lundi> = Lundi::toutes().collect();
        assert_eq!(semaines.len(), 105);
        assert_eq!(semaines.first(), Some(&PREMIER_LUNDI));
        assert_eq!(semaines.last(), Some(&DERNIER_LUNDI));
        assert_eq!(DERNIER_LUNDI.dimanche(), d(2028, 9, 3));
    }

    #[test]
    fn l_annee_2026_compte_52_semaines_et_l_annee_2027_53() {
        let a2026: Vec<Lundi> = AnneeScolaire::new(2026).unwrap().semaines().collect();
        let a2027: Vec<Lundi> = AnneeScolaire::new(2027).unwrap().semaines().collect();
        assert_eq!(a2026.len(), 52);
        assert_eq!(a2027.len(), 53);
        assert_eq!(a2026.last().unwrap().date(), d(2027, 8, 23));
        assert_eq!(a2027.first().unwrap().date(), d(2027, 8, 30));
        assert!(AnneeScolaire::new(2028).is_err());
    }

    #[test]
    fn un_instant_utc_du_dimanche_soir_est_deja_lundi_a_paris() {
        let instant = utc("2026-10-11T22:30:00Z");
        assert_eq!(date_paris(instant), d(2026, 10, 12));
        assert_eq!(semaine_courante(instant).unwrap().date(), d(2026, 10, 12));
    }

    #[test]
    fn debut_et_fin_de_semaine_en_heure_de_paris() {
        let s41 = Lundi::new(d(2026, 10, 5)).unwrap();
        assert_eq!(s41.debut(), utc("2026-10-04T22:00:00Z"));
        assert_eq!(s41.fin(), utc("2026-10-11T22:00:00Z"));
        assert_eq!(s41.echeance(), utc("2026-10-11T21:59:59Z"));
    }

    #[test]
    fn la_semaine_du_passage_a_l_heure_d_hiver_dure_169_heures() {
        let s = Lundi::new(d(2026, 10, 19)).unwrap();
        assert_eq!(s.fin() - s.debut(), Duration::hours(169));
        assert_eq!(s.fin(), utc("2026-10-25T23:00:00Z"));
    }

    #[test]
    fn la_semaine_du_passage_a_l_heure_d_ete_dure_167_heures() {
        let s = Lundi::new(d(2027, 3, 22)).unwrap();
        assert_eq!(s.fin() - s.debut(), Duration::hours(167));
        assert_eq!(s.fin(), utc("2027-03-28T22:00:00Z"));
    }

    #[test]
    fn lundi_contenant_ramene_au_lundi_de_la_semaine() {
        assert_eq!(
            Lundi::contenant(d(2026, 10, 11)).unwrap().date(),
            d(2026, 10, 5)
        );
    }

    #[test]
    fn lundi_se_serialise_en_date_iso() {
        let lundi = Lundi::new(d(2026, 10, 5)).unwrap();
        assert_eq!(serde_json::to_string(&lundi).unwrap(), "\"2026-10-05\"");
        assert!(serde_json::from_str::<Lundi>("\"2026-10-06\"").is_err());
    }
}
