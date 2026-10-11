import { screen, waitFor, within } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { http, HttpResponse } from "msw";
import { describe, expect, it } from "vitest";

import { tableauDeBord } from "../../test/fixtures";
import { rendreApp } from "../../test/rendu";
import { serveur } from "../../test/serveur";

describe("Tableau de bord", () => {
  it("affiche les trois indicateurs du serveur", async () => {
    rendreApp();
    const faites = await screen.findByRole("region", { name: "Fiches faites" });
    expect(within(faites).getByText("3 sur 5")).toBeInTheDocument();
    expect(within(faites).getByRole("progressbar")).toHaveAttribute("aria-valuenow", "60");
    expect(
      within(screen.getByRole("region", { name: "En retard" })).getByText("1"),
    ).toBeInTheDocument();
    expect(
      within(screen.getByRole("region", { name: "À faire cette semaine" })).getByText(
        /échéance dimanche 11\/10\/2026/,
      ),
    ).toBeInTheDocument();
  });

  it("les retards viennent du serveur avec créer la fiche et justifier", async () => {
    rendreApp();
    expect(await screen.findByText(/1 fiche en retard\. Semaine 39/)).toBeInTheDocument();
    expect(screen.getAllByRole("button", { name: "Créer la fiche" }).length).toBeGreaterThan(0);
    expect(screen.getByRole("button", { name: "Lever la justification" })).toBeInTheDocument();
  });

  it("le calendrier commence le lundi et repère aujourd'hui", async () => {
    rendreApp();
    const entetes = await screen.findAllByRole("columnheader");
    expect(entetes.map((e) => e.textContent).slice(1, 3)).toEqual(["Lun", "Mar"]);
    const aujourdhui = document.querySelector('[aria-current="date"]');
    expect(aujourdhui?.textContent).toContain("11");
  });

  it("le filtre de type s'applique à tout l'écran", async () => {
    const types: (string | null)[] = [];
    serveur.use(
      http.get("*/api/tableau-de-bord", ({ request }) => {
        types.push(new URL(request.url).searchParams.get("type"));
        return HttpResponse.json(tableauDeBord());
      }),
    );
    rendreApp();
    const groupe = await screen.findByRole("group", { name: "Type de fiche" });
    await userEvent.click(within(groupe).getByRole("button", { name: "Formation" }));
    expect(within(groupe).getByRole("button", { name: "Formation" })).toHaveAttribute(
      "aria-pressed",
      "true",
    );
    expect(types).toContain("formation");
  });

  it("le dialogue de justification se ferme avec Échap et rend le focus", async () => {
    rendreApp();
    const bouton = (await screen.findAllByRole("button", { name: "Justifier" }))[0]!;
    await userEvent.click(bouton);
    expect(
      await screen.findByRole("dialog", { name: "Justifier la semaine 39" }),
    ).toBeInTheDocument();
    await userEvent.keyboard("{Escape}");
    expect(screen.queryByRole("dialog")).not.toBeInTheDocument();
    await waitFor(() => expect(bouton).toHaveFocus());
  });

  it("une erreur de motif Autre s'affiche sous le commentaire", async () => {
    serveur.use(
      http.post("*/api/justifications", () =>
        HttpResponse.json(
          {
            type: "urn:cra:probleme:validation",
            title: "Données invalides",
            status: 422,
            errors: [
              {
                field: "commentaire",
                message: "Précisez le motif : un commentaire est obligatoire pour « Autre ».",
              },
            ],
          },
          { status: 422, headers: { "Content-Type": "application/problem+json" } },
        ),
      ),
    );
    rendreApp();
    await userEvent.click((await screen.findAllByRole("button", { name: "Justifier" }))[0]!);
    const dialogue = await screen.findByRole("dialog");
    await userEvent.click(within(dialogue).getByRole("radio", { name: /Autre/ }));
    await userEvent.click(within(dialogue).getByRole("button", { name: "Justifier la semaine" }));
    const champ = await within(dialogue).findByRole("textbox", { name: "Commentaire" });
    expect(champ).toHaveAttribute("aria-invalid", "true");
    expect(within(dialogue).getByText(/un commentaire est obligatoire/)).toBeInTheDocument();
  });
});
