//! Sous-commandes `healthcheck` (Compose, image distroless sans curl) et
//! `openapi` (génération du contrat, `just gen-api`).

use std::net::SocketAddr;
use std::path::Path;
use std::time::Duration;

use anyhow::{Context, bail};
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpStream;

/// Interroge `GET /health` sur `adresse` ; succès si la réponse est un 200.
pub async fn healthcheck(adresse: SocketAddr) -> anyhow::Result<()> {
    let echange = async {
        let mut flux = TcpStream::connect(adresse).await?;
        flux.write_all(b"GET /health HTTP/1.1\r\nHost: localhost\r\nConnection: close\r\n\r\n")
            .await?;
        let mut reponse = Vec::new();
        flux.read_to_end(&mut reponse).await?;
        anyhow::Ok(reponse)
    };
    let reponse = tokio::time::timeout(Duration::from_secs(3), echange)
        .await
        .context("délai dépassé")??;
    let premiere_ligne = String::from_utf8_lossy(&reponse)
        .lines()
        .next()
        .unwrap_or_default()
        .to_owned();
    if premiere_ligne.split_whitespace().nth(1) == Some("200") {
        Ok(())
    } else {
        bail!("réponse inattendue : {premiere_ligne}")
    }
}

/// Écrit le document OpenAPI en UTF-8, sans passer par une redirection de
/// shell (PowerShell 5.1 écrirait de l'UTF-16, 15.3).
pub fn ecrire_openapi(chemin: &Path) -> anyhow::Result<()> {
    let mut json = adapter_http::openapi()
        .to_pretty_json()
        .context("sérialisation du document OpenAPI")?;
    json.push('\n');
    std::fs::write(chemin, json).with_context(|| format!("écriture de {}", chemin.display()))
}

/// Adresse locale à interroger pour une adresse d'écoute (0.0.0.0 devient 127.0.0.1).
pub fn adresse_locale(bind: SocketAddr) -> SocketAddr {
    let mut locale = bind;
    if locale.ip().is_unspecified() {
        locale.set_ip(std::net::Ipv4Addr::LOCALHOST.into());
    }
    locale
}
