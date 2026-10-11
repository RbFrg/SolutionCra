import { screen } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { describe, expect, it } from "vitest";

import { rendreApp } from "../../test/rendu";

describe("Paramètres", () => {
  it("la modale change le thème et le mémorise", async () => {
    rendreApp("/?parametres=apparence");
    await userEvent.click(await screen.findByRole("radio", { name: "Sombre" }));
    expect(document.documentElement).toHaveAttribute("data-theme", "sombre");
    expect(window.localStorage.getItem("cra.theme")).toBe("sombre");
    await userEvent.click(screen.getByRole("radio", { name: "Système" }));
    expect(document.documentElement).not.toHaveAttribute("data-theme");
  });

  it("l'onglet missions liste les entrées du référentiel", async () => {
    rendreApp("/?parametres=missions");
    expect(await screen.findByText("Mission fictive A")).toBeInTheDocument();
    expect(screen.getByRole("tab", { name: "Missions" })).toHaveAttribute("aria-selected", "true");
  });
});
