import { screen, waitFor } from "@testing-library/react";
import { http, HttpResponse } from "msw";
import { describe, expect, it } from "vitest";

import { config, tableauDeBord } from "../test/fixtures";
import { rendreApp } from "../test/rendu";
import { serveur } from "../test/serveur";

describe("AppShell", () => {
  it("affiche la version du build renvoyée par le serveur", async () => {
    rendreApp();
    expect(await screen.findByText("3a7f9c1")).toBeInTheDocument();
  });

  it("le bandeau de démo dépend de la config du serveur", async () => {
    serveur.use(http.get("*/api/config", () => HttpResponse.json(config("demo"))));
    rendreApp();
    expect(
      await screen.findByText("Démonstration, données fictives. Date figée au 11/10/2026."),
    ).toBeInTheDocument();
    expect(screen.getByRole("button", { name: "Réinitialiser les données" })).toBeInTheDocument();
  });

  it("aucun bandeau de démo en production", async () => {
    rendreApp();
    await screen.findByText("3a7f9c1");
    expect(screen.queryByText(/Démonstration, données fictives/)).not.toBeInTheDocument();
  });

  it("le titre de l'onglet affiche le nombre de retards", async () => {
    serveur.use(
      http.get("*/api/tableau-de-bord", () =>
        HttpResponse.json(
          tableauDeBord({ indicateurs: { ...tableauDeBord().indicateurs, enRetard: 2 } }),
        ),
      ),
    );
    rendreApp();
    await waitFor(() => expect(document.title).toBe("(2) Fiches CRA"));
  });

  it("la navigation principale signale la page courante", async () => {
    rendreApp("/");
    const lien = await screen.findByRole("link", { name: "Tableau de bord" });
    expect(lien).toHaveAttribute("aria-current", "page");
  });
});
