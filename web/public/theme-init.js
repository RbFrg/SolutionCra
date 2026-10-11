// Thème choisi dans Paramètres > Apparence (21.7) : appliqué avant le premier
// rendu pour éviter un flash. « systeme » : aucun attribut, les media queries décident.
(function () {
  try {
    var choix = window.localStorage.getItem("cra.theme");
    if (choix === "clair" || choix === "sombre") {
      document.documentElement.setAttribute("data-theme", choix);
    }
  } catch (e) {
    // Stockage indisponible : le thème du système s'applique.
  }
})();
