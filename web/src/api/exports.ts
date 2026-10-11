// Adresses des exports (GET, fichiers) : le HTML vient toujours du serveur (8.1).

export type FormatFiche = "html" | "docx" | "pdf";

export function urlExportFiche(id: string, format: FormatFiche): string {
  return `/api/fiches/${encodeURIComponent(id)}/export?format=${format}`;
}

export function urlLot(
  periode: "mois" | "annee",
  valeur: string,
  format: "docx" | "pdf",
  type = "toutes",
) {
  const q = new URLSearchParams({ periode, valeur, format, type });
  return `/api/export/lot?${q.toString()}`;
}

export function urlExportComplet(format: "csv" | "json"): string {
  return `/api/export/complet?format=${format}`;
}

export function urlBilanCsv(tableau: string, annee: number, type: string): string {
  return `/api/bilan/${tableau}.csv?annee=${annee}&type=${type}`;
}

export function urlJournalExport(format: "csv" | "json"): string {
  return `/api/journal/export?format=${format}`;
}

async function texte(url: string): Promise<string> {
  const reponse = await globalThis.fetch(url, { credentials: "same-origin" });
  if (!reponse.ok) throw new Error("Le serveur n'a pas fourni l'export.");
  return reponse.text();
}

/** Copie la fiche en HTML (et en texte) dans le presse-papier, dans le geste
 *  de l'utilisateur : la requête se fait à l'intérieur des promesses (8.1). */
export async function copierPourOneNote(id: string): Promise<void> {
  const html = texte(urlExportFiche(id, "html"));
  const brut = html.then((h) => {
    const doc = new DOMParser().parseFromString(h, "text/html");
    return new Blob([doc.body.textContent ?? ""], { type: "text/plain" });
  });
  await navigator.clipboard.write([
    new ClipboardItem({
      "text/html": html.then((h) => new Blob([h], { type: "text/html" })),
      "text/plain": brut,
    }),
  ]);
}
