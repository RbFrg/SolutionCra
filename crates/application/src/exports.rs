//! Sorties (tranche 4, section 8) : une fiche, un lot, l'export complet, les
//! tableaux du bilan et le journal. Chaque export écrit une entrée `export` (EXP10).

use chrono::{Datelike, NaiveDate};
use serde_json::{Value, json};

use domain::bilan::{Bilan, calculer_bilan};
use domain::colonnes::colonnes;
use domain::duree::formater_heures;
use domain::journal::{Evenement, FiltreJournal, TypeEvenement};
use domain::semaine::date_paris;
use domain::{AnneeScolaire, Fiche, FicheId, FiltreType, Lundi, NomReferentiel, TypeFiche};

use crate::app::{App, Noms};
use crate::erreur::{ErreurApp, Resultat};
use crate::ports::Lot;

/// Version des colonnes exportées (18.8), indépendante des migrations.
pub const SCHEMA_EXPORT: u32 = 1;
/// Plafond d'un lot (8.2).
pub const LOT_MAX: usize = 120;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FormatDocument {
    Html,
    Docx,
    Pdf,
}

impl FormatDocument {
    pub fn extension(self) -> &'static str {
        match self {
            FormatDocument::Html => "html",
            FormatDocument::Docx => "docx",
            FormatDocument::Pdf => "pdf",
        }
    }

    pub fn type_mime(self) -> &'static str {
        match self {
            FormatDocument::Html => "text/html; charset=utf-8",
            FormatDocument::Docx => {
                "application/vnd.openxmlformats-officedocument.wordprocessingml.document"
            }
            FormatDocument::Pdf => "application/pdf",
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct LigneDocument {
    pub date: NaiveDate,
    pub mission: String,
    pub competences: Vec<String>,
    pub minutes: u16,
    pub etapes: String,
    pub echanges: Option<String>,
}

/// Une fiche prête à rendre : libellés de colonnes du domaine, noms résolus.
#[derive(Debug, Clone, PartialEq)]
pub struct DocumentFiche {
    pub type_fiche: TypeFiche,
    pub lundi: Lundi,
    pub colonnes: [&'static str; 6],
    pub lignes: Vec<LigneDocument>,
    pub total_minutes: u32,
    /// Mention « Démonstration » (M-06).
    pub demo: bool,
}

impl DocumentFiche {
    pub fn titre(&self) -> String {
        format!("Fiche CRA {}", self.type_fiche.libelle())
    }

    pub fn sous_titre(&self) -> String {
        format!(
            "Semaine {} · du {} au {}",
            self.lundi.numero(),
            self.lundi.date().format("%d/%m/%Y"),
            self.lundi.dimanche().format("%d/%m/%Y")
        )
    }

    pub fn total(&self) -> String {
        format!("Total : {}", formater_heures(self.total_minutes))
    }
}

/// Table à plat (CSV) : une ligne d'en-tête, des cellules en texte.
#[derive(Debug, Clone, PartialEq)]
pub struct Table {
    pub nom: String,
    pub colonnes: Vec<String>,
    pub lignes: Vec<Vec<String>>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Fichier {
    pub nom: String,
    pub type_mime: String,
    pub contenu: Vec<u8>,
}

/// Port des rendus binaires (4.4) ; l'implémentation vit dans `adapter-export`.
pub trait Exporteur: Send + Sync {
    fn fiche(&self, document: &DocumentFiche, format: FormatDocument) -> Result<Vec<u8>, String>;
    /// Un seul fichier : une fiche par page (PDF) ou par section (Word).
    fn lot(
        &self,
        titre: &str,
        documents: &[DocumentFiche],
        format: FormatDocument,
    ) -> Result<Vec<u8>, String>;
    /// Une table en CSV : UTF-8 avec BOM, `;`, formules neutralisées (8.3).
    fn csv(&self, table: &Table) -> Vec<u8>;
    /// Archive zip d'un CSV par table, plus `LISEZMOI.txt`.
    fn archive_csv(&self, tables: &[Table], lisezmoi: &str) -> Result<Vec<u8>, String>;
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Periode {
    Mois { annee: i32, mois: u32 },
    Annee(AnneeScolaire),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FormatDonnees {
    Csv,
    Json,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TableauBilan {
    Totaux,
    RetardsParMois,
    JustificationsParMotif,
    TempsParMission,
    TempsParCompetence,
}

impl TableauBilan {
    pub fn depuis_code(code: &str) -> Option<Self> {
        match code {
            "totaux" => Some(Self::Totaux),
            "retards-par-mois" => Some(Self::RetardsParMois),
            "justifications-par-motif" => Some(Self::JustificationsParMotif),
            "temps-par-mission" => Some(Self::TempsParMission),
            "temps-par-competence" => Some(Self::TempsParCompetence),
            _ => None,
        }
    }

    pub fn code(self) -> &'static str {
        match self {
            Self::Totaux => "totaux",
            Self::RetardsParMois => "retards-par-mois",
            Self::JustificationsParMotif => "justifications-par-motif",
            Self::TempsParMission => "temps-par-mission",
            Self::TempsParCompetence => "temps-par-competence",
        }
    }
}

const MOIS: [&str; 12] = [
    "janvier",
    "février",
    "mars",
    "avril",
    "mai",
    "juin",
    "juillet",
    "août",
    "septembre",
    "octobre",
    "novembre",
    "décembre",
];

pub fn nom_du_mois(mois: u32) -> &'static str {
    MOIS.get(mois.saturating_sub(1) as usize)
        .copied()
        .unwrap_or("")
}

fn heures_point(minutes: u32) -> String {
    format!("{}", f64::from(minutes) / 60.0)
}

fn texte(valeur: &Value) -> String {
    match valeur {
        Value::Null => String::new(),
        Value::String(s) => s.clone(),
        Value::Bool(b) => b.to_string(),
        Value::Number(n) => n.to_string(),
        autre => autre.to_string(),
    }
}

fn erreur_rendu(message: String) -> ErreurApp {
    ErreurApp::Technique(format!("rendu impossible : {message}"))
}

/// Table de l'export complet : colonnes et lignes de valeurs JSON typées.
struct TableDonnees {
    nom: &'static str,
    colonnes: Vec<&'static str>,
    lignes: Vec<Vec<Value>>,
}

impl TableDonnees {
    fn en_json(&self) -> Value {
        Value::Array(
            self.lignes
                .iter()
                .map(|l| {
                    Value::Object(
                        self.colonnes
                            .iter()
                            .zip(l)
                            .map(|(c, v)| ((*c).to_owned(), v.clone()))
                            .collect(),
                    )
                })
                .collect(),
        )
    }

    fn en_table(&self) -> Table {
        Table {
            nom: self.nom.to_owned(),
            colonnes: self.colonnes.iter().map(|c| (*c).to_owned()).collect(),
            lignes: self
                .lignes
                .iter()
                .map(|l| l.iter().map(texte).collect())
                .collect(),
        }
    }
}

impl App {
    pub(crate) fn document(&self, fiche: &Fiche, noms: &Noms) -> DocumentFiche {
        DocumentFiche {
            type_fiche: fiche.type_fiche,
            lundi: fiche.lundi,
            colonnes: colonnes(fiche.type_fiche),
            lignes: fiche
                .lignes
                .iter()
                .map(|l| LigneDocument {
                    date: l.date,
                    mission: noms.nom(l.mission),
                    competences: l.competences.iter().map(|c| noms.nom(*c)).collect(),
                    minutes: l.duree.en_minutes(),
                    etapes: l.etapes.clone(),
                    echanges: l.echanges_tuteur.clone(),
                })
                .collect(),
            total_minutes: fiche.total_minutes(),
            demo: self.mode == crate::Mode::Demo,
        }
    }

    async fn tracer_export(&self, resume: String, details: Value) -> Resultat<()> {
        let evenement = self.evenement(
            TypeEvenement::Export,
            ("export", resume.clone()),
            None,
            resume,
            None,
            Some(details),
        );
        self.depot
            .valider(Lot {
                evenements: vec![evenement],
                ..Lot::default()
            })
            .await?;
        Ok(())
    }

    pub async fn exporter_fiche(&self, id: FicheId, format: FormatDocument) -> Resultat<Fichier> {
        let fiche = self.lire_fiche(id).await?.fiche;
        let document = self.document(&fiche, &self.noms().await?);
        let contenu = self
            .exporteur
            .fiche(&document, format)
            .map_err(erreur_rendu)?;
        let nom = format!(
            "fiche-cra-{}-{}-S{:02}.{}",
            fiche.type_fiche.code(),
            fiche.lundi.date().iso_week().year(),
            fiche.lundi.numero(),
            format.extension()
        );
        self.tracer_export(
            format!(
                "Export {} de la fiche S{} {}",
                format.extension(),
                fiche.lundi.numero(),
                fiche.type_fiche.libelle()
            ),
            json!({ "format": format.extension(), "ficheId": id, "fichier": nom }),
        )
        .await?;
        Ok(Fichier {
            nom,
            type_mime: format.type_mime().to_owned(),
            contenu,
        })
    }

    /// Lot par mois (du lundi) ou par année scolaire, trié par lundi puis type (8.2).
    pub async fn exporter_lot(
        &self,
        periode: Periode,
        filtre: FiltreType,
        format: FormatDocument,
    ) -> Resultat<Fichier> {
        let mut fiches: Vec<Fiche> = self
            .fiches_seules()
            .await?
            .into_iter()
            .filter(|f| filtre.accepte(f.type_fiche))
            .filter(|f| match periode {
                Periode::Mois { annee, mois } => {
                    f.lundi.date().year() == annee && f.lundi.date().month() == mois
                }
                Periode::Annee(a) => f.lundi.annee_scolaire() == a,
            })
            .collect();
        if fiches.is_empty() {
            return Err(ErreurApp::Introuvable("Aucune fiche pour cette période."));
        }
        if fiches.len() > LOT_MAX {
            return Err(ErreurApp::regle(
                "lot-trop-grand",
                format!("Un lot compte {LOT_MAX} fiches au plus."),
            ));
        }
        fiches.sort_by_key(|f| (f.lundi, f.type_fiche));
        let noms = self.noms().await?;
        let documents: Vec<DocumentFiche> =
            fiches.iter().map(|f| self.document(f, &noms)).collect();
        let (titre, suffixe) = match periode {
            Periode::Mois { annee, mois } => (
                format!("Fiches CRA · {} {annee}", nom_du_mois(mois)),
                format!("{annee}-{mois:02}"),
            ),
            Periode::Annee(a) => (
                format!("Fiches CRA · année {}-{}", a.valeur(), a.valeur() + 1),
                format!("{}-{}", a.valeur(), a.valeur() + 1),
            ),
        };
        let contenu = self
            .exporteur
            .lot(&titre, &documents, format)
            .map_err(erreur_rendu)?;
        let nom = format!("fiches-cra-{suffixe}.{}", format.extension());
        self.tracer_export(
            format!(
                "Export {} du lot {suffixe} ({} fiches)",
                format.extension(),
                documents.len()
            ),
            json!({ "format": format.extension(), "fiches": documents.len(), "fichier": nom }),
        )
        .await?;
        Ok(Fichier {
            nom,
            type_mime: format.type_mime().to_owned(),
            contenu,
        })
    }

    async fn tables_completes(&self) -> Resultat<Vec<TableDonnees>> {
        let mut tables = Vec::new();
        for (nom_table, referentiel) in [
            ("mission", NomReferentiel::Missions),
            ("competence", NomReferentiel::Competences),
            ("motif", NomReferentiel::Motifs),
        ] {
            tables.push(TableDonnees {
                nom: nom_table,
                colonnes: vec!["id", "nom", "code", "archivee"],
                lignes: self
                    .depot
                    .referentiel(referentiel)
                    .await?
                    .into_iter()
                    .map(|e| vec![json!(e.id), json!(e.nom), json!(e.code), json!(e.archivee)])
                    .collect(),
            });
        }
        let mut plan = self.depot.plan().await?;
        plan.sort_by_key(|p| p.lundi);
        tables.push(TableDonnees {
            nom: "semaine_plan",
            colonnes: vec!["lundi", "typeAttendu", "libelle"],
            lignes: plan
                .into_iter()
                .map(|p| vec![json!(p.lundi), json!(p.type_attendu), json!(p.libelle)])
                .collect(),
        });
        let mut fiches = self.depot.fiches().await?;
        fiches.sort_by_key(|s| (s.fiche.lundi, s.fiche.type_fiche));
        let mut t_fiche = TableDonnees {
            nom: "fiche",
            colonnes: vec![
                "id",
                "lundi",
                "type",
                "statut",
                "termineeLe",
                "transmiseLe",
                "valideeLe",
                "modifieeApresTransmissionLe",
                "version",
                "creeeLe",
                "modifieeLe",
            ],
            lignes: Vec::new(),
        };
        let mut t_ligne = TableDonnees {
            nom: "ligne",
            colonnes: vec![
                "id",
                "ficheId",
                "position",
                "date",
                "missionId",
                "dureeMinutes",
                "dureeHeures",
                "etapes",
                "echangesTuteur",
            ],
            lignes: Vec::new(),
        };
        let mut t_ligne_competence = TableDonnees {
            nom: "ligne_competence",
            colonnes: vec!["ligneId", "competenceId"],
            lignes: Vec::new(),
        };
        for s in &fiches {
            let f = &s.fiche;
            t_fiche.lignes.push(vec![
                json!(f.id),
                json!(f.lundi),
                json!(f.type_fiche),
                json!(f.statut),
                json!(f.terminee_le),
                json!(f.transmise_le),
                json!(f.validee_le),
                json!(f.modifiee_apres_transmission_le),
                json!(s.version),
                json!(s.creee_le),
                json!(s.modifiee_le),
            ]);
            for (position, l) in f.lignes.iter().enumerate() {
                t_ligne.lignes.push(vec![
                    json!(l.id),
                    json!(f.id),
                    json!(position),
                    json!(l.date),
                    json!(l.mission),
                    json!(l.duree.en_minutes()),
                    json!(l.duree.en_heures()),
                    json!(l.etapes),
                    json!(l.echanges_tuteur),
                ]);
                for c in &l.competences {
                    t_ligne_competence.lignes.push(vec![json!(l.id), json!(c)]);
                }
            }
        }
        tables.push(t_fiche);
        tables.push(t_ligne);
        tables.push(t_ligne_competence);
        let mut justifications = self.depot.justifications().await?;
        justifications.sort_by_key(|j| j.id);
        tables.push(TableDonnees {
            nom: "justification",
            colonnes: vec![
                "id",
                "lundi",
                "type",
                "motifId",
                "commentaire",
                "creeeLe",
                "leveeLe",
            ],
            lignes: justifications
                .into_iter()
                .map(|j| {
                    vec![
                        json!(j.id),
                        json!(j.lundi),
                        json!(j.type_fiche),
                        json!(j.motif),
                        json!(j.commentaire),
                        json!(j.creee_le),
                        json!(j.levee_le),
                    ]
                })
                .collect(),
        });
        let mut journal = self
            .journal_complet(&FiltreJournal::default())
            .await?
            .entrees;
        journal.reverse();
        tables.push(table_journal(&journal));
        Ok(tables)
    }

    /// Export complet (8.3) : JSON unique ou archive de CSV.
    pub async fn exporter_complet(&self, format: FormatDonnees) -> Resultat<Fichier> {
        let instant = self.maintenant();
        let tables = self.tables_completes().await?;
        let jour = date_paris(instant).format("%Y-%m-%d");
        let fichier = match format {
            FormatDonnees::Json => {
                let document = json!({
                    "schemaVersion": SCHEMA_EXPORT,
                    "exporteLe": instant,
                    "demonstration": self.mode == crate::Mode::Demo,
                    "tables": tables
                        .iter()
                        .map(|t| (t.nom.to_owned(), t.en_json()))
                        .collect::<serde_json::Map<String, Value>>(),
                });
                Fichier {
                    nom: format!("cra-export-{jour}.json"),
                    type_mime: "application/json".to_owned(),
                    contenu: serde_json::to_vec_pretty(&document)
                        .map_err(|e| ErreurApp::Technique(e.to_string()))?,
                }
            }
            FormatDonnees::Csv => {
                let lisezmoi = format!(
                    "Export complet Solution CRA\n\
                     Version de schéma : {SCHEMA_EXPORT}\n\
                     Exporté le : {}\n{}\n\
                     Un fichier par table, UTF-8 avec BOM, séparateur « ; ».\n\
                     Identifiants : UUID v7. Dates : AAAA-MM-JJ. Instants : ISO 8601 UTC.\n\
                     Durées : dureeMinutes (entier) et dureeHeures (point décimal).\n\
                     Tables : {}.\n",
                    instant.to_rfc3339(),
                    if self.mode == crate::Mode::Demo {
                        "Démonstration : données fictives.\n"
                    } else {
                        ""
                    },
                    tables.iter().map(|t| t.nom).collect::<Vec<_>>().join(", ")
                );
                let csv: Vec<Table> = tables.iter().map(TableDonnees::en_table).collect();
                Fichier {
                    nom: format!("cra-export-v{SCHEMA_EXPORT}-{jour}.zip"),
                    type_mime: "application/zip".to_owned(),
                    contenu: self
                        .exporteur
                        .archive_csv(&csv, &lisezmoi)
                        .map_err(erreur_rendu)?,
                }
            }
        };
        self.tracer_export(
            format!(
                "Export complet ({})",
                if format == FormatDonnees::Json {
                    "json"
                } else {
                    "csv"
                }
            ),
            json!({ "fichier": fichier.nom, "tables": tables.len() }),
        )
        .await?;
        Ok(fichier)
    }

    pub async fn exporter_journal(
        &self,
        filtre: &FiltreJournal,
        format: FormatDonnees,
    ) -> Resultat<Fichier> {
        let entrees = self.journal_complet(filtre).await?.entrees;
        let jour = date_paris(self.maintenant()).format("%Y-%m-%d");
        let table = table_journal(&entrees);
        let fichier = match format {
            FormatDonnees::Json => Fichier {
                nom: format!("cra-journal-{jour}.json"),
                type_mime: "application/json".to_owned(),
                contenu: serde_json::to_vec_pretty(&table.en_json())
                    .map_err(|e| ErreurApp::Technique(e.to_string()))?,
            },
            FormatDonnees::Csv => Fichier {
                nom: format!("cra-journal-{jour}.csv"),
                type_mime: "text/csv; charset=utf-8".to_owned(),
                contenu: self.exporteur.csv(&table.en_table()),
            },
        };
        self.tracer_export(
            format!("Export du journal ({} entrées)", entrees.len()),
            json!({ "fichier": fichier.nom, "entrees": entrees.len() }),
        )
        .await?;
        Ok(fichier)
    }

    pub async fn bilan(&self, annee: AnneeScolaire, filtre: FiltreType) -> Resultat<(Bilan, Noms)> {
        let plan = self.depot.plan().await?;
        let fiches = self.fiches_seules().await?;
        let justifications = self.depot.justifications().await?;
        let bilan = calculer_bilan(
            annee,
            filtre,
            &plan,
            &fiches,
            &justifications,
            self.maintenant(),
        );
        Ok((bilan, self.noms().await?))
    }

    /// Un tableau du bilan en CSV (B-07).
    pub async fn exporter_bilan(
        &self,
        tableau: TableauBilan,
        annee: AnneeScolaire,
        filtre: FiltreType,
    ) -> Resultat<Fichier> {
        let (b, noms) = self.bilan(annee, filtre).await?;
        let c = &b.compteurs;
        let (colonnes, lignes): (Vec<&str>, Vec<Vec<String>>) = match tableau {
            TableauBilan::Totaux => (
                vec!["indicateur", "valeur"],
                vec![
                    vec!["Semaines attendues".into(), c.attendues.to_string()],
                    vec!["Fiches faites".into(), c.faites.to_string()],
                    vec!["Semaines justifiées".into(), c.justifiees.to_string()],
                    vec!["Semaines en retard".into(), c.en_retard.to_string()],
                    vec![
                        "Taux de fiches faites".into(),
                        c.taux_faites.map(|t| format!("{t}")).unwrap_or_default(),
                    ],
                    vec![
                        "Taux de ponctualité".into(),
                        b.taux_ponctualite
                            .map(|t| format!("{t}"))
                            .unwrap_or_default(),
                    ],
                    vec![
                        "Délai moyen de clôture (jours)".into(),
                        b.delai_moyen_jours
                            .map(|d| format!("{d:.1}"))
                            .unwrap_or_default(),
                    ],
                    vec!["Total d'heures".into(), heures_point(b.total_minutes)],
                ],
            ),
            TableauBilan::RetardsParMois => (
                vec!["mois", "enRetard", "justifiees", "faites"],
                b.par_mois
                    .iter()
                    .map(|m| {
                        vec![
                            format!("{} {}", nom_du_mois(m.mois), m.annee),
                            m.en_retard.to_string(),
                            m.justifiees.to_string(),
                            m.faites.to_string(),
                        ]
                    })
                    .collect(),
            ),
            TableauBilan::JustificationsParMotif => (
                vec!["motif", "semaines"],
                b.par_motif
                    .iter()
                    .map(|(id, n)| vec![noms.nom(*id), n.to_string()])
                    .collect(),
            ),
            TableauBilan::TempsParMission => (
                vec!["mission", "heures"],
                b.par_mission
                    .iter()
                    .map(|(id, m)| vec![noms.nom(*id), heures_point(*m)])
                    .collect(),
            ),
            TableauBilan::TempsParCompetence => (
                vec!["competence", "heures"],
                b.par_competence
                    .iter()
                    .map(|(id, m)| vec![noms.nom(*id), heures_point(*m)])
                    .collect(),
            ),
        };
        let table = Table {
            nom: tableau.code().to_owned(),
            colonnes: colonnes.into_iter().map(str::to_owned).collect(),
            lignes,
        };
        let nom = format!("bilan-{}-{}.csv", tableau.code(), annee.valeur());
        self.tracer_export(
            format!("Export du bilan : {}", tableau.code()),
            json!({ "fichier": nom, "lignes": table.lignes.len() }),
        )
        .await?;
        Ok(Fichier {
            nom,
            type_mime: "text/csv; charset=utf-8".to_owned(),
            contenu: self.exporteur.csv(&table),
        })
    }
}

fn table_journal(entrees: &[Evenement]) -> TableDonnees {
    TableDonnees {
        nom: "evenement",
        colonnes: vec![
            "id",
            "horodatage",
            "type",
            "cibleType",
            "cibleId",
            "lundi",
            "resume",
            "avant",
            "apres",
        ],
        lignes: entrees
            .iter()
            .map(|e| {
                vec![
                    json!(e.id),
                    json!(e.horodatage),
                    json!(e.type_evenement),
                    json!(e.cible_type),
                    json!(e.cible_id),
                    json!(e.lundi),
                    json!(e.resume),
                    e.avant
                        .clone()
                        .map_or(Value::Null, |v| Value::String(v.to_string())),
                    e.apres
                        .clone()
                        .map_or(Value::Null, |v| Value::String(v.to_string())),
                ]
            })
            .collect(),
    }
}
