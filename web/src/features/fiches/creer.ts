import { useNavigate } from "react-router";

import { ErreurApi, type Schemas } from "../../api/client";
import { useCreerFiche } from "../../api/hooks";

/** Crée la fiche d'une semaine puis l'ouvre ; une fiche existante est ouverte (L-10). */
export function useCreerEtOuvrir() {
  const creer = useCreerFiche();
  const naviguer = useNavigate();
  const lancer = (corps: Schemas["CreerFicheDto"]) =>
    creer.mutate(corps, {
      onSuccess: (fiche) => void naviguer(`/fiches/${fiche.id}`),
      onError: (e) => {
        if (e instanceof ErreurApi && e.code === "fiche-existante" && e.probleme.ficheId) {
          void naviguer(`/fiches/${e.probleme.ficheId}`);
        }
      },
    });
  return { lancer, mutation: creer };
}
