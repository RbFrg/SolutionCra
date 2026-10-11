import type { KeyboardEvent, ReactNode } from "react";

import { Icone, type NomIcone } from "./Icone";

/** Carte de chiffre-clé, avec barre de progression facultative (21.5). */
export function CarteChiffre({
  libelle,
  valeur,
  legende,
  progression,
}: {
  libelle: string;
  valeur: ReactNode;
  legende?: ReactNode;
  progression?: { valeur: number; libelle: string };
}) {
  return (
    <section
      aria-label={libelle}
      className="flex-1 basis-56 rounded-lg border border-line bg-surf p-5"
    >
      <h2 className="text-xs font-semibold tracking-[0.02em] text-mut uppercase">{libelle}</h2>
      <p className="mt-1 text-[38px] leading-[1.1] font-semibold tracking-[-0.02em]">{valeur}</p>
      {progression && (
        <div
          role="progressbar"
          aria-label={progression.libelle}
          aria-valuemin={0}
          aria-valuemax={100}
          aria-valuenow={Math.round(progression.valeur * 100)}
          className="mt-3 h-2 rounded bg-line"
        >
          <div className="h-2 rounded bg-acc" style={{ width: `${progression.valeur * 100}%` }} />
        </div>
      )}
      {legende && <p className="mt-2 text-[13px] text-ink2">{legende}</p>}
    </section>
  );
}

/** Choix exclusif court : `aria-pressed`, flèches gauche et droite (21.5). */
export function Segmente<T extends string | number>({
  libelle,
  options,
  valeur,
  onChange,
}: {
  libelle: string;
  options: { valeur: T; libelle: string }[];
  valeur: T;
  onChange: (v: T) => void;
}) {
  const auClavier = (e: KeyboardEvent<HTMLButtonElement>, i: number) => {
    const pas = e.key === "ArrowRight" ? 1 : e.key === "ArrowLeft" ? -1 : 0;
    if (!pas) return;
    const suivante = options[(i + pas + options.length) % options.length];
    if (suivante) onChange(suivante.valeur);
  };
  return (
    <div
      role="group"
      aria-label={libelle}
      className="inline-flex overflow-hidden rounded-md border border-line2"
    >
      {options.map((o, i) => {
        const actif = o.valeur === valeur;
        return (
          <button
            key={String(o.valeur)}
            type="button"
            aria-pressed={actif}
            onClick={() => onChange(o.valeur)}
            onKeyDown={(e) => auClavier(e, i)}
            className={`min-h-11 border-line2 px-[18px] text-[15px] not-first:border-l ${
              actif ? "bg-press-bg font-semibold text-press-fg" : "bg-surf text-ink"
            }`}
          >
            {o.libelle}
          </button>
        );
      })}
    </div>
  );
}

const TONS = {
  err: { classes: "border-err bg-err-bg text-err", icone: "retard" },
  warn: { classes: "border-warn bg-warn-bg text-warn", icone: "info" },
  ok: { classes: "border-ok bg-ok-bg text-ok", icone: "faite" },
  info: { classes: "border-acc bg-acc-bg text-acc", icone: "info" },
} as const satisfies Record<string, { classes: string; icone: NomIcone }>;

/** Bandeau d'état : `alert` pour les erreurs, `status` pour les informations. */
export function Bandeau({
  ton,
  children,
  actions,
}: {
  ton: keyof typeof TONS;
  children: ReactNode;
  actions?: ReactNode;
}) {
  const t = TONS[ton];
  return (
    <div
      role={ton === "err" || ton === "warn" ? "alert" : "status"}
      className={`flex flex-wrap items-center gap-3 rounded-lg border p-4 ${t.classes}`}
    >
      <Icone nom={t.icone} />
      <div className="min-w-0 flex-1 text-sm font-medium">{children}</div>
      {actions && <div className="flex flex-wrap gap-2">{actions}</div>}
    </div>
  );
}

export function Carte({
  titre,
  actions,
  children,
}: {
  titre: string;
  actions?: ReactNode;
  children: ReactNode;
}) {
  return (
    <section className="rounded-lg border border-line bg-surf">
      <header className="flex flex-wrap items-center justify-between gap-2 border-b border-line px-4 py-3">
        <h2 className="text-xl font-semibold tracking-[-0.01em]">{titre}</h2>
        {actions}
      </header>
      <div className="p-4">{children}</div>
    </section>
  );
}

export function Chargement({ libelle = "Chargement…" }: { libelle?: string }) {
  return (
    <p role="status" className="text-ink2">
      {libelle}
    </p>
  );
}

export function ErreurChargement({
  erreur,
  reessayer,
}: {
  erreur: unknown;
  reessayer?: () => void;
}) {
  const message = erreur instanceof Error ? erreur.message : "Le chargement a échoué.";
  return (
    <Bandeau
      ton="err"
      actions={
        reessayer && (
          <button
            type="button"
            className="min-h-11 rounded-md border border-err bg-surf px-4 text-sm"
            onClick={reessayer}
          >
            Réessayer
          </button>
        )
      }
    >
      {message}
    </Bandeau>
  );
}

/** Champ avec étiquette visible, aide et erreur reliées (21.5, E-20). */
export function Champ({
  id,
  libelle,
  aide,
  erreur,
  children,
}: {
  id: string;
  libelle: string;
  aide?: string;
  erreur?: string;
  children: ReactNode;
}) {
  return (
    <div className="flex flex-col gap-1.5">
      <label htmlFor={id} className="text-sm font-medium">
        {libelle}
      </label>
      {children}
      {aide && (
        <p id={`${id}-aide`} className="text-[13px] text-ink2">
          {aide}
        </p>
      )}
      {erreur && (
        <p id={`${id}-erreur`} className="flex items-center gap-1 text-[13px] text-err">
          <Icone nom="retard" taille={14} />
          {erreur}
        </p>
      )}
    </div>
  );
}

export const CLASSES_CHAMP =
  "min-h-11 w-full rounded-md border border-line2 bg-surf px-3 text-[15px] text-ink aria-[invalid=true]:border-2 aria-[invalid=true]:border-err";
