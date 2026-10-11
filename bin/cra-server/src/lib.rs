//! Composition root : lit la configuration, assemble les adaptateurs selon
//! le mode (4.5) et démarre le serveur.

pub mod config;
pub mod front;
pub mod outils;

use std::sync::Arc;

use adapter_export::Exports;
use adapter_http::{Assets, Etat};
use adapter_memory::{DepotMemoire, jeu_demo};
use adapter_store::DepotSqlite;
use anyhow::Context;
use application::mode::instant_demo;
use application::{App, FixedClock, Mode, SequenceIds, SystemClock, UuidV7};
use tokio::net::TcpListener;

pub use config::{Config, lire_config};

/// Horodatage des identifiants créés pendant une démo (après ceux du jeu).
const MILLIS_DEMO: u64 = 1_791_734_400_000;

/// Assemble l'application : démo en mémoire à date figée, ou production sur SQLite.
pub async fn construire_app(config: &Config) -> anyhow::Result<App> {
    let app = match config.mode {
        Mode::Demo => {
            let depot = DepotMemoire::new(jeu_demo());
            App::new(
                Arc::new(depot.clone()),
                Arc::new(FixedClock::new(instant_demo())),
                Arc::new(SequenceIds::new(MILLIS_DEMO)),
                Arc::new(Exports),
                Mode::Demo,
            )
            .avec_reinitialisation(Arc::new(depot))
        }
        Mode::Production => {
            let depot = DepotSqlite::ouvrir(&config.db_path)
                .await
                .with_context(|| format!("ouverture de la base {}", config.db_path.display()))?;
            App::new(
                Arc::new(depot),
                Arc::new(SystemClock),
                Arc::new(UuidV7),
                Arc::new(Exports),
                Mode::Production,
            )
        }
    };
    app.amorcer().await.context("amorçage des motifs")?;
    Ok(app)
}

/// Assemble le routeur complet pour une application donnée.
pub fn assembler(config: &Config, app: App, assets: Arc<dyn Assets>) -> axum::Router {
    adapter_http::router(Etat {
        app: Arc::new(app),
        build: config.build.clone(),
        assets,
        origines_autorisees: Arc::new(config.origines_autorisees.clone()),
    })
}

/// Sert l'application sur un écouteur déjà ouvert, jusqu'à Ctrl+C ou SIGTERM.
pub async fn servir(ecouteur: TcpListener, routeur: axum::Router) -> anyhow::Result<()> {
    axum::serve(ecouteur, routeur)
        .with_graceful_shutdown(arret())
        .await?;
    Ok(())
}

async fn arret() {
    let ctrl_c = async {
        let _ = tokio::signal::ctrl_c().await;
    };
    #[cfg(unix)]
    let terminer = async {
        match tokio::signal::unix::signal(tokio::signal::unix::SignalKind::terminate()) {
            Ok(mut signal) => {
                signal.recv().await;
            }
            Err(_) => std::future::pending::<()>().await,
        }
    };
    #[cfg(not(unix))]
    let terminer = std::future::pending::<()>();
    tokio::select! {
        () = ctrl_c => {},
        () = terminer => {},
    }
}
