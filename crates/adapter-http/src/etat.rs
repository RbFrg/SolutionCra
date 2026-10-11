use std::sync::Arc;

use application::App;

use crate::assets::Assets;

/// État partagé par toutes les routes.
#[derive(Clone)]
pub struct Etat {
    pub app: Arc<App>,
    /// Version du build (SHA du commit), exposée par `/health` et `/api/config`.
    pub build: String,
    pub assets: Arc<dyn Assets>,
    /// Origines autorisées pour les écritures (CSRF, 17.2).
    pub origines_autorisees: Arc<Vec<String>>,
}
