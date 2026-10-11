import { useSyncExternalStore } from "react";

/** Seuil de la déclinaison mobile (4.3, proposition à confirmer). */
export const REQUETE_MOBILE = "(max-width: 767.98px)";

function abonner(rappel: () => void) {
  if (typeof window === "undefined" || !window.matchMedia) return () => {};
  const m = window.matchMedia(REQUETE_MOBILE);
  m.addEventListener("change", rappel);
  return () => m.removeEventListener("change", rappel);
}

function lire() {
  return typeof window !== "undefined" && !!window.matchMedia?.(REQUETE_MOBILE).matches;
}

/** Vrai sous 768 px : barre d'onglets et feuilles au lieu des dialogues. */
export function useEstMobile(): boolean {
  return useSyncExternalStore(abonner, lire, () => false);
}

export type Theme = "systeme" | "clair" | "sombre";

const CLE_THEME = "cra.theme";

export function lireTheme(): Theme {
  try {
    const t = window.localStorage.getItem(CLE_THEME);
    return t === "clair" || t === "sombre" ? t : "systeme";
  } catch {
    return "systeme";
  }
}

/** Mémorise le thème dans le navigateur et l'applique (21.7, G-07). */
export function appliquerTheme(theme: Theme) {
  try {
    window.localStorage.setItem(CLE_THEME, theme);
  } catch {
    // Stockage indisponible : le choix vaut pour la session.
  }
  if (theme === "systeme") {
    document.documentElement.removeAttribute("data-theme");
  } else {
    document.documentElement.setAttribute("data-theme", theme);
  }
}
