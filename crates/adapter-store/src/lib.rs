//! Dépôt SQLite (`sqlx`, mode WAL, 18.2). Un lot est une transaction : le
//! changement métier et son entrée de journal, ou rien (3.9).

use std::collections::HashMap;
use std::path::Path;
use std::str::FromStr;
use std::time::Duration;

use async_trait::async_trait;
use chrono::{DateTime, NaiveDate, SecondsFormat, Utc};
use sqlx::migrate::Migrator;
use sqlx::sqlite::{
    SqliteConnectOptions, SqliteJournalMode, SqlitePoolOptions, SqliteRow, SqliteSynchronous,
};
use sqlx::{Row, Sqlite, SqlitePool, Transaction};
use uuid::Uuid;

use application::ports::{Depot, EcritureFiche, ErreurDepot, FicheStockee, Lot, PageJournal};
use domain::journal::{Evenement, FiltreJournal, TypeEvenement, trier_recent_d_abord};
use domain::referentiel::cle_de_nom;
use domain::{
    Duree, EntreeId, EntreeReferentiel, EvenementId, Fiche, FicheId, Justification,
    JustificationId, Ligne, LigneId, Lundi, NomReferentiel, SemainePlan, StatutFiche, TypeAttendu,
    TypeFiche,
};

static MIGRATIONS: Migrator = sqlx::migrate!("../../migrations");

#[derive(Debug, Clone)]
pub struct DepotSqlite {
    pool: SqlitePool,
}

fn technique(erreur: impl std::fmt::Display) -> ErreurDepot {
    ErreurDepot::Technique(erreur.to_string())
}

fn instant(i: DateTime<Utc>) -> String {
    i.to_rfc3339_opts(SecondsFormat::Millis, true)
}

fn lire_instant(texte: &str) -> Result<DateTime<Utc>, ErreurDepot> {
    DateTime::parse_from_rfc3339(texte)
        .map(|d| d.with_timezone(&Utc))
        .map_err(technique)
}

fn lire_date(texte: &str) -> Result<NaiveDate, ErreurDepot> {
    NaiveDate::from_str(texte).map_err(technique)
}

fn lire_lundi(texte: &str) -> Result<Lundi, ErreurDepot> {
    Lundi::new(lire_date(texte)?).map_err(technique)
}

fn lire_uuid(texte: &str) -> Result<Uuid, ErreurDepot> {
    Uuid::parse_str(texte).map_err(technique)
}

fn colonne<T>(ligne: &SqliteRow, nom: &str) -> Result<T, ErreurDepot>
where
    T: for<'r> sqlx::Decode<'r, Sqlite> + sqlx::Type<Sqlite>,
{
    ligne.try_get(nom).map_err(technique)
}

fn table(nom: NomReferentiel) -> &'static str {
    match nom {
        NomReferentiel::Missions => "mission",
        NomReferentiel::Competences => "competence",
        NomReferentiel::Motifs => "motif",
    }
}

/// Traduit une violation de contrainte en erreur du port.
fn contrainte(erreur: sqlx::Error, cas: ErreurDepot) -> ErreurDepot {
    match &erreur {
        sqlx::Error::Database(db) if db.is_unique_violation() => cas,
        _ => technique(erreur),
    }
}

impl DepotSqlite {
    /// Ouvre (ou crée) la base et applique les migrations.
    pub async fn ouvrir(chemin: &Path) -> Result<Self, ErreurDepot> {
        let options = SqliteConnectOptions::new()
            .filename(chemin)
            .create_if_missing(true)
            .journal_mode(SqliteJournalMode::Wal)
            .synchronous(SqliteSynchronous::Normal)
            .foreign_keys(true)
            .busy_timeout(Duration::from_secs(5));
        let pool = SqlitePoolOptions::new()
            .max_connections(4)
            .connect_with(options)
            .await
            .map_err(technique)?;
        Self::migrer(pool).await
    }

    /// Base en mémoire, pour les tests : une seule connexion, jamais recyclée.
    pub async fn en_memoire() -> Result<Self, ErreurDepot> {
        let options = SqliteConnectOptions::from_str("sqlite::memory:")
            .map_err(technique)?
            .foreign_keys(true);
        let pool = SqlitePoolOptions::new()
            .max_connections(1)
            .idle_timeout(None)
            .max_lifetime(None)
            .connect_with(options)
            .await
            .map_err(technique)?;
        Self::migrer(pool).await
    }

