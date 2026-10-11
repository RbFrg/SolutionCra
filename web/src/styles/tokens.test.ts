import { readFileSync } from "node:fs";
import { resolve } from "node:path";
import { describe, expect, it } from "vitest";

const css = readFileSync(resolve(process.cwd(), "src/styles/tokens.css"), "utf8");

function bloc(selecteur: string): string {
  const debut = css.indexOf(selecteur);
  const ouverture = css.indexOf("{", debut);
  const fermeture = css.indexOf("}", ouverture);
  return css
    .slice(ouverture + 1, fermeture)
    .replace(/\s+/g, " ")
    .trim();
}

describe("jetons de couleur", () => {
  it("les deux blocs sombres ont exactement le même contenu", () => {
    expect(bloc(':root:not([data-theme="clair"])')).toBe(bloc(':root[data-theme="sombre"]'));
  });

  it("chaque jeton clair a son équivalent sombre", () => {
    const noms = (texte: string) => [...texte.matchAll(/--[\w-]+/g)].map((m) => m[0]).sort();
    expect(noms(bloc(":root {"))).toEqual(noms(bloc(':root[data-theme="sombre"]')));
  });
});
