//! Front React embarqué dans le binaire (4.2). En debug, `rust-embed` lit le
//! dossier à l'exécution ; en release, les fichiers sont dans le binaire.

use std::borrow::Cow;

use adapter_http::Assets;

#[derive(rust_embed::Embed)]
#[folder = "../../web/dist/"]
#[exclude = ".gitkeep"]
struct Dist;

#[derive(Debug, Default, Clone, Copy)]
pub struct FrontEmbarque;

impl Assets for FrontEmbarque {
    fn lire(&self, chemin: &str) -> Option<Cow<'static, [u8]>> {
        Dist::get(chemin).map(|f| f.data)
    }
}
