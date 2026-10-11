import "@testing-library/jest-dom/vitest";

import { cleanup } from "@testing-library/react";
import { afterAll, afterEach, beforeAll } from "vitest";

import { serveur } from "./serveur";

// Un appel d'API non prévu fait échouer le test (7.6).
beforeAll(() => serveur.listen({ onUnhandledRequest: "error" }));
afterEach(() => {
  serveur.resetHandlers();
  cleanup();
  window.localStorage.clear();
  document.documentElement.removeAttribute("data-theme");
});
afterAll(() => serveur.close());

// jsdom n'implémente pas matchMedia : bureau par défaut.
if (!window.matchMedia) {
  Object.defineProperty(window, "matchMedia", {
    writable: true,
    value: (requete: string) => ({
      matches: false,
      media: requete,
      onchange: null,
      addEventListener: () => {},
      removeEventListener: () => {},
      addListener: () => {},
      removeListener: () => {},
      dispatchEvent: () => false,
    }),
  });
}
