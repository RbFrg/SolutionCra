// Mise en forme pour l'affichage seulement (N-08) : aucune règle métier ici.

const PARIS = "Europe/Paris";

/** « 2026-10-05 » devient « 05/10/2026 ». */
export function date(iso: string | null | undefined): string {
  if (!iso) return "";
  const [a, m, j] = iso.slice(0, 10).split("-");
  return `${j}/${m}/${a}`;
}

/** « 2026-10-05 » devient « 05/10 ». */
export function jourMois(iso: string): string {
  return date(iso).slice(0, 5);
}

/** Plage d'une semaine : « 05–11/10/2026 » ou « 28/09–04/10/2026 ». */
export function plage(lundi: string, dimanche: string): string {
  const memeMois = lundi.slice(0, 7) === dimanche.slice(0, 7);
  return memeMois
    ? `${lundi.slice(8, 10)}–${date(dimanche)}`
    : `${jourMois(lundi)}–${date(dimanche)}`;
}

/** Instant UTC affiché en heure de Paris : « 11/10/2026 à 14:40 ». */
export function instant(iso: string | null | undefined): string {
  if (!iso) return "";
  const d = new Date(iso);
  const jour = new Intl.DateTimeFormat("fr-FR", {
    timeZone: PARIS,
    day: "2-digit",
    month: "2-digit",
    year: "numeric",
  }).format(d);
  const heure = new Intl.DateTimeFormat("fr-FR", {
    timeZone: PARIS,
    hour: "2-digit",
    minute: "2-digit",
  }).format(d);
  return `${jour} à ${heure}`;
}

/** Heures décimales en français : 8.5 devient « 8,5 h ». */
export function heures(h: number): string {
  return `${String(Math.round(h * 100) / 100).replace(".", ",")} h`;
}

export function pourcentage(taux: number | null | undefined): string {
  return taux === null || taux === undefined ? "—" : `${Math.round(taux * 100)} %`;
}

export function type(t: string | null | undefined): string {
  switch (t) {
    case "entreprise":
      return "Entreprise";
    case "formation":
      return "Formation";
    case "aucune":
      return "Aucune";
    default:
      return "";
  }
}

/** Année scolaire : 2026 devient « 2026-2027 ». */
export function annee(a: number): string {
  return `${a}-${a + 1}`;
}

/** Jours de la semaine à partir du lundi, au format ISO. */
export function joursDeLaSemaine(lundi: string): string[] {
  const debut = new Date(`${lundi}T12:00:00Z`);
  return Array.from({ length: 7 }, (_, i) => {
    const d = new Date(debut);
    d.setUTCDate(debut.getUTCDate() + i);
    return d.toISOString().slice(0, 10);
  });
}
