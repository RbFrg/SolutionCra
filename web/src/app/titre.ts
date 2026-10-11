import { useEffect } from "react";

/** « (2) Fiches CRA » quand deux semaines sont en retard (T-14). */
export function useTitreOnglet(retards: number | undefined) {
  useEffect(() => {
    document.title = retards ? `(${retards}) Fiches CRA` : "Fiches CRA";
  }, [retards]);
}
