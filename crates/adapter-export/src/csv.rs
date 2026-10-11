//! CSV (8.3) : UTF-8 avec BOM, séparateur `;`, guillemets RFC 4180, formules
//! de tableur neutralisées (EXP7).

use std::io::{Cursor, Write};

use application::exports::Table;
use zip::write::SimpleFileOptions;

fn cellule(valeur: &str) -> String {
    let valeur = if valeur.starts_with(['=', '+', '-', '@']) {
        format!("'{valeur}")
    } else {
        valeur.to_owned()
    };
    if valeur.contains([';', '"', '\n', '\r']) {
        format!("\"{}\"", valeur.replace('"', "\"\""))
    } else {
        valeur
    }
}

pub fn table(table: &Table) -> Vec<u8> {
    let mut texte = String::from("\u{feff}");
    let entete: Vec<String> = table.colonnes.iter().map(|c| cellule(c)).collect();
    texte.push_str(&entete.join(";"));
    texte.push_str("\r\n");
    for ligne in &table.lignes {
        let cellules: Vec<String> = ligne.iter().map(|c| cellule(c)).collect();
        texte.push_str(&cellules.join(";"));
        texte.push_str("\r\n");
    }
    texte.into_bytes()
}

/// Archive zip : un `<table>.csv` par table, plus `LISEZMOI.txt`.
pub fn archive(tables: &[Table], lisezmoi: &str) -> Result<Vec<u8>, String> {
    let mut zip = zip::ZipWriter::new(Cursor::new(Vec::new()));
    let options = SimpleFileOptions::default().compression_method(zip::CompressionMethod::Deflated);
    let ecrire = |zip: &mut zip::ZipWriter<Cursor<Vec<u8>>>, nom: &str, contenu: &[u8]| {
        zip.start_file(nom, options).map_err(|e| e.to_string())?;
        zip.write_all(contenu).map_err(|e| e.to_string())
    };
    ecrire(&mut zip, "LISEZMOI.txt", lisezmoi.as_bytes())?;
    for t in tables {
        ecrire(&mut zip, &format!("{}.csv", t.nom), &table(t))?;
    }
    zip.finish()
        .map(Cursor::into_inner)
        .map_err(|e| e.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn une_table(lignes: Vec<Vec<&str>>) -> Table {
        Table {
            nom: "t".to_owned(),
            colonnes: vec!["a".to_owned(), "b".to_owned()],
            lignes: lignes
                .into_iter()
                .map(|l| l.into_iter().map(str::to_owned).collect())
                .collect(),
        }
    }

    #[test]
    fn les_csv_neutralisent_les_formules_de_tableur() {
        let csv = String::from_utf8(table(&une_table(vec![vec!["=SOMME(A1)", "@x"]]))).unwrap();
        assert!(csv.contains("'=SOMME(A1);'@x"));
    }

    #[test]
    fn un_csv_commence_par_le_bom_et_echappe_les_guillemets() {
        let csv =
            String::from_utf8(table(&une_table(vec![vec!["dit \"oui\"", "a;b\nc"]]))).unwrap();
        assert!(csv.starts_with('\u{feff}'));
        assert!(csv.contains("\"dit \"\"oui\"\"\";\"a;b\nc\""));
    }
}
