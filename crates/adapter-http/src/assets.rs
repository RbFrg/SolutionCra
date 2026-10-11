//! Port `Assets` (4.2) : fichiers du front lus par chemin. L'implémentation
//! `rust-embed` vit dans `bin/cra-server`, celle des tests est en mémoire.

use std::borrow::Cow;
use std::collections::HashMap;

use axum::body::Body;
use axum::extract::State;
use axum::http::{HeaderValue, Method, StatusCode, Uri, header};
use axum::response::{IntoResponse, Response};

use crate::etat::Etat;
use crate::probleme::Probleme;

pub trait Assets: Send + Sync {
    fn lire(&self, chemin: &str) -> Option<Cow<'static, [u8]>>;
}

/// Assets en mémoire, pour les tests.
#[derive(Debug, Default, Clone)]
pub struct AssetsEnMemoire(HashMap<String, Vec<u8>>);

impl AssetsEnMemoire {
    pub fn avec(mut self, chemin: &str, contenu: &str) -> Self {
        self.0
            .insert(chemin.to_owned(), contenu.as_bytes().to_vec());
        self
    }
}

impl Assets for AssetsEnMemoire {
    fn lire(&self, chemin: &str) -> Option<Cow<'static, [u8]>> {
        self.0.get(chemin).map(|c| Cow::Owned(c.clone()))
    }
}

const CACHE_IMMUABLE: &str = "public, max-age=31536000, immutable";
const SANS_CACHE: &str = "no-cache";

/// Repli de toutes les routes hors `/api` : un fichier du front s'il existe,
/// sinon `index.html` (routage côté client, H10, H11, H13).
pub async fn servir_front(State(etat): State<Etat>, methode: Method, uri: Uri) -> Response {
    if methode != Method::GET && methode != Method::HEAD {
        return Probleme::introuvable("Ressource inconnue.").into_response();
    }
    let chemin = uri.path().trim_start_matches('/');
    if !chemin.is_empty() {
        if let Some(contenu) = etat.assets.lire(chemin) {
            let cache = if chemin.starts_with("assets/") {
                CACHE_IMMUABLE
            } else {
                SANS_CACHE
            };
            return fichier(chemin, contenu, cache);
        }
        if chemin.starts_with("assets/") {
            return Probleme::introuvable("Fichier introuvable.").into_response();
        }
    }
    match etat.assets.lire("index.html") {
        Some(index) => fichier("index.html", index, SANS_CACHE),
        None => (
            StatusCode::SERVICE_UNAVAILABLE,
            "Interface non construite : lancer `npm --prefix web run build`.",
        )
            .into_response(),
    }
}

fn fichier(chemin: &str, contenu: Cow<'static, [u8]>, cache: &'static str) -> Response {
    let type_mime = mime_guess::from_path(chemin).first_or_octet_stream();
    let mut reponse = Response::new(Body::from(contenu.into_owned()));
    let entetes = reponse.headers_mut();
    if let Ok(valeur) = HeaderValue::from_str(type_mime.as_ref()) {
        entetes.insert(header::CONTENT_TYPE, valeur);
    }
    entetes.insert(header::CACHE_CONTROL, HeaderValue::from_static(cache));
    reponse
}
