// Tracés des icônes des maquettes (trait 1,5, currentColor), décoratives par défaut.

const TRACES = {
  retard: "M8 2.5l6 10.5H2zM8 6.5v3M8 11.2v.1",
  afaire: "M8 2.5a5.5 5.5 0 1 0 0 11 5.5 5.5 0 0 0 0-11zM8 5v3.2l2 1.3",
  faite: "M3 8.5l3.2 3L13 4.5",
  avenir: "M4 8h8",
  justifiee: "M8 2l5 2v4c0 3-2.2 5-5 6-2.8-1-5-3-5-6V4zM5.8 8l1.6 1.6 2.8-3",
  nonatt: "M8 2.5a5.5 5.5 0 1 0 0 11 5.5 5.5 0 0 0 0-11zM4.1 4.1l7.8 7.8",
  brouillon: "M3 13l.8-3L11 2.8 13.2 5 6 12.2zM9.8 4l2.2 2.2",
  terminee: "M8 2.5a5.5 5.5 0 1 0 0 11 5.5 5.5 0 0 0 0-11zM5.6 8.2l1.7 1.7 3.1-3.4",
  transmise: "M2.5 8l11-5-4.5 10-1.8-4.2z",
  validee: "M1.8 8.6l2.8 2.8 5-6M7 10.6l1 .9 5.4-6",
  gear: "M8 5.5a2.5 2.5 0 1 0 0 5 2.5 2.5 0 0 0 0-5zM8 1.8v1.6M8 12.6v1.6M1.8 8h1.6M12.6 8h1.6M3.6 3.6l1.1 1.1M11.3 11.3l1.1 1.1M3.6 12.4l1.1-1.1M11.3 4.7l1.1-1.1",
  plus: "M8 3v10M3 8h10",
  chevL: "M10 3L5 8l5 5",
  chevR: "M6 3l5 5-5 5",
  chevD: "M4 6l4 4 4-4",
  chevU: "M4 10l4-4 4 4",
  trash: "M2.5 4.5h11M6 4.5V3h4v1.5M4 4.5l.7 8.5h6.6l.7-8.5M6.8 7v4M9.2 7v4",
  download: "M8 2.5v8M4.5 7.5L8 11l3.5-3.5M3 13.5h10",
  upload: "M8 11V3M4.5 6.5L8 3l3.5 3.5M3 13.5h10",
  close: "M3.5 3.5l9 9M12.5 3.5l-9 9",
  edit: "M3 13l.8-3L11 2.8 13.2 5 6 12.2z",
  info: "M8 2.5a5.5 5.5 0 1 0 0 11 5.5 5.5 0 0 0 0-11zM8 7.2v3.6M8 5.1v.1",
  cal: "M3 4.5h10V13H3zM3 7h10M5.5 3v3M10.5 3v3",
  list: "M3 4.5h10M3 8h10M3 11.5h10",
  copie: "M5 5h8v8H5zM3 11V3h8",
} as const;

export type NomIcone = keyof typeof TRACES;

export function Icone({
  nom,
  taille = 16,
  libelle,
}: {
  nom: NomIcone;
  taille?: number;
  libelle?: string;
}) {
  return (
    <svg
      width={taille}
      height={taille}
      viewBox="0 0 16 16"
      fill="none"
      stroke="currentColor"
      strokeWidth={1.5}
      strokeLinecap="round"
      strokeLinejoin="round"
      aria-hidden={libelle ? undefined : true}
      role={libelle ? "img" : undefined}
      aria-label={libelle}
      focusable="false"
      className="shrink-0"
    >
      <path d={TRACES[nom]} />
    </svg>
  );
}
