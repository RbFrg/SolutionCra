//! Lecture de la configuration (18.6), une seule fois au démarrage.

use std::net::SocketAddr;
use std::path::PathBuf;

use application::Mode;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Config {
    pub mode: Mode,
    pub bind: SocketAddr,
    pub db_path: PathBuf,
    pub origines_autorisees: Vec<String>,
    pub build: String,
}

#[derive(Debug, thiserror::Error, PartialEq, Eq)]
pub enum ErreurConfig {
    #[error("APP_MODE doit valoir `production` ou `demo` (reçu : `{0}`)")]
    ModeInconnu(String),
    #[error("CRA_BIND invalide : `{0}`")]
    AdresseInvalide(String),
    #[error(
        "le mode démo refuse {0} : une démo ne touche jamais une base ni un secret réels (M-07)"
    )]
    DemoAvecSecret(&'static str),
}

const BIND_PAR_DEFAUT: &str = "0.0.0.0:8080";
const BASE_PAR_DEFAUT: &str = "cra.db";

/// `variable` renvoie la valeur d'une variable d'environnement ; une valeur
/// vide vaut une variable absente (Compose ne sait pas retirer une variable).
pub fn lire_config(variable: impl Fn(&str) -> Option<String>) -> Result<Config, ErreurConfig> {
    let lire = |nom: &str| variable(nom).filter(|v| !v.trim().is_empty());

    let mode = match lire("APP_MODE").as_deref().map(str::trim) {
        None | Some("production") => Mode::Production,
        Some("demo") => Mode::Demo,
        Some(autre) => return Err(ErreurConfig::ModeInconnu(autre.to_owned())),
    };

    if mode == Mode::Demo {
        if lire("CRA_DB_PATH").is_some() {
            return Err(ErreurConfig::DemoAvecSecret("CRA_DB_PATH"));
        }
        if lire("CRA_SECRET_KEY").is_some() {
            return Err(ErreurConfig::DemoAvecSecret("CRA_SECRET_KEY"));
        }
    }

    let bind_texte = lire("CRA_BIND").unwrap_or_else(|| BIND_PAR_DEFAUT.to_owned());
    let bind = bind_texte
        .parse()
        .map_err(|_| ErreurConfig::AdresseInvalide(bind_texte.clone()))?;

    let origines_autorisees = lire("CRA_ALLOWED_ORIGINS")
        .map(|v| {
            v.split(',')
                .map(|o| o.trim().trim_end_matches('/').to_owned())
                .filter(|o| !o.is_empty())
                .collect()
        })
        .unwrap_or_else(|| vec!["http://localhost:5173".to_owned()]);

    Ok(Config {
        mode,
        bind,
        db_path: PathBuf::from(lire("CRA_DB_PATH").unwrap_or_else(|| BASE_PAR_DEFAUT.to_owned())),
        origines_autorisees,
        build: lire("GIT_SHA").unwrap_or_else(|| "dev".to_owned()),
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;

    fn env(paires: &[(&str, &str)]) -> impl Fn(&str) -> Option<String> {
        let table: HashMap<String, String> = paires
            .iter()
            .map(|(k, v)| ((*k).to_owned(), (*v).to_owned()))
            .collect();
        move |nom| table.get(nom).cloned()
    }

    #[test]
    fn sans_variable_le_mode_est_production_sur_le_port_8080() {
        let config = lire_config(env(&[])).unwrap();
        assert_eq!(config.mode, Mode::Production);
        assert_eq!(config.bind.port(), 8080);
        assert_eq!(config.build, "dev");
    }

    #[test]
    fn app_mode_demo_est_lu() {
        let config = lire_config(env(&[("APP_MODE", "demo")])).unwrap();
        assert_eq!(config.mode, Mode::Demo);
    }

    #[test]
    fn le_demarrage_refuse_demo_avec_cra_db_path_ou_cra_secret_key() {
        assert_eq!(
            lire_config(env(&[
                ("APP_MODE", "demo"),
                ("CRA_DB_PATH", "/data/cra.db")
            ])),
            Err(ErreurConfig::DemoAvecSecret("CRA_DB_PATH"))
        );
        assert_eq!(
            lire_config(env(&[("APP_MODE", "demo"), ("CRA_SECRET_KEY", "c2VjcmV0")])),
            Err(ErreurConfig::DemoAvecSecret("CRA_SECRET_KEY"))
        );
    }

    #[test]
    fn une_variable_vide_vaut_une_variable_absente() {
        let config = lire_config(env(&[("APP_MODE", "demo"), ("CRA_DB_PATH", "")])).unwrap();
        assert_eq!(config.mode, Mode::Demo);
    }

    #[test]
    fn un_mode_inconnu_est_refuse() {
        assert!(matches!(
            lire_config(env(&[("APP_MODE", "test")])),
            Err(ErreurConfig::ModeInconnu(_))
        ));
    }

    #[test]
    fn les_origines_autorisees_se_separent_par_des_virgules() {
        let config = lire_config(env(&[(
            "CRA_ALLOWED_ORIGINS",
            "https://a.duckdns.org/, http://localhost:5173",
        )]))
        .unwrap();
        assert_eq!(
            config.origines_autorisees,
            vec!["https://a.duckdns.org", "http://localhost:5173"]
        );
    }
}