    async fn migrer(pool: SqlitePool) -> Result<Self, ErreurDepot> {
        MIGRATIONS.run(&pool).await.map_err(technique)?;
        Ok(Self { pool })
    }

    /// Accès direct, pour les tests des contraintes et des déclencheurs.
    pub fn pool(&self) -> &SqlitePool {
        &self.pool
    }

    pub async fn fermer(&self) {
        self.pool.close().await;
    }
}

async fn inserer_lignes(
    tx: &mut Transaction<'_, Sqlite>,
    fiche: &Fiche,
) -> Result<(), ErreurDepot> {
    for (position, l) in fiche.lignes.iter().enumerate() {
        sqlx::query(
            "INSERT INTO ligne (id, fiche_id, position, date, mission_id, duree_minutes, etapes, echanges)
             VALUES (?, ?, ?, ?, ?, ?, ?, ?)",
        )
        .bind(l.id.to_string())
        .bind(fiche.id.to_string())
        .bind(i64::try_from(position).map_err(technique)?)
        .bind(l.date.to_string())
        .bind(l.mission.to_string())
        .bind(i64::from(l.duree.en_minutes()))
        .bind(&l.etapes)
        .bind(&l.echanges_tuteur)
        .execute(&mut **tx)
        .await
        .map_err(technique)?;
        for (rang, c) in l.competences.iter().enumerate() {
            sqlx::query(
                "INSERT INTO ligne_competence (ligne_id, competence_id, rang) VALUES (?, ?, ?)",
            )
            .bind(l.id.to_string())
            .bind(c.to_string())
            .bind(i64::try_from(rang).map_err(technique)?)
            .execute(&mut **tx)
            .await
            .map_err(technique)?;
        }
    }
    Ok(())
}

async fn ecrire_fiche(
    tx: &mut Transaction<'_, Sqlite>,
    ecriture: EcritureFiche,
    maintenant: &str,
) -> Result<(), ErreurDepot> {
    match ecriture {
        EcritureFiche::Creer(fiche) => {
            let existante: Option<String> =
                sqlx::query_scalar("SELECT id FROM fiche WHERE lundi = ? AND type_fiche = ?")
                    .bind(fiche.lundi.to_string())
                    .bind(fiche.type_fiche.code())
                    .fetch_optional(&mut **tx)
                    .await
                    .map_err(technique)?;
            if let Some(id) = existante {
                return Err(ErreurDepot::FicheExistante {
                    fiche_id: FicheId(lire_uuid(&id)?),
                });
            }
            sqlx::query(
                "INSERT INTO fiche (id, lundi, type_fiche, statut, terminee_le, transmise_le, validee_le,
                                    modifiee_apres_transmission_le, version, created_at, updated_at)
                 VALUES (?, ?, ?, ?, ?, ?, ?, ?, 1, ?, ?)",
            )
            .bind(fiche.id.to_string())
            .bind(fiche.lundi.to_string())
            .bind(fiche.type_fiche.code())
            .bind(fiche.statut.code())
            .bind(fiche.terminee_le.map(instant))
            .bind(fiche.transmise_le.map(|d| d.to_string()))
            .bind(fiche.validee_le.map(|d| d.to_string()))
            .bind(fiche.modifiee_apres_transmission_le.map(instant))
            .bind(maintenant)
            .bind(maintenant)
            .execute(&mut **tx)
            .await
            .map_err(|e| {
                contrainte(e, ErreurDepot::FicheExistante { fiche_id: fiche.id })
            })?;
            inserer_lignes(tx, &fiche).await
        }
        EcritureFiche::Remplacer {
            fiche,
            version_attendue,
        } => {
            let version: Option<i64> = sqlx::query_scalar("SELECT version FROM fiche WHERE id = ?")
                .bind(fiche.id.to_string())
                .fetch_optional(&mut **tx)
                .await
                .map_err(technique)?;
            let version = version.ok_or(ErreurDepot::Introuvable)?;
            if version != i64::from(version_attendue) {
                return Err(ErreurDepot::Conflit {
                    version_courante: u32::try_from(version).map_err(technique)?,
                });
            }
            sqlx::query(
                "UPDATE fiche SET statut = ?, terminee_le = ?, transmise_le = ?, validee_le = ?,
                        modifiee_apres_transmission_le = ?, version = version + 1, updated_at = ?
                 WHERE id = ?",
            )
            .bind(fiche.statut.code())
            .bind(fiche.terminee_le.map(instant))
            .bind(fiche.transmise_le.map(|d| d.to_string()))
            .bind(fiche.validee_le.map(|d| d.to_string()))
            .bind(fiche.modifiee_apres_transmission_le.map(instant))
            .bind(maintenant)
            .bind(fiche.id.to_string())
            .execute(&mut **tx)
            .await
            .map_err(technique)?;
            sqlx::query("DELETE FROM ligne WHERE fiche_id = ?")
                .bind(fiche.id.to_string())
                .execute(&mut **tx)
                .await
                .map_err(technique)?;
            inserer_lignes(tx, &fiche).await
        }
        EcritureFiche::Supprimer { id } => {
            let resultat = sqlx::query("DELETE FROM fiche WHERE id = ?")
                .bind(id.to_string())
                .execute(&mut **tx)
                .await
                .map_err(technique)?;
            if resultat.rows_affected() == 0 {
                return Err(ErreurDepot::Introuvable);
            }
            Ok(())
        }
    }
}

