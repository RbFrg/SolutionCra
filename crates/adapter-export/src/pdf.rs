//! PDF minimal sans dépendance : A4 paysage, polices standard Helvetica en
//! WinAnsiEncoding (accents et « œ » compris), tableau à six colonnes avec
//! retour à la ligne, une fiche par page. Le skill proposait `typst` ; ce
//! générateur évite une dépendance lourde tant que le gabarit reste simple.

use std::fmt::Write;

use application::exports::DocumentFiche;
use domain::duree::formater_heures;

const LARGEUR: f32 = 841.89;
const HAUTEUR: f32 = 595.28;
const MARGE: f32 = 36.0;
const TAILLE: f32 = 9.0;
const INTERLIGNE: f32 = 11.0;
const REMBOURRAGE: f32 = 4.0;
/// Parts de la largeur utile par colonne (Date, Mission, Compétences, Durée, Étapes, dernière).
const PARTS: [f32; 6] = [0.08, 0.17, 0.20, 0.07, 0.28, 0.20];

/// Largeurs Helvetica (millièmes d'em) des caractères ASCII 32 à 126.
const LARGEURS_ASCII: [u16; 95] = [
    278, 278, 355, 556, 556, 889, 667, 191, 333, 333, 389, 584, 278, 333, 278, 278, 556, 556, 556,
    556, 556, 556, 556, 556, 556, 556, 278, 278, 584, 584, 584, 556, 1015, 667, 667, 722, 722, 667,
    611, 778, 722, 278, 500, 667, 556, 833, 722, 778, 667, 778, 722, 667, 611, 722, 667, 944, 667,
    667, 611, 278, 278, 278, 469, 556, 333, 556, 556, 500, 556, 556, 278, 556, 556, 222, 222, 500,
    222, 833, 556, 556, 556, 556, 333, 500, 278, 556, 500, 722, 500, 500, 500, 334, 260, 334, 584,
];

/// Caractère vers son octet WinAnsi ; `?` s'il n'existe pas dans le codage.
fn winansi(c: char) -> u8 {
    match c {
        '€' => 0x80,
        '‚' => 0x82,
        '„' => 0x84,
        '…' => 0x85,
        'Œ' => 0x8C,
        '‘' => 0x91,
        '’' => 0x92,
        '“' => 0x93,
        '”' => 0x94,
        '•' => 0x95,
        '–' => 0x96,
        '—' => 0x97,
        'œ' => 0x9C,
        'Ÿ' => 0x9F,
        '\u{a0}'..='\u{ff}' => u8::try_from(u32::from(c)).unwrap_or(b'?'),
        ' '..='~' => u8::try_from(u32::from(c)).unwrap_or(b'?'),
        _ => b'?',
    }
}

/// Largeur approchée d'un caractère : les lettres accentuées prennent celle de leur base.
fn largeur_caractere(c: char) -> u16 {
    let base = match c {
        'à' | 'â' | 'ä' | 'á' | 'ã' | 'å' => 'a',
        'ç' => 'c',
        'é' | 'è' | 'ê' | 'ë' => 'e',
        'î' | 'ï' | 'í' | 'ì' => 'i',
        'ô' | 'ö' | 'ó' | 'ò' | 'õ' => 'o',
        'ù' | 'û' | 'ü' | 'ú' => 'u',
        'ÿ' => 'y',
        'À' | 'Â' | 'Ä' => 'A',
        'Ç' => 'C',
        'É' | 'È' | 'Ê' | 'Ë' => 'E',
        'Î' | 'Ï' => 'I',
        'Ô' | 'Ö' => 'O',
        'Ù' | 'Û' | 'Ü' => 'U',
        'œ' => return 944,
        'Œ' => return 1000,
        '’' | '‘' => return 222,
        '«' | '»' => return 556,
        '–' => return 556,
        '—' | '…' => return 1000,
        '·' => return 278,
        autre => autre,
    };
    let code = u32::from(base);
    if (32..=126).contains(&code) {
        LARGEURS_ASCII[(code - 32) as usize]
    } else {
        556
    }
}

fn largeur(texte: &str, taille: f32, gras: bool) -> f32 {
    let milliemes: u32 = texte.chars().map(|c| u32::from(largeur_caractere(c))).sum();
    #[allow(clippy::cast_precision_loss)]
    let l = milliemes as f32 * taille / 1000.0;
    if gras { l * 1.06 } else { l }
}

/// Coupe un texte en lignes d'au plus `max` points, mot par mot ; un mot
/// trop long est coupé à la lettre.
fn couper(texte: &str, max: f32, gras: bool) -> Vec<String> {
    let mut lignes = Vec::new();
    for paragraphe in texte.split('\n') {
        let mut courante = String::new();
        for mot in paragraphe.split_whitespace() {
            let essai = if courante.is_empty() {
                mot.to_owned()
            } else {
                format!("{courante} {mot}")
            };
            if largeur(&essai, TAILLE, gras) <= max {
                courante = essai;
                continue;
            }
            if !courante.is_empty() {
                lignes.push(std::mem::take(&mut courante));
            }
            for c in mot.chars() {
                if largeur(&format!("{courante}{c}"), TAILLE, gras) > max && !courante.is_empty() {
                    lignes.push(std::mem::take(&mut courante));
                }
                courante.push(c);
            }
        }
        lignes.push(courante);
    }
    lignes
}

