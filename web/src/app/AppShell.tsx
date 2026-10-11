import { useState, type ReactNode } from "react";
import { NavLink, useLocation, useSearchParams } from "react-router";

import { useConfig, useReinitialiserDemo, useTableauDeBord } from "../api/hooks";
import { Icone, type NomIcone } from "../components/ui/Icone";
import { Dialogue } from "../components/ui/Dialogue";
import { ParametresDialog } from "../features/parametres/ParametresDialog";
import { date } from "../lib/format";
import { useEstMobile } from "../lib/media";
import { useTitreOnglet } from "./titre";

const LIENS: { href: string; libelle: string; icone: NomIcone }[] = [
  { href: "/", libelle: "Tableau de bord", icone: "afaire" },
  { href: "/fiches", libelle: "Fiches", icone: "list" },
  { href: "/calendrier", libelle: "Calendrier", icone: "cal" },
  { href: "/bilan", libelle: "Bilan", icone: "faite" },
  { href: "/journal", libelle: "Journal", icone: "edit" },
];

/** Bandeau permanent du mode démo, piloté par GET /api/config (21.8, M-03). */
export function BandeauDemo() {
  const { data: config } = useConfig();
  const reinitialiser = useReinitialiserDemo();
  if (config?.mode !== "demo") return null;
  return (
    <div
      role="status"
      className="flex flex-wrap items-center gap-3 border-b border-warn bg-warn-bg px-4 py-2 text-sm font-medium text-warn md:px-10"
    >
      <Icone nom="info" />
      <span className="flex-1">
        Démonstration, données fictives. Date figée au {date(config.dateFigee)}.
      </span>
      <button
        type="button"
        onClick={() => reinitialiser.mutate()}
        disabled={reinitialiser.isPending}
        className="min-h-11 rounded-md border border-warn bg-surf px-4 text-warn"
      >
        Réinitialiser les données
      </button>
    </div>
  );
}

function NavigationBureau() {
  return (
    <nav aria-label="Navigation principale" className="flex flex-wrap gap-1">
      {LIENS.map((l) => (
        <NavLink
          key={l.href}
          to={l.href}
          end={l.href === "/"}
          className={({ isActive }) =>
            `inline-flex min-h-11 items-center border-b-2 px-2 text-sm font-medium ${
              isActive ? "border-acc text-ink" : "border-transparent text-ink2"
            }`
          }
        >
          {l.libelle}
        </NavLink>
      ))}
    </nav>
  );
}

/** Barre d'onglets fixée en bas sous 768 px : cinq onglets dont « Plus » (21.5). */
function BarreOnglets({ ouvrirParametres }: { ouvrirParametres: () => void }) {
  const [plus, setPlus] = useState(false);
  const { pathname } = useLocation();
  const dansPlus = pathname.startsWith("/journal");
  const onglets = LIENS.filter((l) => l.href !== "/journal");
  const classe = (actif: boolean) =>
    `flex min-h-14 flex-1 flex-col items-center justify-center gap-0.5 border-t-[3px] text-[13px] ${
      actif ? "border-acc font-semibold text-acc" : "border-transparent text-ink2"
    }`;
  return (
    <>
      <nav
        aria-label="Navigation principale"
        className="fixed inset-x-0 bottom-0 z-30 flex border-t border-line bg-surf pb-[env(safe-area-inset-bottom)]"
      >
        {onglets.map((l) => (
          <NavLink
            key={l.href}
            to={l.href}
            end={l.href === "/"}
            className={({ isActive }) => classe(isActive)}
          >
            <Icone nom={l.icone} taille={22} />
            {l.libelle === "Tableau de bord" ? "Tableau" : l.libelle}
          </NavLink>
        ))}
        <button
          type="button"
          aria-haspopup="dialog"
          aria-current={dansPlus ? "page" : undefined}
          onClick={() => setPlus(true)}
          className={classe(dansPlus)}
        >
          <Icone nom="plus" taille={22} />
          Plus
        </button>
      </nav>
      <Dialogue titre="Plus" ouvert={plus} surFermeture={() => setPlus(false)}>
        <NavLink
          to="/journal"
          onClick={() => setPlus(false)}
          className="flex min-h-11 items-center gap-2"
        >
          <Icone nom="edit" /> Journal
        </NavLink>
        <button
          type="button"
          onClick={() => {
            setPlus(false);
            ouvrirParametres();
          }}
          className="flex min-h-11 items-center gap-2 text-left"
        >
          <Icone nom="gear" /> Paramètres
        </button>
      </Dialogue>
    </>
  );
}

export function AppShell({ children }: { children: ReactNode }) {
  const mobile = useEstMobile();
  const { data: config } = useConfig();
  const [parametres, setParametres] = useSearchParams();
  const onglet = parametres.get("parametres");
  const ouvrirParametres = () =>
    setParametres((p) => {
      p.set("parametres", "missions");
      return p;
    });
  const fermerParametres = () =>
    setParametres((p) => {
      p.delete("parametres");
      return p;
    });
  // Le titre de l'onglet porte le nombre de retards (T-14).
  const { data: tableau } = useTableauDeBord(undefined, "toutes");
  useTitreOnglet(tableau?.indicateurs.enRetard);

  return (
    <div className={`min-h-screen bg-bg text-ink ${mobile ? "pb-20" : ""}`}>
      <BandeauDemo />
      <header className="sticky top-0 z-20 flex min-h-14 flex-wrap items-center gap-x-9 gap-y-2 border-b border-line bg-surf px-4 md:min-h-16 md:px-10">
        <NavLink
          to="/"
          className="inline-flex min-h-11 items-center gap-2 text-[17px] font-semibold tracking-[-0.01em]"
        >
          <span aria-hidden className="inline-block size-3.5 border-2 border-acc" />
          Fiches CRA
        </NavLink>
        {!mobile && <NavigationBureau />}
        <div className="ml-auto flex items-center gap-3">
          <button
            type="button"
            onClick={ouvrirParametres}
            aria-label={mobile ? "Paramètres" : undefined}
            className="inline-flex min-h-11 min-w-11 items-center justify-center gap-2 rounded-md border border-line2 bg-surf px-3 text-sm font-medium"
          >
            <Icone nom="gear" />
            {!mobile && "Paramètres"}
          </button>
          {!mobile && config && (
            <span className="font-mono text-xs text-mut">{config.build.slice(0, 7)}</span>
          )}
        </div>
      </header>
      <main className="mx-auto flex max-w-[1200px] flex-col gap-7 px-4 pt-4 pb-8 md:px-10 md:pt-8 md:pb-16">
        {children}
      </main>
      {mobile && <BarreOnglets ouvrirParametres={ouvrirParametres} />}
      {onglet !== null && <ParametresDialog onglet={onglet} surFermeture={fermerParametres} />}
    </div>
  );
}
