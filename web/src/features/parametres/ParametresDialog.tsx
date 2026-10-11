import { useState } from "react";
import { useSearchParams } from "react-router";

import { ErreurApi, type Schemas } from "../../api/client";
import { urlExportComplet, urlLot } from "../../api/exports";
import {
  useAjouterEntree,
  useImporterReferentiel,
  useModifierEntree,
  useReferentiel,
  type NomReferentiel,
} from "../../api/hooks";
import { Bouton } from "../../components/ui/Bouton";
import { Dialogue } from "../../components/ui/Dialogue";
import { Bandeau, Champ, CLASSES_CHAMP } from "../../components/ui/divers";
import { Icone } from "../../components/ui/Icone";
import { appliquerTheme, lireTheme, type Theme } from "../../lib/media";

const ONGLETS = [
  { id: "missions", libelle: "Missions" },
  { id: "competences", libelle: "Compétences" },
  { id: "motifs", libelle: "Motifs" },
  { id: "apparence", libelle: "Apparence" },
  { id: "donnees", libelle: "Données" },
] as const;

function LigneEntree({ nom, entree }: { nom: NomReferentiel; entree: Schemas["EntreeDto"] }) {
  const modifier = useModifierEntree(nom);
  const [edition, setEdition] = useState<string>();
  const [avertissement, setAvertissement] = useState<string | null>();
  const erreur =
    modifier.error instanceof ErreurApi
      ? (modifier.error.champ("nom") ?? modifier.error.message)
      : undefined;
  return (
    <li className="flex flex-col gap-1 border-b border-line py-2 last:border-0">
      <div className="flex flex-wrap items-center gap-2">
        {edition === undefined ? (
          <span className={`min-w-0 flex-1 ${entree.archivee ? "text-mut line-through" : ""}`}>
            {entree.nom}
            {entree.code === "autre" && (
              <span className="ml-2 text-[13px] text-ink2">commentaire obligatoire</span>
            )}
          </span>
        ) : (
          <input
            aria-label={`Nouveau nom de « ${entree.nom} »`}
            value={edition}
            onChange={(e) => setEdition(e.target.value)}
            className={`${CLASSES_CHAMP} flex-1`}
            autoFocus
          />
        )}
        {edition === undefined ? (
          <>
            <button
              type="button"
              aria-label={`Renommer « ${entree.nom} »`}
              onClick={() => setEdition(entree.nom)}
              className="inline-flex size-11 items-center justify-center rounded-md border border-line2"
            >
              <Icone nom="edit" />
            </button>
            {entree.code !== "autre" && (
              <Bouton
                onClick={() => modifier.mutate({ id: entree.id, archivee: !entree.archivee })}
                disabled={modifier.isPending}
              >
                {entree.archivee ? "Restaurer" : "Archiver"}
              </Bouton>
            )}
          </>
        ) : (
          <>
            <Bouton
              variante="primaire"
              onClick={() =>
                modifier.mutate(
                  { id: entree.id, nom: edition },
                  {
                    onSuccess: (r) => {
                      setEdition(undefined);
                      setAvertissement(r.avertissement);
                    },
                  },
                )
              }
            >
              Enregistrer
            </Bouton>
            <Bouton onClick={() => setEdition(undefined)}>Annuler</Bouton>
          </>
        )}
      </div>
      {erreur && <p className="text-[13px] text-err">{erreur}</p>}
      {avertissement && <p className="text-[13px] text-warn">{avertissement}</p>}
    </li>
  );
}