fn chaine(texte: &str) -> String {
    let mut s = String::from("(");
    for c in texte.chars() {
        match winansi(c) {
            b'(' => s.push_str("\\("),
            b')' => s.push_str("\\)"),
            b'\\' => s.push_str("\\\\"),
            o if o < 0x80 => s.push(char::from(o)),
            o => {
                let _ = write!(s, "\\{o:03o}");
            }
        }
    }
    s.push(')');
    s
}

/// Contenu d'une page en cours de construction.
struct Page {
    flux: String,
    y: f32,
}

impl Page {
    fn new() -> Self {
        Self {
            flux: String::new(),
            y: HAUTEUR - MARGE,
        }
    }

    fn texte(&mut self, x: f32, y: f32, taille: f32, gras: bool, texte: &str) {
        let police = if gras { "F2" } else { "F1" };
        let _ = writeln!(
            self.flux,
            "BT /{police} {taille:.1} Tf {x:.2} {y:.2} Td {} Tj ET",
            chaine(texte)
        );
    }

    fn cadre(&mut self, x: f32, y: f32, l: f32, h: f32, fond: bool) {
        if fond {
            let _ = writeln!(
                self.flux,
                "0.933 0.941 0.949 rg {x:.2} {y:.2} {l:.2} {h:.2} re f 0 g"
            );
        }
        let _ = writeln!(self.flux, "0.6 G 0.5 w {x:.2} {y:.2} {l:.2} {h:.2} re S");
    }
}

struct Rendu {
    pages: Vec<String>,
}

impl Rendu {
    fn colonnes() -> [f32; 6] {
        let utile = LARGEUR - 2.0 * MARGE;
        PARTS.map(|p| p * utile)
    }

    fn rangee(page: &mut Page, cellules: &[Vec<String>; 6], gras: bool, fond: bool) {
        let hauteur = Self::hauteur(cellules);
        let mut x = MARGE;
        for (i, l) in Self::colonnes().iter().enumerate() {
            page.cadre(x, page.y - hauteur, *l, hauteur, fond);
            let mut y = page.y - REMBOURRAGE - TAILLE;
            for ligne in &cellules[i] {
                page.texte(x + REMBOURRAGE, y, TAILLE, gras, ligne);
                y -= INTERLIGNE;
            }
            x += l;
        }
        page.y -= hauteur;
    }

    fn hauteur(cellules: &[Vec<String>; 6]) -> f32 {
        let lignes = cellules.iter().map(Vec::len).max().unwrap_or(1).max(1);
        #[allow(clippy::cast_precision_loss)]
        let h = lignes as f32 * INTERLIGNE + 2.0 * REMBOURRAGE;
        h
    }

    fn decouper(valeurs: [String; 6], gras: bool) -> [Vec<String>; 6] {
        let colonnes = Self::colonnes();
        let mut i = 0;
        valeurs.map(|v| {
            let lignes = couper(&v, colonnes[i] - 2.0 * REMBOURRAGE, gras);
            i += 1;
            lignes
        })
    }

    fn entete(page: &mut Page, document: &DocumentFiche, suite: bool) {
        let titre = if suite {
            format!("{} (suite)", document.titre())
        } else {
            document.titre()
        };
        page.texte(MARGE, page.y - 16.0, 16.0, true, &titre);
        page.texte(MARGE, page.y - 32.0, 11.0, false, &document.sous_titre());
        if document.demo {
            let mention = "Démonstration, données fictives.";
            let x = LARGEUR - MARGE - largeur(mention, 9.0, false);
            page.texte(x, page.y - 16.0, 9.0, false, mention);
        }
        page.y -= 44.0;
        let entetes = Self::decouper(document.colonnes.map(str::to_owned), true);
        Self::rangee(page, &entetes, true, true);
    }

    fn fiche(&mut self, document: &DocumentFiche) {
        let mut page = Page::new();
        Self::entete(&mut page, document, false);
        for l in &document.lignes {
            let cellules = Self::decouper(
                [
                    l.date.format("%d/%m/%Y").to_string(),
                    l.mission.clone(),
                    l.competences.join("\n"),
                    formater_heures(u32::from(l.minutes))
                        .trim_end_matches(" h")
                        .to_owned(),
                    l.etapes.clone(),
                    l.echanges.clone().unwrap_or_default(),
                ],
                false,
            );
            if page.y - Self::hauteur(&cellules) < MARGE + 20.0 {
                self.pages.push(std::mem::take(&mut page.flux));
                page = Page::new();
                Self::entete(&mut page, document, true);
            }
            Self::rangee(&mut page, &cellules, false, false);
        }
        if page.y - 20.0 < MARGE {
            self.pages.push(std::mem::take(&mut page.flux));
            page = Page::new();
        }
        page.texte(MARGE, page.y - 16.0, 10.0, true, &document.total());
        self.pages.push(page.flux);
    }

