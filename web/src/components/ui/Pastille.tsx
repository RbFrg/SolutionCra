import type { Schemas } from "../../api/client";
import { Icone, type NomIcone } from "./Icone";

type StatutSemaine = Schemas["StatutSemaineDto"];
type StatutFiche = Schemas["StatutFicheDto"];

type Apparence = { libelle: string; icone: NomIcone; classes: string; glyphe?: string };

/** Statut de semaine **calculé par le serveur** : icône + libellé + couleur (21.5). */
export const SEMAINE: Record<StatutSemaine, Apparence> = {
  faite: { libelle: "Faite", icone: "faite", classes: "text-ok bg-ok-bg border-ok", glyphe: "✓" },
  a_faire: {
    libelle: "À faire",
    icone: "afaire",
    classes: "text-acc bg-acc-bg border-acc",
    glyphe: "●",
  },
  en_retard: {
    libelle: "En retard",
    icone: "retard",
    classes: "text-err bg-err-bg border-err",
    glyphe: "!",
  },
  justifiee: {
    libelle: "Justifiée",
    icone: "justifiee",
    classes: "text-warn bg-warn-bg border-warn",
    glyphe: "J",
  },
  a_venir: {
    libelle: "À venir",
    icone: "avenir",
    classes: "text-mut bg-surf border-line2",
    glyphe: "·",
  },
  non_attendue: {
    libelle: "Non attendue",
    icone: "nonatt",
    classes: "text-mut bg-head border-line2",
    glyphe: "–",
  },
};

export const FICHE: Record<StatutFiche, Apparence> = {
  brouillon: {
    libelle: "Brouillon",
    icone: "brouillon",
    classes: "text-ink2 bg-surf border-line2",
  },
  terminee: { libelle: "Terminée", icone: "terminee", classes: "text-acc bg-acc-bg border-acc" },
  transmise: { libelle: "Transmise", icone: "transmise", classes: "text-acc bg-acc-bg border-acc" },
  validee: { libelle: "Validée", icone: "validee", classes: "text-ok bg-ok-bg border-ok" },
};

type Props =
  | { genre: "semaine"; statut: StatutSemaine; detail?: string }
  | { genre: "fiche"; statut: StatutFiche; detail?: string };

export function Pastille(props: Props) {
  const a = props.genre === "semaine" ? SEMAINE[props.statut] : FICHE[props.statut];
  return (
    <span
      className={`inline-flex items-center gap-1.5 whitespace-nowrap rounded-[14px] border py-0.5 pr-2.5 pl-2 text-[13px] font-medium ${a.classes}`}
    >
      <Icone nom={a.icone} taille={14} />
      {a.libelle}
      {props.detail && <span className="font-normal">· {props.detail}</span>}
    </span>
  );
}
