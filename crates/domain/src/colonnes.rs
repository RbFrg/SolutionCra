//! Libellés des six colonnes (18.4), servis par l'API : aucun n'est codé en
//! dur dans le front ni dans un exporteur.

use crate::plan::TypeFiche;

pub fn colonnes(type_fiche: TypeFiche) -> [&'static str; 6] {
    let derniere = match type_fiche {
        TypeFiche::Entreprise => "Évaluation par le tuteur",
        TypeFiche::Formation => "Retour du tuteur / Echanges avec tuteur",
    };
    [
        "Date",
        "Mission",
        "Compétences mises en œuvre (être capable de)",
        "Durée (h)",
        "Etapes de réalisation",
        derniere,
    ]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn colonnes_entreprise_retourne_les_six_libelles() {
        assert_eq!(
            colonnes(TypeFiche::Entreprise),
            [
                "Date",
                "Mission",
                "Compétences mises en œuvre (être capable de)",
                "Durée (h)",
                "Etapes de réalisation",
                "Évaluation par le tuteur",
            ]
        );
    }

    #[test]
    fn colonnes_formation_retourne_les_six_libelles() {
        assert_eq!(
            colonnes(TypeFiche::Formation)[5],
            "Retour du tuteur / Echanges avec tuteur"
        );
    }

    #[test]
    fn colonnes_ne_different_que_par_la_derniere() {
        let e = colonnes(TypeFiche::Entreprise);
        let f = colonnes(TypeFiche::Formation);
        assert_eq!(e[..5], f[..5]);
        assert_ne!(e[5], f[5]);
    }
}
