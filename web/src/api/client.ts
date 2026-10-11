import createClient from "openapi-fetch";

import type { components, paths } from "./schema";

export type Schemas = components["schemas"];
export type Probleme = Schemas["Probleme"];

/** Client typé par le contrat généré ; l'API est toujours à l'adresse relative /api. */
export const api = createClient<paths>({
  baseUrl: typeof window === "undefined" ? "" : window.location.origin,
  // Lu à chaque appel : les tests remplacent fetch après la création du client.
  fetch: (requete) => globalThis.fetch(requete),
});

/** Erreur de l'API, au format problem+json (4.1). */
export class ErreurApi extends Error {
  constructor(readonly probleme: Probleme) {
    super(probleme.detail ?? probleme.title);
  }

  /** Message du champ, d'après errors[].field (E-20). */
  champ(nom: string): string | undefined {
    return this.probleme.errors?.find((e) => e.field === nom)?.message;
  }

  get code(): string {
    return this.probleme.type.replace("urn:cra:probleme:", "");
  }
}

/** Déballe une réponse openapi-fetch : la donnée, ou une ErreurApi. */
export function deballer<T>(reponse: { data?: T; error?: unknown; response: Response }): T {
  if (reponse.error !== undefined || reponse.data === undefined) {
    const erreur = reponse.error as Partial<Probleme> | undefined;
    throw new ErreurApi({
      type: erreur?.type ?? "urn:cra:probleme:inconnu",
      title: erreur?.title ?? "Erreur",
      status: erreur?.status ?? reponse.response.status,
      detail: erreur?.detail ?? "Une erreur inattendue s'est produite. Rechargez la page.",
      errors: erreur?.errors ?? [],
    });
  }
  return reponse.data;
}

/** Une réponse vide attendue (204). */
export function verifier(reponse: { error?: unknown; response: Response }): void {
  if (!reponse.response.ok) {
    deballer({ ...reponse, data: undefined });
  }
}