#[async_trait]
impl Depot for DepotSqlite {
    async fn plan(&self) -> Result<Vec<SemainePlan>, ErreurDepot> {
        let lignes =
            sqlx::query("SELECT lundi, type_attendu, libelle FROM semaine_plan ORDER BY lundi")
                .fetch_all(&self.pool)
                .await
                .map_err(technique)?;
        lignes
            .iter()
            .map(|l| {
                let code: String = colonne(l, "type_attendu")?;
                Ok(SemainePlan {
                    lundi: lire_lundi(&colonne::<String>(l, "lundi")?)?,
                    type_attendu: TypeAttendu::depuis_code(&code).ok_or_else(|| technique(code))?,
                    libelle: colonne(l, "libelle")?,
                })
            })
            .collect()
    }

    async fn fiches(&self) -> Result<Vec<FicheStockee>, ErreurDepot> {
        let competences = sqlx::query(
            "SELECT ligne_id, competence_id FROM ligne_competence ORDER BY ligne_id, rang",
        )
        .fetch_all(&self.pool)
        .await
        .map_err(technique)?;
        let mut par_ligne: HashMap<String, Vec<EntreeId>> = HashMap::new();
        for c in &competences {
            par_ligne
                .entry(colonne(c, "ligne_id")?)
                .or_default()
                .push(EntreeId(lire_uuid(&colonne::<String>(
                    c,
                    "competence_id",
                )?)?));
        }
        let lignes = sqlx::query(
            "SELECT id, fiche_id, date, mission_id, duree_minutes, etapes, echanges
             FROM ligne ORDER BY fiche_id, position",
        )
        .fetch_all(&self.pool)
        .await
        .map_err(technique)?;
        let mut par_fiche: HashMap<String, Vec<Ligne>> = HashMap::new();
        for l in &lignes {
            let id: String = colonne(l, "id")?;
            let minutes: i64 = colonne(l, "duree_minutes")?;
            par_fiche
                .entry(colonne(l, "fiche_id")?)
                .or_default()
                .push(Ligne {
                    id: LigneId(lire_uuid(&id)?),
                    date: lire_date(&colonne::<String>(l, "date")?)?,
                    mission: EntreeId(lire_uuid(&colonne::<String>(l, "mission_id")?)?),
                    competences: par_ligne.remove(&id).unwrap_or_default(),
                    duree: Duree::minutes(u16::try_from(minutes).map_err(technique)?)
                        .map_err(technique)?,
                    etapes: colonne(l, "etapes")?,
                    echanges_tuteur: colonne(l, "echanges")?,
                });
        }
        let fiches = sqlx::query(
            "SELECT id, lundi, type_fiche, statut, terminee_le, transmise_le, validee_le,
                    modifiee_apres_transmission_le, version, created_at, updated_at FROM fiche",
        )
        .fetch_all(&self.pool)
        .await
        .map_err(technique)?;
        fiches
            .iter()
            .map(|f| {
                let id: String = colonne(f, "id")?;
                let type_fiche: String = colonne(f, "type_fiche")?;
                let statut: String = colonne(f, "statut")?;
                let version: i64 = colonne(f, "version")?;
                Ok(FicheStockee {
                    fiche: Fiche {
                        id: FicheId(lire_uuid(&id)?),
                        lundi: lire_lundi(&colonne::<String>(f, "lundi")?)?,
                        type_fiche: TypeFiche::depuis_code(&type_fiche)
                            .ok_or_else(|| technique(type_fiche))?,
                        statut: StatutFiche::depuis_code(&statut)
                            .ok_or_else(|| technique(statut))?,
                        terminee_le: colonne::<Option<String>>(f, "terminee_le")?
                            .as_deref()
                            .map(lire_instant)
                            .transpose()?,
                        transmise_le: colonne::<Option<String>>(f, "transmise_le")?
                            .as_deref()
                            .map(lire_date)
                            .transpose()?,
                        validee_le: colonne::<Option<String>>(f, "validee_le")?
                            .as_deref()
                            .map(lire_date)
                            .transpose()?,
                        modifiee_apres_transmission_le: colonne::<Option<String>>(
                            f,
                            "modifiee_apres_transmission_le",
                        )?
                        .as_deref()
                        .map(lire_instant)
                        .transpose()?,
                        lignes: par_fiche.remove(&id).unwrap_or_default(),
                    },
                    version: u32::try_from(version).map_err(technique)?,
                    creee_le: lire_instant(&colonne::<String>(f, "created_at")?)?,
                    modifiee_le: lire_instant(&colonne::<String>(f, "updated_at")?)?,
                })
            })
            .collect()
    }

