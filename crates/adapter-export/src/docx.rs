//! Word (8.1) : tableau de six colonnes en paysage, une fiche par section.

use std::io::Cursor;

use application::exports::DocumentFiche;
use docx_rs::{
    BreakType, Docx, PageOrientationType, Paragraph, Run, RunFonts, Shading, Table, TableCell,
    TableRow, WidthType,
};
use domain::duree::formater_heures;

/// Largeurs des colonnes en vingtièmes de point (A4 paysage, marges comprises).
const LARGEURS: [usize; 6] = [1300, 2300, 2700, 900, 3500, 2900];

fn run(texte: &str) -> Run {
    Run::new()
        .add_text(texte)
        .fonts(RunFonts::new().ascii("Calibri").hi_ansi("Calibri"))
        .size(20)
}

/// Un paragraphe par ligne de texte : les retours à la ligne sont conservés.
fn cellule(texte: &str, entete: bool) -> TableCell {
    let mut c = TableCell::new();
    for morceau in texte.split('\n') {
        let r = if entete {
            run(morceau).bold()
        } else {
            run(morceau)
        };
        c = c.add_paragraph(Paragraph::new().add_run(r));
    }
    if entete {
        c = c.shading(Shading::new().fill("EEF0F2"));
    }
    c
}

fn tableau(document: &DocumentFiche) -> Table {
    let mut lignes = vec![TableRow::new(
        document.colonnes.iter().map(|c| cellule(c, true)).collect(),
    )];
    for l in &document.lignes {
        let valeurs = [
            l.date.format("%d/%m/%Y").to_string(),
            l.mission.clone(),
            l.competences.join("\n"),
            formater_heures(u32::from(l.minutes))
                .trim_end_matches(" h")
                .to_owned(),
            l.etapes.clone(),
            l.echanges.clone().unwrap_or_default(),
        ];
        lignes.push(TableRow::new(
            valeurs.iter().map(|v| cellule(v, false)).collect(),
        ));
    }
    Table::new(lignes)
        .set_grid(LARGEURS.to_vec())
        .width(LARGEURS.iter().sum(), WidthType::Dxa)
}

fn ajouter_fiche(mut docx: Docx, document: &DocumentFiche) -> Docx {
    docx = docx
        .add_paragraph(Paragraph::new().add_run(run(&document.titre()).bold().size(32)))
        .add_paragraph(Paragraph::new().add_run(run(&document.sous_titre()).size(22)));
    if document.demo {
        docx = docx.add_paragraph(
            Paragraph::new().add_run(
                run("Démonstration, données fictives.")
                    .italic()
                    .color("8A5A00"),
            ),
        );
    }
    docx.add_table(tableau(document))
        .add_paragraph(Paragraph::new().add_run(run(&document.total()).bold()))
}

fn emballer(docx: Docx) -> Result<Vec<u8>, String> {
    let mut sortie = Cursor::new(Vec::new());
    docx.build()
        .pack(&mut sortie)
        .map_err(|e| format!("docx : {e}"))?;
    Ok(sortie.into_inner())
}

fn nouveau() -> Docx {
    // A4 paysage : 29,7 × 21 cm en vingtièmes de point.
    Docx::new()
        .page_size(16838, 11906)
        .page_orient(PageOrientationType::Landscape)
        .page_margin(
            docx_rs::PageMargin::new()
                .top(720)
                .bottom(720)
                .left(720)
                .right(720),
        )
}

pub fn fiche(document: &DocumentFiche) -> Result<Vec<u8>, String> {
    emballer(ajouter_fiche(nouveau(), document))
}

/// Lot : une fiche par section, séparées par un saut de page (EXP3).
pub fn lot(titre: &str, documents: &[DocumentFiche]) -> Result<Vec<u8>, String> {
    let mut docx = nouveau().add_paragraph(Paragraph::new().add_run(run(titre).bold().size(28)));
    for document in documents {
        docx = docx.add_paragraph(Paragraph::new().add_run(Run::new().add_break(BreakType::Page)));
        docx = ajouter_fiche(docx, document);
    }
    emballer(docx)
}
