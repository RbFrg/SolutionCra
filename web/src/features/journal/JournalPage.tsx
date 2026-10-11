import { useState } from "react";

import type { Schemas } from "../../api/client";
import { urlJournalExport } from "../../api/exports";
import { useJournal, type FiltreJournal } from "../../api/hooks";
import { Bouton } from "../../components/ui/Bouton";
import { Chargement, CLASSES_CHAMP, ErreurChargement } from "../../components/ui/divers";
import { Icone } from "../../components/ui/Icone";
import { instant } from "../../lib/format";

const TYPES: { valeur: Schemas["TypeEvenementDto"]; libelle: string }[] = [
  { valeur: "fiche_creee", libelle: "Fiche créée" },
  { valeur: "statut_modifie", libelle: "Statut modifié" },
  { valeur: "fiche_modifiee_apres_transmission", libelle: "Fiche modifiée après transmission" },
  { valeur: "fiche_supprimee", libelle: "Fiche supprimée" },
  { valeur: "semaine_justifiee", libelle: "Semaine justifiée" },
  { valeur: "justification_levee", libelle: "Justification levée" },
  { valeur: "calendrier_modifie", libelle: "Calendrier modifié" },
  { valeur: "referentiel_modifie", libelle: "Référentiel modifié" },
  { valeur: "import", libelle: "Import" },
  { valeur: "export", libelle: "Export" },
];

function Entree({ e }: { e: Schemas["EvenementDto"] }) {
  const [ouvert, setOuvert] = useState(false);
  const detail = e.avant != null || e.apres != null;
  return (
    <li className="border-b border-line py-3 last:border-0">
      <div className="flex flex-wrap items-center gap-x-4 gap-y-1">
        <span className="font-mono text-sm text-ink2">{instant(e.horodatage)}</span>
        <span className="font-medium">{e.libelle}</span>
        <span className="min-w-0 flex-1 text-ink2">{e.resume}</span>
        {detail && (
          <button
            type="button"
            aria-expanded={ouvert}
            onClick={() => setOuvert((o) => !o)}
            className="inline-flex min-h-11 items-center gap-1 text-sm text-acc"
          >
            <Icone nom={ouvert ? "chevU" : "chevD"} /> Avant / Après
          </button>
        )}
      </div>
      {ouvert && (
        <div className="mt-2 grid gap-2 md:grid-cols-2">
          {(["avant", "apres"] as const).map((cote) => (
            <div key={cote}>
              <p className="text-xs font-semibold text-mut uppercase">
                {cote === "avant" ? "Avant" : "Après"}
              </p>
              <pre className="overflow-x-auto rounded-md border border-line bg-head p-2 font-mono text-[13px] whitespace-pre-wrap">
                {e[cote] == null ? "—" : JSON.stringify(e[cote], null, 2)}
              </pre>
            </div>
          ))}
        </div>
      )}
    </li>
  );
}

/** Journal des événements, en lecture seule (V-01 à V-08). */
export function JournalPage() {
  const [filtre, setFiltre] = useState<FiltreJournal>({ page: 1 });
  const requete = useJournal(filtre);
  const j = requete.data;

  return (
    <>
      <div className="flex flex-wrap items-end justify-between gap-4">
        <h1 className="text-2xl font-semibold tracking-[-0.02em] md:text-[30px]">Journal</h1>
        <div className="flex gap-2">
          <a
            href={urlJournalExport("csv")}
            className="inline-flex min-h-11 items-center gap-2 rounded-md border border-line2 bg-surf px-4 text-sm font-medium"
          >
            <Icone nom="download" /> CSV
          </a>
          <a
            href={urlJournalExport("json")}
            className="inline-flex min-h-11 items-center gap-2 rounded-md border border-line2 bg-surf px-4 text-sm font-medium"
          >
            <Icone nom="download" /> JSON
          </a>
        </div>
      </div>
      <div className="flex flex-wrap items-end gap-3">
        <label className="flex flex-col gap-1.5 text-sm font-medium">
          Type
          <select
            value={filtre.type ?? ""}
            onChange={(e) =>
              setFiltre({
                ...filtre,
                page: 1,
                type: (e.target.value || undefined) as FiltreJournal["type"],
              })
            }
            className={CLASSES_CHAMP}
          >
            <option value="">Tous les événements</option>
            {TYPES.map((t) => (
              <option key={t.valeur} value={t.valeur}>
                {t.libelle}
              </option>
            ))}
          </select>
        </label>
        <label className="flex flex-col gap-1.5 text-sm font-medium">
          Du
          <input
            type="date"
            value={filtre.du ?? ""}
            onChange={(e) => setFiltre({ ...filtre, page: 1, du: e.target.value || undefined })}
            className={CLASSES_CHAMP}
          />
        </label>
        <label className="flex flex-col gap-1.5 text-sm font-medium">
          Au
          <input
            type="date"
            value={filtre.au ?? ""}
            onChange={(e) => setFiltre({ ...filtre, page: 1, au: e.target.value || undefined })}
            className={CLASSES_CHAMP}
          />
        </label>
      </div>
      {requete.isPending && <Chargement />}
      {requete.isError && (
        <ErreurChargement erreur={requete.error} reessayer={() => void requete.refetch()} />
      )}
      {j && (
        <section aria-label="Événements" className="rounded-lg border border-line bg-surf px-4">
          {j.entrees.length === 0 ? (
            <p className="py-4 text-ink2">Aucun événement pour ces filtres.</p>
          ) : (
            <ol>
              {j.entrees.map((e) => (
                <Entree key={e.id} e={e} />
              ))}
            </ol>
          )}
        </section>
      )}
      {j && j.total > 0 && (
        <nav aria-label="Pagination" className="flex flex-wrap items-center gap-3">
          <Bouton
            icone="chevL"
            disabled={j.page <= 1}
            onClick={() => setFiltre({ ...filtre, page: j.page - 1 })}
          >
            Précédents
          </Bouton>
          <span className="font-mono text-sm">
            {(j.page - 1) * j.taille + 1}–{Math.min(j.page * j.taille, j.total)} sur {j.total}
          </span>
          <Bouton
            disabled={j.page >= j.pages}
            onClick={() => setFiltre({ ...filtre, page: j.page + 1 })}
          >
            Suivants <Icone nom="chevR" />
          </Bouton>
        </nav>
      )}
    </>
  );
}