function ImportCsv({ nom }: { nom: NomReferentiel }) {
  const importer = useImporterReferentiel(nom);
  const [csv, setCsv] = useState<string>();
  const apercu = importer.data;
  return (
    <div className="flex flex-col gap-2 rounded-md border border-line p-3">
      <label className="flex flex-col gap-1.5 text-sm font-medium">
        Importer un fichier CSV (colonnes nom ; code ; archivee)
        <input
          type="file"
          accept=".csv,text/csv"
          onChange={async (e) => {
            const fichier = e.target.files?.[0];
            if (!fichier) return;
            const texte = await fichier.text();
            setCsv(texte);
            importer.mutate({ csv: texte, mode: "apercu" });
          }}
          className="text-sm"
        />
      </label>
      {apercu && (
        <div className="flex flex-col gap-2 text-sm">
          <p>
            {apercu.lignes.length} ligne{apercu.lignes.length > 1 ? "s" : ""} lue
            {apercu.lignes.length > 1 ? "s" : ""}, {apercu.aCreer} à créer, {apercu.erreurs.length}{" "}
            en erreur.
          </p>
          {apercu.erreurs.map((m) => (
            <p key={`e${m.ligne}${m.message}`} className="text-err">
              Ligne {m.ligne} : {m.message}
            </p>
          ))}
          {apercu.avertissements.map((m) => (
            <p key={`a${m.ligne}${m.message}`} className="text-warn">
              Ligne {m.ligne} : {m.message}
            </p>
          ))}
          {apercu.ecrit ? (
            <Bandeau ton="ok">Import terminé.</Bandeau>
          ) : (
            <Bouton
              variante="primaire"
              disabled={
                !csv || apercu.erreurs.length > 0 || apercu.aCreer === 0 || importer.isPending
              }
              onClick={() => csv && importer.mutate({ csv, mode: "valider" })}
            >
              Importer {apercu.aCreer} entrée{apercu.aCreer > 1 ? "s" : ""}
            </Bouton>
          )}
        </div>
      )}
      {importer.error && <Bandeau ton="err">{importer.error.message}</Bandeau>}
    </div>
  );
}

function OngletReferentiel({ nom }: { nom: NomReferentiel }) {
  const { data: entrees = [] } = useReferentiel(nom, true);
  const ajouter = useAjouterEntree(nom);
  const [nouveau, setNouveau] = useState("");
  const [avertissement, setAvertissement] = useState<string | null>();
  const erreur =
    ajouter.error instanceof ErreurApi
      ? (ajouter.error.champ("nom") ?? ajouter.error.message)
      : undefined;
  return (
    <div className="flex flex-col gap-4">
      <form
        className="flex flex-wrap items-end gap-2"
        onSubmit={(e) => {
          e.preventDefault();
          ajouter.mutate(
            { nom: nouveau, code: null },
            {
              onSuccess: (r) => {
                setNouveau("");
                setAvertissement(r.avertissement);
              },
            },
          );
        }}
      >
        <div className="min-w-0 flex-1">
          <Champ id={`ajout-${nom}`} libelle="Nouvelle entrée" erreur={erreur}>
            <input
              id={`ajout-${nom}`}
              value={nouveau}
              onChange={(e) => setNouveau(e.target.value)}
              aria-invalid={erreur ? true : undefined}
              className={CLASSES_CHAMP}
            />
          </Champ>
        </div>
        <button
          type="submit"
          disabled={ajouter.isPending}
          className="inline-flex min-h-11 items-center gap-2 rounded-md border border-acc bg-acc px-4 text-sm font-semibold text-on-acc"
        >
          <Icone nom="plus" /> Ajouter
        </button>
      </form>
      {avertissement && <p className="text-[13px] text-warn">{avertissement}</p>}
      <ul>
        {entrees.map((e) => (
          <LigneEntree key={e.id} nom={nom} entree={e} />
        ))}
      </ul>
      {nom !== "motifs" && <ImportCsv nom={nom} />}
    </div>
  );
}

function OngletApparence() {
  const [theme, setTheme] = useState<Theme>(lireTheme);
  return (
    <fieldset className="flex flex-col gap-2">
      <legend className="mb-1 text-sm font-medium">Thème</legend>
      {(
        [
          ["systeme", "Système"],
          ["clair", "Clair"],
          ["sombre", "Sombre"],
        ] as const
      ).map(([valeur, libelle]) => (
        <label
          key={valeur}
          className={`flex min-h-11 items-center gap-3 rounded-md border px-3 ${theme === valeur ? "border-acc bg-acc-bg" : "border-line2"}`}
        >
          <input
            type="radio"
            name="theme"
            value={valeur}
            checked={theme === valeur}
            onChange={() => {
              setTheme(valeur);
              appliquerTheme(valeur);
            }}
            className="size-5 accent-[var(--acc)]"
          />
          {libelle}
        </label>
      ))}
    </fieldset>
  );
}

