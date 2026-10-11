//! Lecture du journal (V-01 à V-08) : aucune écriture ici, seulement `lire`.

use domain::journal::{FiltreJournal, TAILLE_PAGE, TAILLE_PAGE_MAX};

use crate::app::App;
use crate::erreur::Resultat;
use crate::ports::PageJournal;

impl App {
    /// Page `page` (à partir de 1) du journal filtré, 50 entrées par défaut.
    pub async fn journal(
        &self,
        filtre: &FiltreJournal,
        page: usize,
        taille: Option<usize>,
    ) -> Resultat<(PageJournal, usize, usize)> {
        let taille = taille.unwrap_or(TAILLE_PAGE).clamp(1, TAILLE_PAGE_MAX);
        let page = page.max(1);
        Ok((
            self.depot.journal(filtre, page, taille).await?,
            page,
            taille,
        ))
    }

    /// Journal complet filtré (export V-07).
    pub async fn journal_complet(&self, filtre: &FiltreJournal) -> Resultat<PageJournal> {
        Ok(self.depot.journal(filtre, 1, usize::MAX).await?)
    }
}
