//! E1 à E6, EXP3, EXP11, DEM8 : rendus d'une fiche et d'un lot.

#![allow(clippy::unwrap_used, clippy::expect_used)]

use std::io::Read;

use adapter_export::Exports;
use application::exports::{DocumentFiche, Exporteur, FormatDocument, LigneDocument};
use chrono::NaiveDate;
use domain::colonnes::colonnes;
use domain::{Lundi, TypeFiche};

fn d(m: u32, j: u32) -> NaiveDate {
    NaiveDate::from_ymd_opt(2026, m, j).unwrap()
}

fn document(type_fiche: TypeFiche, demo: bool) -> DocumentFiche {
    DocumentFiche {
        type_fiche,
        lundi: Lundi::new(d(9, 14)).unwrap(),
        colonnes: colonnes(type_fiche),
        lignes: vec![
            LigneDocument {
                date: d(9, 15),
                mission: "Mise en place du pipeline d'intégration continue".to_owned(),
                competences: vec![
                    "Déployer et exploiter".to_owned(),
                    "Tester et valider".to_owned(),
                ],
                minutes: 240,
                etapes: "1. Écrire le workflow\n2. Ajouter le cache Cargo".to_owned(),
                echanges: Some("Mise en œuvre validée".to_owned()),
            },
            LigneDocument {
                date: d(9, 16),
                mission: "Écriture des tests <domaine> & \"CI\"".to_owned(),
                competences: vec![],
                minutes: 150,
                etapes: String::new(),
                echanges: None,
            },
        ],
        total_minutes: 390,
        demo,
    }
}

fn vide(type_fiche: TypeFiche) -> DocumentFiche {
    DocumentFiche {
        lignes: vec![],
        total_minutes: 0,
        ..document(type_fiche, false)
    }
}

fn html(doc: &DocumentFiche) -> String {
    String::from_utf8(Exports.fiche(doc, FormatDocument::Html).unwrap()).unwrap()
}

#[test]
fn html_fiche_entreprise() {
    insta::assert_snapshot!(html(&document(TypeFiche::Entreprise, false)));
}

#[test]
fn html_fiche_formation() {
    insta::assert_snapshot!(html(&document(TypeFiche::Formation, false)));
}

#[test]
fn html_echappe_les_caracteres_speciaux() {
    let h = html(&document(TypeFiche::Entreprise, false));
    assert!(h.contains("Écriture des tests &lt;domaine&gt; &amp; &quot;CI&quot;"));
    assert!(!h.contains("<domaine>"));
}

#[test]
fn html_conserve_les_retours_a_la_ligne_des_cellules() {
    assert!(
        html(&document(TypeFiche::Entreprise, false))
            .contains("1. Écrire le workflow<br>2. Ajouter le cache Cargo")
    );
}

#[test]
fn le_html_onenote_a_les_six_colonnes_dans_l_ordre_v3() {
    let h = html(&document(TypeFiche::Formation, false));
    let positions: Vec<usize> = colonnes(TypeFiche::Formation)
        .iter()
        .map(|c| h.find(&format!(">{c}</th>")).unwrap())
        .collect();
    assert!(positions.windows(2).all(|p| p[0] < p[1]));
    assert!(!h.contains("class="));
}

#[test]
fn fiche_sans_ligne_est_exportable_en_gabarit_vide() {
    let h = html(&vide(TypeFiche::Entreprise));
    assert!(h.contains("Évaluation par le tuteur"));
    assert!(h.contains("Total : 0 h"));
    assert!(
        Exports
            .fiche(&vide(TypeFiche::Entreprise), FormatDocument::Docx)
            .is_ok()
    );
    assert!(
        Exports
            .fiche(&vide(TypeFiche::Entreprise), FormatDocument::Pdf)
            .unwrap()
            .starts_with(b"%PDF-")
    );
}

fn document_xml(octets: Vec<u8>) -> String {
    let mut archive = zip::ZipArchive::new(std::io::Cursor::new(octets)).unwrap();
    let mut xml = String::new();
    archive
        .by_name("word/document.xml")
        .unwrap()
        .read_to_string(&mut xml)
        .unwrap();
    xml
}

#[test]
fn docx_contient_six_cellules_par_ligne_et_les_en_tetes() {
    let xml = document_xml(
        Exports
            .fiche(
                &document(TypeFiche::Entreprise, false),
                FormatDocument::Docx,
            )
            .unwrap(),
    );
    for colonne in colonnes(TypeFiche::Entreprise) {
        assert!(xml.contains(colonne), "{colonne}");
    }
    let rangees = xml.matches("<w:tr>").count() + xml.matches("<w:tr ").count();
    let cellules = xml.matches("<w:tc>").count() + xml.matches("<w:tc ").count();
    assert_eq!(rangees, 3);
    assert_eq!(cellules, 18);
}

#[test]
fn pdf_est_non_vide_et_commence_par_pdf_header() {
    let pdf = Exports
        .fiche(&document(TypeFiche::Entreprise, false), FormatDocument::Pdf)
        .unwrap();
    assert!(pdf.starts_with(b"%PDF-"));
    assert!(pdf.ends_with(b"%%EOF\n"));
    assert!(pdf.len() > 1000);
}

#[test]
fn exports_conservent_accents_et_ligatures() {
    let doc = document(TypeFiche::Entreprise, false);
    let h = html(&doc);
    assert!(h.contains("Évaluation") && h.contains("œuvre"));
    let xml = document_xml(Exports.fiche(&doc, FormatDocument::Docx).unwrap());
    assert!(xml.contains("Évaluation") && xml.contains("œuvre"));
    // PDF en WinAnsi : « É » s'écrit \311 et « œ » \234.
    let pdf =
        String::from_utf8_lossy(&Exports.fiche(&doc, FormatDocument::Pdf).unwrap()).into_owned();
    assert!(pdf.contains("\\311valuation") && pdf.contains("\\234uvre"));
}

#[test]
fn le_lot_met_une_fiche_par_page_ou_par_section() {
    let docs = vec![
        document(TypeFiche::Entreprise, false),
        document(TypeFiche::Formation, false),
    ];
    let pdf = String::from_utf8_lossy(
        &Exports
            .lot("Fiches CRA · septembre 2026", &docs, FormatDocument::Pdf)
            .unwrap(),
    )
    .into_owned();
    // Page de garde + une page par fiche.
    assert!(pdf.contains("/Count 3"));
    let xml = document_xml(Exports.lot("Lot", &docs, FormatDocument::Docx).unwrap());
    assert_eq!(xml.matches("w:type=\"page\"").count(), 2);
}

#[test]
fn les_exports_de_la_demo_portent_la_mention_demonstration() {
    let doc = document(TypeFiche::Entreprise, true);
    assert!(html(&doc).contains("Démonstration"));
    assert!(
        document_xml(Exports.fiche(&doc, FormatDocument::Docx).unwrap()).contains("Démonstration")
    );
    let pdf =
        String::from_utf8_lossy(&Exports.fiche(&doc, FormatDocument::Pdf).unwrap()).into_owned();
    assert!(pdf.contains("D\\351monstration"));
}