function OngletDonnees() {
  const [mois, setMois] = useState(() => new Date().toISOString().slice(0, 7));
  const [annee, setAnnee] = useState("2026");
  const lien =
    "inline-flex min-h-11 items-center gap-2 rounded-md border border-line2 bg-surf px-4 text-sm font-medium";
  return (
    <div className="flex flex-col gap-5">
      <section className="flex flex-col gap-2">
        <h3 className="font-semibold">Export complet</h3>
        <p className="text-[13px] text-ink2">
          Toutes les tables, à plat. Ce n'est pas une sauvegarde de la base.
        </p>
        <div className="flex flex-wrap gap-2">
          <a className={lien} href={urlExportComplet("csv")}>
            <Icone nom="download" /> CSV (zip)
          </a>
          <a className={lien} href={urlExportComplet("json")}>
            <Icone nom="download" /> JSON
          </a>
        </div>
      </section>
      <section className="flex flex-col gap-2">
        <h3 className="font-semibold">Lot de fiches</h3>
        <label className="flex flex-wrap items-center gap-2 text-sm font-medium">
          Mois
          <input
            type="month"
            value={mois}
            onChange={(e) => setMois(e.target.value)}
            className={`${CLASSES_CHAMP} w-auto`}
          />
        </label>
        <div className="flex flex-wrap gap-2">
          <a className={lien} href={urlLot("mois", mois, "pdf")}>
            <Icone nom="download" /> PDF du mois
          </a>
          <a className={lien} href={urlLot("mois", mois, "docx")}>
            <Icone nom="download" /> Word du mois
          </a>
        </div>
        <label className="flex flex-wrap items-center gap-2 text-sm font-medium">
          Année scolaire
          <select
            value={annee}
            onChange={(e) => setAnnee(e.target.value)}
            className={`${CLASSES_CHAMP} w-auto`}
          >
            <option value="2026">2026-2027</option>
            <option value="2027">2027-2028</option>
          </select>
        </label>
        <div className="flex flex-wrap gap-2">
          <a className={lien} href={urlLot("annee", annee, "pdf")}>
            <Icone nom="download" /> PDF de l'année
          </a>
          <a className={lien} href={urlLot("annee", annee, "docx")}>
            <Icone nom="download" /> Word de l'année
          </a>
        </div>
      </section>
    </div>
  );
}

/** Modale Paramètres, ouverte par `?parametres=<onglet>` sur toute route (G-01). */
export function ParametresDialog({
  onglet,
  surFermeture,
}: {
  onglet: string;
  surFermeture: () => void;
}) {
  const [, setParametres] = useSearchParams();
  const actif = ONGLETS.find((o) => o.id === onglet)?.id ?? "missions";
  const choisir = (id: string) =>
    setParametres((p) => {
      p.set("parametres", id);
      return p;
    });
  return (
    <Dialogue titre="Paramètres" ouvert surFermeture={surFermeture}>
      <div
        role="tablist"
        aria-label="Rubriques des paramètres"
        className="flex flex-wrap gap-1 border-b border-line"
      >
        {ONGLETS.map((o) => (
          <button
            key={o.id}
            type="button"
            role="tab"
            id={`onglet-${o.id}`}
            aria-selected={o.id === actif}
            aria-controls={`panneau-${o.id}`}
            onClick={() => choisir(o.id)}
            className={`min-h-11 border-b-2 px-2 text-sm font-medium ${o.id === actif ? "border-acc text-ink" : "border-transparent text-ink2"}`}
          >
            {o.libelle}
          </button>
        ))}
      </div>
      <div role="tabpanel" id={`panneau-${actif}`} aria-labelledby={`onglet-${actif}`}>
        {(actif === "missions" || actif === "competences" || actif === "motifs") && (
          <OngletReferentiel key={actif} nom={actif} />
        )}
        {actif === "apparence" && <OngletApparence />}
        {actif === "donnees" && <OngletDonnees />}
      </div>
    </Dialogue>
  );
}
