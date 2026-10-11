import { QueryClient, QueryClientProvider } from "@tanstack/react-query";
import { render } from "@testing-library/react";
import type { ReactNode } from "react";
import { MemoryRouter, Route, Routes } from "react-router";

import { Routage } from "../app/App";

/** Rend l'application complète à une adresse donnée, sans réessai réseau. */
export function rendreApp(adresse = "/") {
  const client = new QueryClient({ defaultOptions: { queries: { retry: false } } });
  return render(
    <QueryClientProvider client={client}>
      <MemoryRouter initialEntries={[adresse]}>
        <Routage />
      </MemoryRouter>
    </QueryClientProvider>,
  );
}

/** Rend un composant seul, avec le client de requêtes et un routeur. */
export function rendre(element: ReactNode, adresse = "/") {
  const client = new QueryClient({ defaultOptions: { queries: { retry: false } } });
  return render(
    <QueryClientProvider client={client}>
      <MemoryRouter initialEntries={[adresse]}>
        <Routes>
          <Route path="*" element={element} />
        </Routes>
      </MemoryRouter>
    </QueryClientProvider>,
  );
}
