// Données fictives typées par le schéma généré : un changement de contrat
// casse la compilation des tests (7.3).
import type { Schemas } from "../api/client";

type Semaine = Schemas["SemaineDto"];

export function config(mode: Schemas["ModeDto"] = "production"): Schemas["Config"] {
  return mode === "demo"
    ? { mode, dateFigee: "2026-10-11", build: "3a7f9c1e" }
    : { mode, build: "3a7f9c1e" };
}

function semaine(
  lundi: string,
  dimanche: string,
  numero: number,
  valeurs: Partial<Semaine>,
): Semaine {
  return {
    lundi,
    dimanche,
    numero,
    typeAttendu: "entreprise",
    type: "entreprise",
    statut: "a_venir",
    echeance: `${dimanche}T21:59:59Z`,
    ...valeurs,
  };
}

export const semaines: Semaine[] = [
  semaine("2026-09-14", "2026-09-20", 38, {
    statut: "faite",
    ficheId: "f38",
    statutFiche: "transmise",
  }),
  semaine("2026-09-21", "2026-09-27", 39, { statut: "en_retard", joursRetard: 14 }),
  semaine("2026-09-28", "2026-10-04", 40, {
    statut: "justifiee",
    typeAttendu: "formation",
    type: "formation",
    justificationId: "j40",
  }),
  semaine("2026-10-05", "2026-10-11", 41, { statut: "a_faire" }),
  semaine("2026-10-12", "2026-10-18", 42, { statut: "a_venir" }),
];

export function tableauDeBord(
  valeurs: Partial<Schemas["TableauDeBordDto"]> = {},
): Schemas["TableauDeBordDto"] {
  return {
    aujourdhui: "2026-10-11",
    semaineCourante: "2026-10-05",
    annee: 2026,
    indicateurs: {
      attendues: 6,
      faites: 3,
      justifiees: 1,
      enRetard: 1,
      aFaire: 1,
      comptees: 5,
      tauxFaites: 0.6,
      aFaireCetteSemaine: {
        lundi: "2026-10-05",
        numero: 41,
        type: "entreprise",
        echeance: "2026-10-11T21:59:59Z",
      },
    },
    retards: semaines.filter((s) => s.statut === "en_retard"),
    semaines,
    ...valeurs,
  };
}

export const missions: Schemas["EntreeDto"][] = [
  { id: "m1", nom: "Mission fictive A", code: null, archivee: false },
  { id: "m2", nom: "Mission fictive B", code: null, archivee: false },
];

export const competences: Schemas["EntreeDto"][] = [
  { id: "c1", nom: "Tester et valider", code: null, archivee: false },
];

export const motifs: Schemas["EntreeDto"][] = [
  { id: "mo1", nom: "Congés", code: null, archivee: false },
  { id: "mo5", nom: "Autre", code: "autre", archivee: false },
];

export function fiche(valeurs: Partial<Schemas["FicheDto"]> = {}): Schemas["FicheDto"] {
  return {
    id: "f38",
    lundi: "2026-09-14",
    dimanche: "2026-09-20",
    numero: 38,
    type: "entreprise",
    statut: "brouillon",
    modifieeApresTransmission: false,
    version: 3,
    totalHeures: 4,
    colonnes: ["Colonne A", "Colonne B", "Colonne C", "Colonne D", "Colonne E", "Colonne F"],
    lignes: [
      {
        id: "l1",
        date: "2026-09-15",
        missionId: "m1",
        mission: { id: "m1", nom: "Mission fictive A", archivee: false },
        competenceIds: ["c1"],
        competences: [{ id: "c1", nom: "Tester et valider", archivee: false }],
        dureeHeures: 4,
        etapes: "Étape fictive",
        echangesTuteur: null,
      },
    ],
    creeeLe: "2026-09-14T06:30:00Z",
    modifieeLe: "2026-09-22T17:10:00Z",
    ...valeurs,
  };
}