    fn page_de_garde(&mut self, titre: &str, documents: &[DocumentFiche]) {
        let mut page = Page::new();
        page.texte(MARGE, HAUTEUR / 2.0 + 20.0, 22.0, true, titre);
        let total: u32 = documents.iter().map(|d| d.total_minutes).sum();
        let resume = format!("{} fiches · {}", documents.len(), formater_heures(total));
        page.texte(MARGE, HAUTEUR / 2.0 - 6.0, 12.0, false, &resume);
        if documents.iter().any(|d| d.demo) {
            page.texte(
                MARGE,
                HAUTEUR / 2.0 - 26.0,
                10.0,
                false,
                "Démonstration, données fictives.",
            );
        }
        self.pages.push(page.flux);
    }

    /// Assemble les objets PDF et la table des références croisées.
    fn assembler(self) -> Vec<u8> {
        let n = self.pages.len();
        // 1 catalogue, 2 pages, 3 et 4 polices, puis une page et son contenu par page.
        let mut objets: Vec<String> = Vec::with_capacity(4 + 2 * n);
        objets.push("<< /Type /Catalog /Pages 2 0 R >>".to_owned());
        let kids: Vec<String> = (0..n).map(|i| format!("{} 0 R", 5 + 2 * i)).collect();
        objets.push(format!(
            "<< /Type /Pages /Kids [{}] /Count {n} >>",
            kids.join(" ")
        ));
        objets.push(
            "<< /Type /Font /Subtype /Type1 /BaseFont /Helvetica /Encoding /WinAnsiEncoding >>"
                .to_owned(),
        );
        objets.push("<< /Type /Font /Subtype /Type1 /BaseFont /Helvetica-Bold /Encoding /WinAnsiEncoding >>".to_owned());
        for (i, flux) in self.pages.iter().enumerate() {
            objets.push(format!(
                "<< /Type /Page /Parent 2 0 R /MediaBox [0 0 {LARGEUR:.2} {HAUTEUR:.2}] /Resources << /Font << /F1 3 0 R /F2 4 0 R >> >> /Contents {} 0 R >>",
                6 + 2 * i
            ));
            objets.push(format!(
                "<< /Length {} >>\nstream\n{flux}endstream",
                flux.len()
            ));
        }
        let mut pdf: Vec<u8> = b"%PDF-1.4\n%\xE2\xE3\xCF\xD3\n".to_vec();
        let mut positions = Vec::with_capacity(objets.len());
        for (i, objet) in objets.iter().enumerate() {
            positions.push(pdf.len());
            pdf.extend_from_slice(format!("{} 0 obj\n{objet}\nendobj\n", i + 1).as_bytes());
        }
        let xref = pdf.len();
        let mut table = format!("xref\n0 {}\n0000000000 65535 f \n", objets.len() + 1);
        for p in positions {
            let _ = writeln!(table, "{p:010} 00000 n ");
        }
        let _ = write!(
            table,
            "trailer\n<< /Size {} /Root 1 0 R >>\nstartxref\n{xref}\n%%EOF\n",
            objets.len() + 1
        );
        pdf.extend_from_slice(table.as_bytes());
        pdf
    }
}

pub fn fiche(document: &DocumentFiche) -> Vec<u8> {
    let mut rendu = Rendu { pages: Vec::new() };
    rendu.fiche(document);
    rendu.assembler()
}

/// Lot : une page de garde, puis une fiche par page (EXP3).
pub fn lot(titre: &str, documents: &[DocumentFiche]) -> Vec<u8> {
    let mut rendu = Rendu { pages: Vec::new() };
    rendu.page_de_garde(titre, documents);
    for document in documents {
        rendu.fiche(document);
    }
    rendu.assembler()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn les_accents_et_les_ligatures_passent_en_winansi() {
        assert_eq!(winansi('é'), 0xE9);
        assert_eq!(winansi('œ'), 0x9C);
        assert_eq!(winansi('É'), 0xC9);
        assert_eq!(
            chaine("Évaluation (œuvre)"),
            "(\\311valuation \\(\\234uvre\\))"
        );
    }

    #[test]
    fn un_texte_long_est_coupe_sans_depasser_la_largeur() {
        let lignes = couper(
            "Mise en place du pipeline d'intégration continue",
            80.0,
            false,
        );
        assert!(lignes.len() > 1);
        assert!(lignes.iter().all(|l| largeur(l, TAILLE, false) <= 80.0));
    }

    #[test]
    fn les_retours_a_la_ligne_sont_conserves() {
        assert_eq!(couper("a\nb", 500.0, false), vec!["a", "b"]);
    }
}