    async fn fiche(&self, id: FicheId) -> Result<Option<FicheStockee>, ErreurDepot> {
        Ok(self.fiches().await?.into_iter().find(|s| s.fiche.id == id))
    }

    async fn justifications(&self) -> Result<Vec<Justification>, ErreurDepot> {
        let lignes = sqlx::query(
            "SELECT id, lundi, type_fiche, motif_id, commentaire, creee_le, levee_le FROM justification ORDER BY id",
        )
        .fetch_all(&self.pool)
        .await
        .map_err(technique)?;
        lignes
            .iter()
            .map(|l| {
                let type_fiche: String = colonne(l, "type_fiche")?;
                Ok(Justification {
                    id: JustificationId(lire_uuid(&colonne::<String>(l, "id")?)?),
                    lundi: lire_lundi(&colonne::<String>(l, "lundi")?)?,
                    type_fiche: TypeFiche::depuis_code(&type_fiche)
                        .ok_or_else(|| technique(type_fiche))?,
                    motif: EntreeId(lire_uuid(&colonne::<String>(l, "motif_id")?)?),
                    commentaire: colonne(l, "commentaire")?,
                    creee_le: lire_instant(&colonne::<String>(l, "creee_le")?)?,
                    levee_le: colonne::<Option<String>>(l, "levee_le")?
                        .as_deref()
                        .map(lire_instant)
                        .transpose()?,
                })
            })
            .collect()
    }

    async fn referentiel(
        &self,
        nom: NomReferentiel,
    ) -> Result<Vec<EntreeReferentiel>, ErreurDepot> {
        let requete = format!(
            "SELECT id, nom, code, archivee FROM {} ORDER BY id",
            table(nom)
        );
        let lignes = sqlx::query(&requete)
            .fetch_all(&self.pool)
            .await
            .map_err(technique)?;
        lignes
            .iter()
            .map(|l| {
                Ok(EntreeReferentiel {
                    id: EntreeId(lire_uuid(&colonne::<String>(l, "id")?)?),
                    nom: colonne(l, "nom")?,
                    code: colonne(l, "code")?,
                    archivee: colonne::<i64>(l, "archivee")? != 0,
                })
            })
            .collect()
    }

    async fn journal(
        &self,
        filtre: &FiltreJournal,
        page: usize,
        taille: usize,
    ) -> Result<PageJournal, ErreurDepot> {
        // Mono-utilisateur : quelques milliers d'entrées au plus ; le filtre
        // reprend la règle du domaine plutôt que de la recopier en SQL.
        let lignes = sqlx::query(
            "SELECT id, horodatage, type, cible_type, cible_id, lundi, resume, avant, apres FROM evenement",
        )
        .fetch_all(&self.pool)
        .await
        .map_err(technique)?;
        let mut entrees = Vec::with_capacity(lignes.len());
        for l in &lignes {
            let type_code: String = colonne(l, "type")?;
            let json = |nom: &str| -> Result<Option<serde_json::Value>, ErreurDepot> {
                colonne::<Option<String>>(l, nom)?
                    .map(|t| serde_json::from_str(&t).map_err(technique))
                    .transpose()
            };
            let e = Evenement {
                id: EvenementId(lire_uuid(&colonne::<String>(l, "id")?)?),
                horodatage: lire_instant(&colonne::<String>(l, "horodatage")?)?,
                type_evenement: TypeEvenement::depuis_code(&type_code)
                    .ok_or_else(|| technique(type_code))?,
                cible_type: colonne(l, "cible_type")?,
                cible_id: colonne(l, "cible_id")?,
                lundi: colonne::<Option<String>>(l, "lundi")?
                    .as_deref()
                    .map(lire_lundi)
                    .transpose()?,
                resume: colonne(l, "resume")?,
                avant: json("avant")?,
                apres: json("apres")?,
            };
            if filtre.accepte(&e) {
                entrees.push(e);
            }
        }
        trier_recent_d_abord(&mut entrees);
        let total = entrees.len();
        let debut = page.saturating_sub(1).saturating_mul(taille);
        Ok(PageJournal {
            entrees: entrees.into_iter().skip(debut).take(taille).collect(),
            total,
        })
    }

