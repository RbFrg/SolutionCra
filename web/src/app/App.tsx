import { QueryClient, QueryClientProvider } from "@tanstack/react-query";
import { BrowserRouter, Route, Routes } from "react-router";

import { BilanPage } from "../features/bilan/BilanPage";
import { CalendrierPage } from "../features/calendrier/CalendrierPage";
import { FicheEditeurPage } from "../features/fiches/FicheEditeurPage";
import { FichesPage } from "../features/fiches/FichesPage";
import { JournalPage } from "../features/journal/JournalPage";
import { TableauDeBordPage } from "../features/tableau-de-bord/TableauDeBordPage";
import { AppShell } from "./AppShell";

export function creerClient() {
  return new QueryClient({
    defaultOptions: { queries: { retry: 1, refetchOnWindowFocus: true } },
  });
}

export function Routage() {
  return (
    <AppShell>
      <Routes>
        <Route path="/" element={<TableauDeBordPage />} />
        <Route path="/fiches" element={<FichesPage />} />
        <Route path="/fiches/:id" element={<FicheEditeurPage />} />
        <Route path="/calendrier" element={<CalendrierPage />} />
        <Route path="/bilan" element={<BilanPage />} />
        <Route path="/journal" element={<JournalPage />} />
        <Route path="*" element={<TableauDeBordPage />} />
      </Routes>
    </AppShell>
  );
}

export function App({ client = creerClient() }: { client?: QueryClient }) {
  return (
    <QueryClientProvider client={client}>
      <BrowserRouter>
        <Routage />
      </BrowserRouter>
    </QueryClientProvider>
  );
}
