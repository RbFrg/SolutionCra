//! Port `IdGenerator` (18.1) : identifiants UUID v7, générés hors de `domain`.

use std::sync::atomic::{AtomicU64, Ordering};

use uuid::{Builder, Uuid};

pub trait IdGenerator: Send + Sync {
    fn generer(&self) -> Uuid;
}

/// Production : UUID v7 aléatoires, triables par date de création.
#[derive(Debug, Default, Clone, Copy)]
pub struct UuidV7;

impl IdGenerator for UuidV7 {
    fn generer(&self) -> Uuid {
        Uuid::now_v7()
    }
}

/// Tests et démo : UUID v7 déterministes. L'horodatage est fixe, le compteur
/// occupe la partie « aléatoire » : l'ordre de génération est aussi l'ordre de tri.
#[derive(Debug)]
pub struct SequenceIds {
    millis: u64,
    compteur: AtomicU64,
}

impl SequenceIds {
    pub fn new(millis: u64) -> Self {
        Self {
            millis,
            compteur: AtomicU64::new(0),
        }
    }
}

impl IdGenerator for SequenceIds {
    fn generer(&self) -> Uuid {
        let n = self.compteur.fetch_add(1, Ordering::Relaxed);
        let mut octets = [0u8; 10];
        octets[2..].copy_from_slice(&n.to_be_bytes());
        Builder::from_unix_timestamp_millis(self.millis, &octets).into_uuid()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn les_identifiants_generes_se_trient_par_ordre_de_creation() {
        let ids = SequenceIds::new(1_791_712_800_000);
        let generes: Vec<Uuid> = (0..300).map(|_| ids.generer()).collect();

        let mut tries = generes.clone();
        tries.sort();
        assert_eq!(generes, tries);
        assert!(generes.iter().all(|u| u.get_version_num() == 7));
    }

    #[test]
    fn deux_sequences_identiques_donnent_les_memes_identifiants() {
        let a = SequenceIds::new(42);
        let b = SequenceIds::new(42);
        assert_eq!(a.generer(), b.generer());
    }
}
