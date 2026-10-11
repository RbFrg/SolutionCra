use std::path::PathBuf;
use std::process::ExitCode;
use std::sync::Arc;

use clap::{Parser, Subcommand};
use cra_server::front::FrontEmbarque;
use cra_server::{assembler, construire_app, lire_config, outils, servir};
use tracing_subscriber::EnvFilter;

#[derive(Parser)]
#[command(name = "cra-server", about = "Solution CRA Powered by Rust")]
struct Cli {
    #[command(subcommand)]
    commande: Option<Commande>,
}

#[derive(Subcommand)]
enum Commande {
    /// Démarre le serveur (par défaut).
    Serve,
    /// Interroge /health en local : code de sortie 0 si le serveur répond 200.
    Healthcheck,
    /// Écrit le document OpenAPI en UTF-8.
    Openapi {
        #[arg(long, default_value = "openapi.json")]
        out: PathBuf,
    },
}

#[tokio::main]
async fn main() -> ExitCode {
    match executer(Cli::parse()).await {
        Ok(()) => ExitCode::SUCCESS,
        Err(erreur) => {
            eprintln!("erreur : {erreur:#}");
            ExitCode::FAILURE
        }
    }
}

async fn executer(cli: Cli) -> anyhow::Result<()> {
    match cli.commande.unwrap_or(Commande::Serve) {
        Commande::Openapi { out } => outils::ecrire_openapi(&out),
        Commande::Healthcheck => {
            let config = lire_config(|nom| std::env::var(nom).ok())?;
            outils::healthcheck(outils::adresse_locale(config.bind)).await
        }
        Commande::Serve => {
            tracing_subscriber::fmt()
                .with_env_filter(
                    EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("info")),
                )
                .init();
            let config = lire_config(|nom| std::env::var(nom).ok())?;
            let app = construire_app(&config).await?;
            let routeur = assembler(&config, app, Arc::new(FrontEmbarque));
            let ecouteur = tokio::net::TcpListener::bind(config.bind).await?;
            tracing::info!(adresse = %config.bind, mode = ?config.mode, build = %config.build, "serveur démarré");
            servir(ecouteur, routeur).await
        }
    }
}
