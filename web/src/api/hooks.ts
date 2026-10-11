import { keepPreviousData, useMutation, useQuery, useQueryClient } from "@tanstack/react-query";

import { api, deballer, verifier, type Schemas } from "./client";

export type Filtre = "toutes" | "entreprise" | "formation";
export type NomReferentiel = "missions" | "competences" | "motifs";

export const cles = {
  config: ["config"] as const,
  tableau: (annee: number | undefined, type: Filtre) => ["tableau-de-bord", annee, type] as const,
  plan: (annee: number | undefined) => ["plan", annee] as const,
  fiches: ["fiches"] as const,
  fiche: (id: string) => ["fiches", id] as const,
  referentiel: (nom: NomReferentiel, archivees: boolean) =>
    ["referentiels", nom, archivees] as const,
  journal: (filtre: object) => ["journal", filtre] as const,
  bilan: (annee: number | undefined, type: Filtre) => ["bilan", annee, type] as const,
};

export function useConfig() {
  return useQuery({
    queryKey: cles.config,
    queryFn: async () => deballer(await api.GET("/api/config")),
    staleTime: Infinity,
  });
}

export function useTableauDeBord(annee: number | undefined, type: Filtre) {
  return useQuery({
    queryKey: cles.tableau(annee, type),
    placeholderData: keepPreviousData,
    queryFn: async () =>
      deballer(await api.GET("/api/tableau-de-bord", { params: { query: { annee, type } } })),
  });
}

export function usePlan(annee: number | undefined) {
  return useQuery({
    queryKey: cles.plan(annee),
    placeholderData: keepPreviousData,
    queryFn: async () => deballer(await api.GET("/api/plan", { params: { query: { annee } } })),
  });
}

export function useFiches() {
  return useQuery({
    queryKey: cles.fiches,
    queryFn: async () => deballer(await api.GET("/api/fiches")),
  });
}

export function useFiche(id: string) {
  return useQuery({
    queryKey: cles.fiche(id),
    queryFn: async () => deballer(await api.GET("/api/fiches/{id}", { params: { path: { id } } })),
  });
}

export function useReferentiel(nom: NomReferentiel, archivees = false) {
  return useQuery({
    queryKey: cles.referentiel(nom, archivees),
    queryFn: async () =>
      deballer(
        await api.GET("/api/referentiels/{nom}", {
          params: { path: { nom }, query: { archivees } },
        }),
      ),
  });
}

export function useBilan(annee: number | undefined, type: Filtre) {
  return useQuery({
    queryKey: cles.bilan(annee, type),
    placeholderData: keepPreviousData,
    queryFn: async () =>
      deballer(await api.GET("/api/bilan", { params: { query: { annee, type } } })),
  });
}

export type FiltreJournal = {
  type?: Schemas["TypeEvenementDto"];
  du?: string;
  au?: string;
  page: number;
};

export function useJournal(filtre: FiltreJournal) {
  return useQuery({
    queryKey: cles.journal(filtre),
    placeholderData: keepPreviousData,
    queryFn: async () =>
      deballer(
        await api.GET("/api/journal", {
          params: {
            query: { type: filtre.type, du: filtre.du, au: filtre.au, page: filtre.page },
          },
        }),
      ),
  });
}

/** Toute écriture peut changer les statuts : on invalide tout l'état serveur. */
function useInvalider() {
  const client = useQueryClient();
  return () => client.invalidateQueries({ predicate: (q) => q.queryKey[0] !== "config" });
}

export function useCreerFiche() {
  const invalider = useInvalider();
  return useMutation({
    mutationFn: async (corps: Schemas["CreerFicheDto"]) =>
      deballer(await api.POST("/api/fiches", { body: corps })),
    onSuccess: invalider,
  });
}

export function useSauvegarderFiche(id: string) {
  const client = useQueryClient();
  return useMutation({
    mutationFn: async (corps: Schemas["SauvegarderFicheDto"]) =>
      deballer(await api.PUT("/api/fiches/{id}", { params: { path: { id } }, body: corps })),
    onSuccess: (fiche) => {
      client.setQueryData(cles.fiche(id), fiche);
      void client.invalidateQueries({
        predicate: (q) => q.queryKey[0] !== "config" && q.queryKey[1] !== id,
      });
    },
  });
}

export function useChangerStatut(id: string) {
  const invalider = useInvalider();
  return useMutation({
    mutationFn: async (corps: Schemas["ChangerStatutDto"]) =>
      deballer(await api.PUT("/api/fiches/{id}/statut", { params: { path: { id } }, body: corps })),
    onSuccess: invalider,
  });
}

export function useSupprimerFiche() {
  const invalider = useInvalider();
  return useMutation({
    mutationFn: async (id: string) =>
      verifier(await api.DELETE("/api/fiches/{id}", { params: { path: { id } } })),
    onSuccess: invalider,
  });
}

export function useJustifier() {
  const invalider = useInvalider();
  return useMutation({
    mutationFn: async (corps: Schemas["JustifierDto"]) =>
      deballer(await api.POST("/api/justifications", { body: corps })),
    onSuccess: invalider,
  });
}

export function useLeverJustification() {
  const invalider = useInvalider();
  return useMutation({
    mutationFn: async (id: string) =>
      deballer(await api.DELETE("/api/justifications/{id}", { params: { path: { id } } })),
    onSuccess: invalider,
  });
}

export function useAppliquerPlan() {
  const invalider = useInvalider();
  return useMutation({
    mutationFn: async (corps: Schemas["AppliquerPlanDto"]) =>
      deballer(await api.PUT("/api/plan", { body: corps })),
    onSuccess: invalider,
  });
}

export function useAjouterEntree(nom: NomReferentiel) {
  const invalider = useInvalider();
  return useMutation({
    mutationFn: async (corps: Schemas["AjouterEntreeDto"]) =>
      deballer(
        await api.POST("/api/referentiels/{nom}", { params: { path: { nom } }, body: corps }),
      ),
    onSuccess: invalider,
  });
}

export function useModifierEntree(nom: NomReferentiel) {
  const invalider = useInvalider();
  return useMutation({
    mutationFn: async ({ id, ...corps }: Schemas["ModifierEntreeDto"] & { id: string }) =>
      deballer(
        await api.PATCH("/api/referentiels/{nom}/{id}", {
          params: { path: { nom, id } },
          body: corps,
        }),
      ),
    onSuccess: invalider,
  });
}

export function useImporterReferentiel(nom: NomReferentiel) {
  const invalider = useInvalider();
  return useMutation({
    mutationFn: async ({ csv, mode }: { csv: string; mode: "apercu" | "valider" }) =>
      deballer(
        await api.POST("/api/referentiels/{nom}/import", {
          params: { path: { nom }, query: { mode } },
          body: csv,
          bodySerializer: (corps: string) => corps,
          headers: { "Content-Type": "text/csv" },
        }),
      ),
    onSuccess: (_, { mode }) => {
      if (mode === "valider") void invalider();
    },
  });
}

export function useReinitialiserDemo() {
  const invalider = useInvalider();
  return useMutation({
    mutationFn: async () => verifier(await api.POST("/api/demo/reinitialiser")),
    onSuccess: invalider,
  });
}
