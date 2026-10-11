//! Port `Clock` (4.4) : le seul accès à « maintenant ». `domain` et les cas
//! d'usage reçoivent l'instant en paramètre, jamais l'heure du système.

use std::sync::Mutex;

use chrono::{DateTime, Duration, Utc};

pub trait Clock: Send + Sync {
    fn now(&self) -> DateTime<Utc>;
}

/// Horloge de production.
#[derive(Debug, Default, Clone, Copy)]
pub struct SystemClock;

impl Clock for SystemClock {
    fn now(&self) -> DateTime<Utc> {
        Utc::now()
    }
}

/// Horloge figée (tests, mode démo). Elle s'avance, elle n'attend jamais (7.7).
#[derive(Debug)]
pub struct FixedClock(Mutex<DateTime<Utc>>);

impl FixedClock {
    pub fn new(instant: DateTime<Utc>) -> Self {
        Self(Mutex::new(instant))
    }

    pub fn avancer(&self, duree: Duration) {
        let mut instant = self.0.lock().unwrap_or_else(|e| e.into_inner());
        *instant += duree;
    }

    pub fn regler(&self, nouvel_instant: DateTime<Utc>) {
        let mut instant = self.0.lock().unwrap_or_else(|e| e.into_inner());
        *instant = nouvel_instant;
    }
}

impl Clock for FixedClock {
    fn now(&self) -> DateTime<Utc> {
        *self.0.lock().unwrap_or_else(|e| e.into_inner())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::TimeZone;

    #[test]
    fn horloge_figee_ne_bouge_que_si_on_l_avance() {
        let depart = Utc.with_ymd_and_hms(2026, 10, 11, 10, 0, 0).unwrap();
        let horloge = FixedClock::new(depart);

        assert_eq!(horloge.now(), depart);
        horloge.avancer(Duration::hours(2));
        assert_eq!(horloge.now(), depart + Duration::hours(2));
    }
}
