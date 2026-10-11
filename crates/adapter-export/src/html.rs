//! HTML pour « Copier pour OneNote » (8.1) : un `<table>` à **styles en ligne**,
//! car les classes CSS ne survivent pas au presse-papier.

use std::fmt::Write;

use application::exports::DocumentFiche;
use domain::duree::formater_heures;

const BORDURE: &str = "border:1px solid #9aa0a6;padding:6px 8px;vertical-align:top;text-align:left";

/// Échappe le texte saisi (E2) et conserve ses retours à la ligne (E3).
pub fn echapper(texte: &str) -> String {
    let mut sortie = String::with_capacity(texte.len());
    for c in texte.chars() {
        match c {
            '&' => sortie.push_str("&amp;"),
            '<' => sortie.push_str("&lt;"),
            '>' => sortie.push_str("&gt;"),
            '"' => sortie.push_str("&quot;"),
            '\'' => sortie.push_str("&#39;"),
            '\n' => sortie.push_str("<br>"),
            '\r' => {}
            autre => sortie.push(autre),
        }
    }
    sortie
}

fn cellules(document: &DocumentFiche) -> Vec<[String; 6]> {
    document
        .lignes
        .iter()
        .map(|l| {
            [
                l.date.format("%d/%m/%Y").to_string(),
                echapper(&l.mission),
                l.competences
                    .iter()
                    .map(|c| echapper(c))
                    .collect::<Vec<_>>()
                    .join("<br>"),
                formater_heures(u32::from(l.minutes))
                    .trim_end_matches(" h")
                    .to_owned(),
                echapper(&l.etapes),
                echapper(l.echanges.as_deref().unwrap_or("")),
            ]
        })
        .collect()
}

/// Le fragment d'une fiche : titre, semaine, tableau de six colonnes, total.
pub fn fragment(document: &DocumentFiche) -> String {
    let mut h = String::new();
    let _ = write!(
        h,
        "<h2 style=\"font-family:Calibri,Arial,sans-serif;font-size:16pt;margin:0 0 4px\">{}</h2>",
        echapper(&document.titre())
    );
    let _ = write!(
        h,
        "<p style=\"font-family:Calibri,Arial,sans-serif;font-size:11pt;margin:0 0 8px\">{}</p>",
        echapper(&document.sous_titre())
    );
    if document.demo {
        h.push_str("<p style=\"font-family:Calibri,Arial,sans-serif;font-size:10pt;color:#8a5a00;margin:0 0 8px\">Démonstration, données fictives.</p>");
    }
    h.push_str("<table style=\"border-collapse:collapse;font-family:Calibri,Arial,sans-serif;font-size:11pt\"><thead><tr>");
    for colonne in document.colonnes {
        let _ = write!(
            h,
            "<th style=\"{BORDURE};background-color:#eef0f2;font-weight:bold\">{}</th>",
            echapper(colonne)
        );
    }
    h.push_str("</tr></thead><tbody>");
    for ligne in cellules(document) {
        h.push_str("<tr>");
        for cellule in ligne {
            let _ = write!(h, "<td style=\"{BORDURE}\">{cellule}</td>");
        }
        h.push_str("</tr>");
    }
    h.push_str("</tbody></table>");
    let _ = write!(
        h,
        "<p style=\"font-family:Calibri,Arial,sans-serif;font-size:11pt;font-weight:bold;margin:8px 0 0\">{}</p>",
        echapper(&document.total())
    );
    h
}

/// Document HTML complet, ouvrable seul et collable dans OneNote.
pub fn page(titre: &str, documents: &[DocumentFiche]) -> String {
    let mut h = format!(
        "<!doctype html><html lang=\"fr\"><head><meta charset=\"utf-8\"><title>{}</title></head><body>",
        echapper(titre)
    );
    for (i, document) in documents.iter().enumerate() {
        if i > 0 {
            h.push_str("<hr style=\"margin:24px 0;page-break-after:always\">");
        }
        h.push_str(&fragment(document));
    }
    h.push_str("</body></html>\n");
    h
}

/// Version tabulée pour `text/plain` (8.1).
pub fn texte_tabule(document: &DocumentFiche) -> String {
    let mut t = format!("{}\n{}\n", document.titre(), document.sous_titre());
    t.push_str(&document.colonnes.join("\t"));
    t.push('\n');
    for l in &document.lignes {
        let champs = [
            l.date.format("%d/%m/%Y").to_string(),
            l.mission.clone(),
            l.competences.join(", "),
            formater_heures(u32::from(l.minutes)),
            l.etapes.replace('\n', " "),
            l.echanges.clone().unwrap_or_default().replace('\n', " "),
        ];
        t.push_str(&champs.join("\t"));
        t.push('\n');
    }
    t.push_str(&document.total());
    t.push('\n');
    t
}
