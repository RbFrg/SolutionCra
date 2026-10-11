//! Identifiants : newtypes sur des UUID v7 (18.1), générés hors du domaine.

use std::fmt;

use serde::{Deserialize, Serialize};
use uuid::Uuid;

macro_rules! identifiant {
    ($($nom:ident),* $(,)?) => {$(
        #[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
        #[serde(transparent)]
        pub struct $nom(pub Uuid);

        impl $nom {
            pub fn uuid(self) -> Uuid {
                self.0
            }
        }

        impl From<Uuid> for $nom {
            fn from(uuid: Uuid) -> Self {
                Self(uuid)
            }
        }

        impl fmt::Display for $nom {
            fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
                self.0.fmt(f)
            }
        }
    )*};
}

identifiant!(FicheId, LigneId, JustificationId, EvenementId, EntreeId,);

/// Les trois référentiels partagent la même forme (3.7).
pub type MissionId = EntreeId;
pub type CompetenceId = EntreeId;
pub type MotifId = EntreeId;
