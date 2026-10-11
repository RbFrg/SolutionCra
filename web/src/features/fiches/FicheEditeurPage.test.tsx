import { act, fireEvent, screen } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { http, HttpResponse } from "msw";
import { afterEach, describe, expect, it, vi } from "vitest";

import type { Schemas } from "../../api/client";
import { fiche } from "../../test/fixtures";
import { rendreApp } from "../../test/rendu";
import { serveur } from "../../test/serveur";
import { DELAI_SAUVEGARDE_MS } from "./FicheEditeurPage";

function probleme(status: number, valeurs: Partial<Schemas["Probleme"]>) {
  return HttpResponse.json(
    { type: "urn:cra:probleme:validation", title: "Erreur", status, ...valeurs },
    { status, headers: { "Content-Type": "application/problem+json" } },
  );
}

describe("Éditeur de fiche", () => {
  afterEach(() => vi.useRealTimers());

  it("affiche les six libellés de colonnes fournis par l'API", async () => {
    rendreApp("/fiches/f38");
    for (const libelle of [
      "Colonne A",
      "Colonne B",
      "Colonne C",
      "Colonne D",
      "Colonne E",
      "Colonne F",
    ]) {
      expect(await screen.findByRole("columnheader", { name: libelle })).toBeInTheDocument();
    }
  });

  it("la saisie déclenche un seul PUT après le délai d'inactivité", async () => {
    const corps: Schemas["SauvegarderFicheDto"][] = [];
    serveur.use(
      http.put("*/api/fiches/f38", async ({ request }) => {
        corps.push((await request.json()) as Schemas["SauvegarderFicheDto"]);
        return HttpResponse.json(fiche({ version: 4 }));
      }),
    );
    rendreApp("/fiches/f38");
    const etapes = await screen.findByRole("textbox", { name: "Étapes de la ligne 1" });
    vi.useFakeTimers({ toFake: ["setTimeout", "clearTimeout"] });
    fireEvent.change(etapes, { target: { value: "Étape fictive modifiée" } });
    fireEvent.change(etapes, { target: { value: "Étape fictive modifiée deux fois" } });
    await act(async () => {
      vi.advanceTimersByTime(DELAI_SAUVEGARDE_MS + 10);
    });
    vi.useRealTimers();
    expect(await screen.findByText("Enregistré, version 4")).toBeInTheDocument();
    expect(corps).toHaveLength(1);
    expect(corps[0]?.version).toBe(3);
    expect(corps[0]?.lignes[0]?.etapes).toBe("Étape fictive modifiée deux fois");
  });

  it("une erreur 422 s'affiche sous le champ concerné", async () => {
    serveur.use(
      http.put("*/api/fiches/f38", () =>
        probleme(422, {
          errors: [
            {
              field: "lignes[0].dureeHeures",
              message: "La durée doit être un multiple de 15 minutes.",
            },
          ],
        }),
      ),
    );
    rendreApp("/fiches/f38");
    const duree = await screen.findByRole("spinbutton", { name: "Durée en heures de la ligne 1" });
    fireEvent.change(duree, { target: { value: "1.1" } });
    expect(
      await screen.findByText(
        "La durée doit être un multiple de 15 minutes.",
        {},
        { timeout: 3000 },
      ),
    ).toBeInTheDocument();
    expect(duree).toHaveAttribute("aria-invalid", "true");
  });

  it("un conflit 409 propose de recharger la fiche", async () => {
    serveur.use(
      http.put("*/api/fiches/f38", () =>
        probleme(409, { type: "urn:cra:probleme:conflit-de-version", versionCourante: 5 }),
      ),
    );
    rendreApp("/fiches/f38");
    const etapes = await screen.findByRole("textbox", { name: "Étapes de la ligne 1" });
    fireEvent.change(etapes, { target: { value: "Autre texte" } });
    expect(
      await screen.findByRole("button", { name: "Recharger la fiche" }, { timeout: 3000 }),
    ).toBeInTheDocument();
  });

  it("marquer terminée sans ligne explique pourquoi", async () => {
    serveur.use(
      http.get("*/api/fiches/f38", () => HttpResponse.json(fiche({ lignes: [], totalHeures: 0 }))),
      http.put("*/api/fiches/f38/statut", () =>
        probleme(422, {
          type: "urn:cra:probleme:terminee-sans-ligne",
          detail: "Ajoutez au moins une ligne avant de terminer la fiche.",
        }),
      ),
    );
    rendreApp("/fiches/f38");
    const menu = await screen.findByRole("combobox", { name: "Statut de la fiche" });
    await userEvent.selectOptions(menu, "terminee");
    expect(await screen.findByRole("alert")).toHaveTextContent(
      "Ajoutez au moins une ligne avant de terminer la fiche.",
    );
  });

  it("l'indicateur modifiée après transmission est affiché", async () => {
    serveur.use(
      http.get("*/api/fiches/f38", () =>
        HttpResponse.json(
          fiche({
            statut: "transmise",
            transmiseLe: "2026-09-14",
            modifieeApresTransmission: true,
            modifieeApresTransmissionLe: "2026-10-11T12:40:00Z",
          }),
        ),
      ),
    );
    rendreApp("/fiches/f38");
    expect(
      await screen.findByText("Modifiée après transmission, le 11/10/2026 à 14:40"),
    ).toBeInTheDocument();
  });

  it("la durée se saisit en heures décimales au quart d'heure", async () => {
    rendreApp("/fiches/f38");
    const duree = await screen.findByRole("spinbutton", { name: "Durée en heures de la ligne 1" });
    expect(duree).toHaveAttribute("step", "0.25");
    expect(duree).toHaveValue(4);
  });

  it("copier pour OneNote propose un aperçu si le presse-papier est refusé", async () => {
    Object.defineProperty(navigator, "clipboard", {
      configurable: true,
      value: { write: () => Promise.reject(new Error("refusé")) },
    });
    vi.stubGlobal("ClipboardItem", class {});
    serveur.use(http.get("*/api/fiches/f38/export", () => HttpResponse.text("<table></table>")));
    rendreApp("/fiches/f38");
    await userEvent.click(await screen.findByRole("button", { name: "Copier pour OneNote" }));
    expect(await screen.findByTitle("Aperçu de la fiche")).toHaveAttribute("sandbox", "");
    vi.unstubAllGlobals();
  });
});
