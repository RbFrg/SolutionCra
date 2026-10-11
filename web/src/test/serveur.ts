import { http, HttpResponse } from "msw";
import { setupServer } from "msw/node";

import { config, fiche, missions, competences, motifs, tableauDeBord } from "./fixtures";

/** Réponses par défaut, typées par le contrat ; chaque test peut les remplacer. */
export const gestionnaires = [
  http.get("*/api/config", () => HttpResponse.json(config())),
  http.get("*/api/tableau-de-bord", () => HttpResponse.json(tableauDeBord())),
  http.get("*/api/fiches/:id", () => HttpResponse.json(fiche())),
  http.get("*/api/referentiels/missions", () => HttpResponse.json(missions)),
  http.get("*/api/referentiels/competences", () => HttpResponse.json(competences)),
  http.get("*/api/referentiels/motifs", () => HttpResponse.json(motifs)),
];

export const serveur = setupServer(...gestionnaires);
