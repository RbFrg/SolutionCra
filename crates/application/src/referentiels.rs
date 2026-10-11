//! Référentiels : lister, ajouter, modifier, archiver, importer en CSV (3.7, 19.5).

use std::collections::HashSet;

use domain::journal::TypeEvenement;
use domain::referentiel::{
    CODE_AUTRE, MOTIFS_PAR_DEFAUT, avertissement_competence, cle_de_nom, normaliser_nom,
    peut_archiver, valider_entree,
};
use domain::{EntreeId, EntreeReferentiel, ErreurValidation, NomReferentiel};

use crate::app::{App, json};
use crate::erreur::{ErreurApp, Resultat};
use crate::ports::Lot;

/// Lignes au plus par fichier importé (19.5).
pub const IMPORT_LIGNES_MAX: usize = 1000;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ActionImport {
    Creer,
    Ignorer,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LigneImport {
    /// Numéro de ligne dans le fichier (l'en-tête est la ligne 1).
    pub ligne: usize,
    pub nom: String,
    pub code: Option<String>,
    pub archivee: bool,
    pub action: ActionImport,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MessageLigne {
    pub ligne: usize,
    pub message: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct ApercuImport {
    pub lignes: Vec<LigneImport>,
    pub erreurs: Vec<MessageLigne>,
    pub avertissements: Vec<MessageLigne>,
}

impl ApercuImport {
    pub fn a_creer(&self) -> usize {
        self.lignes
            .iter()
            .filter(|l| l.action == ActionImport::Creer)
            .count()
    }
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct ModificationEntree {
    pub nom: Option<String>,
    pub code: Option<Option<String>>,
    pub archivee: Option<bool>,
}

fn booleen(texte: &str) -> Option<bool> {
    match texte.trim().to_lowercase().as_str() {
        "" | "non" | "0" | "faux" | "false" => Some(false),
        "oui" | "1" | "vrai" | "true" => Some(true),
        _ => None,
    }
}

/// Analyse un CSV de référentiel (19.5) : UTF-8 (BOM toléré), séparateur `;`
/// ou `,`, en-tête obligatoire, colonne `nom` obligatoire.
pub fn analyser_csv(
    texte: &str,
    existantes: &[EntreeReferentiel],
    competences: bool,
) -> ApercuImport {
    let texte = texte.trim_start_matches('\u{feff}');
    let mut apercu = ApercuImport::default();
    let premiere = texte.lines().next().unwrap_or_default();
    let separateur = if premiere.matches(';').count() >= premiere.matches(',').count() {
        b';'
    } else {
        b','
    };
    let mut lecteur = csv::ReaderBuilder::new()
        .delimiter(separateur)
        .flexible(true)
        .from_reader(texte.as_bytes());
    let entetes: Vec<String> = match lecteur.headers() {
        Ok(h) => h.iter().map(|c| c.trim().to_lowercase()).collect(),
        Err(_) => {
            apercu.erreurs.push(MessageLigne {
                ligne: 1,
                message: "En-tête illisible.".to_owned(),
            });
            return apercu;
        }
    };
    let index = |nom: &str| entetes.iter().position(|e| e == nom);
    let Some(i_nom) = index("nom") else {
        apercu.erreurs.push(MessageLigne {
            ligne: 1,
            message: "La colonne « nom » est obligatoire.".to_owned(),
        });
        return apercu;
    };
    let i_code = index("code");
    let i_archivee = index("archivee");
    for inconnue in entetes
        .iter()
        .filter(|e| !["nom", "code", "archivee"].contains(&e.as_str()))
    {
        apercu.avertissements.push(MessageLigne {
            ligne: 1,
            message: format!("Colonne « {inconnue} » ignorée."),
        });
    }

    let cles_existantes: HashSet<String> = existantes.iter().map(|e| cle_de_nom(&e.nom)).collect();
    let mut cles_fichier: HashSet<String> = HashSet::new();
    for (n, enregistrement) in lecteur.records().enumerate() {
        let ligne = n + 2;
        if n >= IMPORT_LIGNES_MAX {
            apercu.erreurs.push(MessageLigne {
                ligne,
                message: format!("Un fichier compte {IMPORT_LIGNES_MAX} lignes au plus."),
            });
            break;
        }
        let Ok(enregistrement) = enregistrement else {
            apercu.erreurs.push(MessageLigne {
                ligne,
                message: "Ligne illisible.".to_owned(),
            });
            continue;
        };
        let champ = |i: Option<usize>| i.and_then(|i| enregistrement.get(i)).unwrap_or("").trim();
        let nom = match normaliser_nom(champ(Some(i_nom))) {
            Ok(nom) => nom,
            Err(e) => {
                apercu.erreurs.push(MessageLigne {
                    ligne,
                    message: e.message,
                });
                continue;
            }
        };
        let Some(archivee) = booleen(champ(i_archivee)) else {
            apercu.erreurs.push(MessageLigne {
                ligne,
                message: "« archivee » vaut oui, non, 1, 0, vrai, faux, true ou false.".to_owned(),
            });
            continue;
        };
        let code = Some(champ(i_code))
            .filter(|c| !c.is_empty())
            .map(str::to_owned);
        let cle = cle_de_nom(&nom);
        if !cles_fichier.insert(cle.clone()) {
            apercu.erreurs.push(MessageLigne {
                ligne,
                message: format!("« {nom} » est en double dans le fichier."),
            });
            continue;
        }
        let action = if cles_existantes.contains(&cle) {
            apercu.avertissements.push(MessageLigne {
                ligne,
                message: format!("« {nom} » existe déjà : ligne ignorée."),
            });
            ActionImport::Ignorer
        } else {
            ActionImport::Creer
        };
        if competences
            && action == ActionImport::Creer
            && let Some(message) = avertissement_competence(&nom)
        {
            apercu.avertissements.push(MessageLigne { ligne, message });
        }
        apercu.lignes.push(LigneImport {
            ligne,
            nom,
            code,
            archivee,
            action,
        });
    }
    apercu
}

impl App {
    pub async fn referentiel(
        &self,
        nom: NomReferentiel,
        avec_archivees: bool,
    ) -> Resultat<Vec<EntreeReferentiel>> {
        let mut entrees: Vec<EntreeReferentiel> = self
            .depot
            .referentiel(nom)
            .await?
            .into_iter()
            .filter(|e| avec_archivees || !e.archivee)
            .collect();
        entrees.sort_by_key(|e| cle_de_nom(&e.nom));
        Ok(entrees)
    }

    /// Amorce les cinq motifs par défaut au premier démarrage (REF7).
    pub async fn amorcer(&self) -> Resultat<()> {
        if !self
            .depot
            .referentiel(NomReferentiel::Motifs)
            .await?
            .is_empty()
        {
            return Ok(());
        }
        let referentiels = MOTIFS_PAR_DEFAUT
            .iter()
            .map(|(nom, code)| {
                (
                    NomReferentiel::Motifs,
                    EntreeReferentiel {
                        id: EntreeId(self.nouvel_id()),
                        nom: (*nom).to_owned(),
                        code: code.map(str::to_owned),
                        archivee: false,
                    },
                )
            })
            .collect();
        self.depot
            .valider(Lot {
                referentiels,
                ..Lot::default()
            })
            .await?;
        Ok(())
    }

    pub async fn ajouter_entree(
        &self,
        referentiel: NomReferentiel,
        nom: &str,
        code: Option<&str>,
    ) -> Resultat<(EntreeReferentiel, Option<String>)> {
        let existantes = self.depot.referentiel(referentiel).await?;
        let id = EntreeId(self.nouvel_id());
        let (nom, code) = valider_entree(&existantes, referentiel, id, nom, code)?;
        if code.as_deref() == Some(CODE_AUTRE) {
            let mut v = ErreurValidation::default();
            v.ajouter("code", "Le code « autre » est réservé.");
            return Err(ErreurApp::Validation(v));
        }
        let entree = EntreeReferentiel {
            id,
            nom,
            code,
            archivee: false,
        };
        let avertissement = (referentiel == NomReferentiel::Competences)
            .then(|| avertissement_competence(&entree.nom))
            .flatten();
        let evenement = self.evenement(
            TypeEvenement::ReferentielModifie,
            (referentiel.code(), entree.id.to_string()),
            None,
            format!(
                "Référentiel modifié ({}) : « {} » ajoutée",
                referentiel.code(),
                entree.nom
            ),
            None,
            json(&entree),
        );
        self.depot
            .valider(Lot {
                referentiels: vec![(referentiel, entree.clone())],
                evenements: vec![evenement],
                ..Lot::default()
            })
            .await?;
        Ok((entree, avertissement))
    }

    /// Renommer, changer le code, archiver ou désarchiver ; jamais supprimer (G-03).
    pub async fn modifier_entree(
        &self,
        referentiel: NomReferentiel,
        id: EntreeId,
        modification: ModificationEntree,
    ) -> Resultat<(EntreeReferentiel, Option<String>)> {
        let existantes = self.depot.referentiel(referentiel).await?;
        let avant = existantes
            .iter()
            .find(|e| e.id == id)
            .cloned()
            .ok_or(ErreurApp::Introuvable("Entrée introuvable."))?;
        let nom_voulu = modification.nom.as_deref().unwrap_or(&avant.nom);
        let code_voulu = match &modification.code {
            Some(c) => c.as_deref(),
            None => avant.code.as_deref(),
        };
        let (nom, code) = valider_entree(&existantes, referentiel, id, nom_voulu, code_voulu)?;
        let archivee = modification.archivee.unwrap_or(avant.archivee);
        if archivee && !avant.archivee && !peut_archiver(&avant, referentiel) {
            return Err(ErreurApp::regle(
                "archivage-interdit",
                "Le motif « Autre » ne peut pas être archivé.",
            ));
        }
        if referentiel == NomReferentiel::Motifs
            && code.as_deref() == Some(CODE_AUTRE)
            && avant.code.as_deref() != Some(CODE_AUTRE)
        {
            let mut v = ErreurValidation::default();
            v.ajouter("code", "Le code « autre » est réservé.");
            return Err(ErreurApp::Validation(v));
        }
        let apres = EntreeReferentiel {
            id,
            nom,
            code,
            archivee,
        };
        if apres == avant {
            return Ok((apres, None));
        }
        let avertissement = (referentiel == NomReferentiel::Competences)
            .then(|| avertissement_competence(&apres.nom))
            .flatten();
        let evenement = self.evenement(
            TypeEvenement::ReferentielModifie,
            (referentiel.code(), id.to_string()),
            None,
            format!(
                "Référentiel modifié ({}) : « {} »",
                referentiel.code(),
                apres.nom
            ),
            json(&avant),
            json(&apres),
        );
        self.depot
            .valider(Lot {
                referentiels: vec![(referentiel, apres.clone())],
                evenements: vec![evenement],
                ..Lot::default()
            })
            .await?;
        Ok((apres, avertissement))
    }

    /// Aperçu (sans écriture) ou validation tout ou rien d'un import CSV (19.5).
    pub async fn importer_referentiel(
        &self,
        referentiel: NomReferentiel,
        csv: &str,
        valider: bool,
    ) -> Resultat<ApercuImport> {
        let existantes = self.depot.referentiel(referentiel).await?;
        let apercu = analyser_csv(csv, &existantes, referentiel == NomReferentiel::Competences);
        if !valider {
            return Ok(apercu);
        }
        if !apercu.erreurs.is_empty() {
            let mut v = ErreurValidation::default();
            for e in &apercu.erreurs {
                v.ajouter(format!("lignes[{}]", e.ligne), e.message.clone());
            }
            return Err(ErreurApp::Validation(v));
        }
        let crees: Vec<EntreeReferentiel> = apercu
            .lignes
            .iter()
            .filter(|l| l.action == ActionImport::Creer)
            .map(|l| EntreeReferentiel {
                id: EntreeId(self.nouvel_id()),
                nom: l.nom.clone(),
                code: l.code.clone(),
                archivee: l.archivee,
            })
            .collect();
        if existantes.len() + crees.len() > domain::referentiel::ENTREES_MAX {
            return Err(ErreurApp::regle(
                "referentiel-plein",
                "Le référentiel dépasserait 2 000 entrées.",
            ));
        }
        let ignorees = apercu.lignes.len() - crees.len();
        let mut evenements = vec![self.evenement(
            TypeEvenement::Import,
            (referentiel.code(), "import".to_owned()),
            None,
            format!(
                "Import ({}) : {} créées, {} ignorées",
                referentiel.code(),
                crees.len(),
                ignorees
            ),
            None,
            Some(serde_json::json!({ "crees": crees.len(), "ignorees": ignorees })),
        )];
        if !crees.is_empty() {
            evenements.push(self.evenement(
                TypeEvenement::ReferentielModifie,
                (referentiel.code(), "import".to_owned()),
                None,
                format!(
                    "Référentiel modifié ({}) : {} entrées importées",
                    referentiel.code(),
                    crees.len()
                ),
                None,
                json(&crees),
            ));
        }
        self.depot
            .valider(Lot {
                referentiels: crees.into_iter().map(|e| (referentiel, e)).collect(),
                evenements,
                ..Lot::default()
            })
            .await?;
        Ok(apercu)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use uuid::Uuid;

    fn existante(nom: &str) -> EntreeReferentiel {
        EntreeReferentiel {
            id: EntreeId(Uuid::from_u128(1)),
            nom: nom.to_owned(),
            code: None,
            archivee: false,
        }
    }

    #[test]
    fn la_colonne_nom_est_obligatoire() {
        let apercu = analyser_csv("code;archivee\nA;non\n", &[], false);
        assert_eq!(
            apercu.erreurs[0].message,
            "La colonne « nom » est obligatoire."
        );
    }

    #[test]
    fn code_et_archivee_sont_facultatives() {
        let apercu = analyser_csv("nom\nPipeline CI\n", &[], false);
        assert!(apercu.erreurs.is_empty());
        assert_eq!(apercu.lignes[0].code, None);
        assert!(!apercu.lignes[0].archivee);
    }

    #[test]
    fn un_doublon_dans_le_fichier_est_signale_avec_son_numero_de_ligne() {
        let apercu = analyser_csv("nom\nDocumenter\ndocumenter\n", &[], false);
        assert_eq!(apercu.erreurs[0].ligne, 3);
    }

    #[test]
    fn un_nom_deja_present_est_ignore_avec_un_avertissement() {
        let apercu = analyser_csv("nom\nDéployer\n", &[existante("deployer")], false);
        assert_eq!(apercu.lignes[0].action, ActionImport::Ignorer);
        assert_eq!(apercu.avertissements.len(), 1);
    }

    #[test]
    fn archivee_accepte_oui_non_1_0_vrai_faux() {
        let csv = "nom;archivee\nA;oui\nB;non\nC;1\nD;0\nE;vrai\nF;faux\nG;true\nH;\n";
        let apercu = analyser_csv(csv, &[], false);
        let valeurs: Vec<bool> = apercu.lignes.iter().map(|l| l.archivee).collect();
        assert_eq!(
            valeurs,
            [true, false, true, false, true, false, true, false]
        );
        assert!(
            analyser_csv("nom;archivee\nA;peut-être\n", &[], false)
                .erreurs
                .len()
                == 1
        );
    }

    #[test]
    fn le_bom_et_la_virgule_sont_toleres_et_une_colonne_inconnue_est_signalee() {
        let apercu = analyser_csv("\u{feff}nom,commentaire\nTester et valider,x\n", &[], true);
        assert_eq!(apercu.lignes[0].nom, "Tester et valider");
        assert_eq!(apercu.avertissements[0].ligne, 1);
    }
}