    async fn valider(&self, lot: Lot) -> Result<(), ErreurDepot> {
        let maintenant = instant(
            lot.evenements
                .first()
                .map_or_else(Utc::now, |e| e.horodatage),
        );
        let mut tx = self.pool.begin().await.map_err(technique)?;

        // Ordre imposé par les clés étrangères : référentiels, plan, fiches, justifications.
        for (nom, e) in lot.referentiels {
            let requete = format!(
                "INSERT INTO {} (id, nom, nom_cle, code, archivee, created_at, updated_at)
                 VALUES (?, ?, ?, ?, ?, ?, ?)
                 ON CONFLICT(id) DO UPDATE SET nom = excluded.nom, nom_cle = excluded.nom_cle,
                   code = excluded.code, archivee = excluded.archivee, updated_at = excluded.updated_at",
                table(nom)
            );
            sqlx::query(&requete)
                .bind(e.id.to_string())
                .bind(&e.nom)
                .bind(cle_de_nom(&e.nom))
                .bind(&e.code)
                .bind(i64::from(e.archivee))
                .bind(&maintenant)
                .bind(&maintenant)
                .execute(&mut *tx)
                .await
                .map_err(|err| contrainte(err, ErreurDepot::NomEnDouble))?;
        }
        for p in lot.plan {
            sqlx::query(
                "INSERT INTO semaine_plan (lundi, type_attendu, libelle, updated_at) VALUES (?, ?, ?, ?)
                 ON CONFLICT(lundi) DO UPDATE SET type_attendu = excluded.type_attendu,
                   libelle = excluded.libelle, updated_at = excluded.updated_at",
            )
            .bind(p.lundi.to_string())
            .bind(p.type_attendu.code())
            .bind(&p.libelle)
            .bind(&maintenant)
            .execute(&mut *tx)
            .await
            .map_err(technique)?;
        }
        for ecriture in lot.fiches {
            ecrire_fiche(&mut tx, ecriture, &maintenant).await?;
        }
        for j in lot.justifications {
            sqlx::query(
                "INSERT INTO justification (id, lundi, type_fiche, motif_id, commentaire, creee_le, levee_le)
                 VALUES (?, ?, ?, ?, ?, ?, ?)
                 ON CONFLICT(id) DO UPDATE SET commentaire = excluded.commentaire, levee_le = excluded.levee_le",
            )
            .bind(j.id.to_string())
            .bind(j.lundi.to_string())
            .bind(j.type_fiche.code())
            .bind(j.motif.to_string())
            .bind(&j.commentaire)
            .bind(instant(j.creee_le))
            .bind(j.levee_le.map(instant))
            .execute(&mut *tx)
            .await
            .map_err(|e| contrainte(e, ErreurDepot::JustificationActiveExistante))?;
        }
        for e in lot.evenements {
            sqlx::query(
                "INSERT INTO evenement (id, horodatage, type, cible_type, cible_id, lundi, resume, avant, apres)
                 VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?)",
            )
            .bind(e.id.to_string())
            .bind(instant(e.horodatage))
            .bind(e.type_evenement.code())
            .bind(&e.cible_type)
            .bind(&e.cible_id)
            .bind(e.lundi.map(|l| l.to_string()))
            .bind(&e.resume)
            .bind(e.avant.map(|v| v.to_string()))
            .bind(e.apres.map(|v| v.to_string()))
            .execute(&mut *tx)
            .await
            .map_err(technique)?;
        }
        tx.commit().await.map_err(technique)
    }
}
