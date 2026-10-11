//! Rendus des exports (section 8) : HTML pour OneNote, Word, PDF, CSV et zip.

pub mod csv;
pub mod docx;
pub mod html;
pub mod pdf;

use application::exports::{DocumentFiche, Exporteur, FormatDocument, Table};

/// Implémentation du port `Exporteur`.
#[derive(Debug, Default, Clone, Copy)]
pub struct Exports;

impl Exporteur for Exports {
    fn fiche(&self, document: &DocumentFiche, format: FormatDocument) -> Result<Vec<u8>, String> {
        match format {
            FormatDocument::Html => {
                Ok(html::page(&document.titre(), std::slice::from_ref(document)).into_bytes())
            }
            FormatDocument::Docx => docx::fiche(document),
            FormatDocument::Pdf => Ok(pdf::fiche(document)),
        }
    }

    fn lot(
        &self,
        titre: &str,
        documents: &[DocumentFiche],
        format: FormatDocument,
    ) -> Result<Vec<u8>, String> {
        match format {
            FormatDocument::Html => Ok(html::page(titre, documents).into_bytes()),
            FormatDocument::Docx => docx::lot(titre, documents),
            FormatDocument::Pdf => Ok(pdf::lot(titre, documents)),
        }
    }

    fn csv(&self, table: &Table) -> Vec<u8> {
        csv::table(table)
    }

    fn archive_csv(&self, tables: &[Table], lisezmoi: &str) -> Result<Vec<u8>, String> {
        csv::archive(tables, lisezmoi)
    }
}
