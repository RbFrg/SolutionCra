import { useEffect, useRef, useState } from "react";
import { Link, useNavigate, useParams } from "react-router";

import { ErreurApi, type Schemas } from "../../api/client";
import { copierPourOneNote, urlExportFiche } from "../../api/exports";
import { useChangerStatut, useFiche, useReferentiel, useSauvegarderFiche } from "../../api/hooks";
import { Bouton } from "../../components/ui/Bouton";
import { Bandeau, Chargement, CLASSES_CHAMP, ErreurChargement } from "../../components/ui/divers";
import { Icone } from "../../components/ui/Icone";
import { FICHE, Pastille } from "../../components/ui/Pastille";
import { date, heures, instant, plage, type } from "../../lib/format";

type Fiche = Schemas["FicheDto"];
type StatutFiche = Schemas["StatutFicheDto"];

/** Délai d'inactivité avant la sauvegarde automatique (E-18). */
export const DELAI_SAUVEGARDE_MS = 800;

/** Ligne en cours d'édition ; `cle` est locale, `id` vient du serveur. */
export type LigneEdition = {
  cle: string;
  id: string | null;
  date: string;
  missionId: string;
  competenceIds: string[];
  dureeHeures: string;
  etapes: string;
  echangesTuteur: string;
};

let compteur = 0;
const nouvelleCle = () => `l${++compteur}`;

function versEdition(fiche: Fiche): LigneEdition[] {
  return fiche.lignes.map((l) => ({
    cle: nouvelleCle(),
    id: l.id,
    date: l.date,
    missionId: l.missionId,
    competenceIds: l.competenceIds,
    dureeHeures: String(l.dureeHeures),
    etapes: l.etapes,
    echangesTuteur: l.echangesTuteur ?? "",
  }));
}

function versSaisie(l: LigneEdition): Schemas["LigneSaisieDto"] {
  return {
    id: l.id,
    date: l.date,
    missionId: l.missionId,
    competenceIds: l.competenceIds,
    dureeHeures: Number(l.dureeHeures.replace(",", ".")),
    etapes: l.etapes,
    echangesTuteur: l.echangesTuteur.trim() ? l.echangesTuteur : null,
  };
}

type EtatSauvegarde =
  | { genre: "enregistre"; version: number }
  | { genre: "en-cours" }
  | { genre: "erreur"; erreur: ErreurApi | Error }
  | { genre: "conflit" };

/** Menu de statut : avancer d'un cran ou revenir en arrière (F-02, F-03). */
function MenuStatut({ fiche }: { fiche: Fiche }) {
  const changer = useChangerStatut(fiche.id);
  const { reset } = changer;
  // Un refus (« Ajoutez au moins une ligne… ») ne vaut que pour la version refusée.
  useEffect(() => reset(), [fiche.version, reset]);
  const ordre: StatutFiche[] = ["brouillon", "terminee", "transmise", "validee"];
  const indice = ordre.indexOf(fiche.statut);
  return (
    <div className="flex flex-col gap-2">
      <label className="flex flex-wrap items-center gap-2 text-sm font-medium">
        Statut de la fiche
        <select
          value={fiche.statut}
          disabled={changer.isPending}
          onChange={(e) =>
            changer.mutate({
              statut: e.target.value as StatutFiche,
              version: fiche.version,
              date: null,
            })
          }
          className={`${CLASSES_CHAMP} w-auto`}
        >
          {ordre.map((s, i) => (
            <option key={s} value={s} disabled={i > indice + 1}>
              {FICHE[s].libelle}
            </option>
          ))}
        </select>
      </label>
      {changer.error && (
        <p role="alert" className="flex items-center gap-1 text-[13px] text-err">
          <Icone nom="retard" taille={14} />
          {changer.error.message}
        </p>
      )}
    </div>
  );
}

function Competences({
  ligne,
  competences,
  noms,
  surChangement,
  invalide,
}: {
  ligne: LigneEdition;
  competences: Schemas["EntreeDto"][];
  noms: Map<string, string>;
  surChangement: (ids: string[]) => void;
  invalide: boolean;
}) {
  const choisies = ligne.competenceIds;
  const proposees = competences.filter((c) => !c.archivee || choisies.includes(c.id));
  return (
    <details className={`rounded-md border ${invalide ? "border-2 border-err" : "border-line2"}`}>
      <summary className="flex min-h-11 cursor-pointer items-center px-3 text-sm">
        {choisies.length === 0 ? (
          <span className="text-mut">Choisir…</span>
        ) : (
          <span className="whitespace-pre-line">
            {choisies.map((id) => noms.get(id) ?? "?").join("\n")}
          </span>
        )}
      </summary>
      <fieldset className="flex flex-col gap-1 border-t border-line p-2">
        <legend className="sr-only">Compétences</legend>
        {proposees.map((c) => (
          <label key={c.id} className="flex min-h-11 items-center gap-2 text-sm">
            <input
              type="checkbox"
              className="size-[22px] accent-[var(--acc)]"
              checked={choisies.includes(c.id)}
              onChange={(e) =>
                surChangement(
                  e.target.checked ? [...choisies, c.id] : choisies.filter((x) => x !== c.id),
                )
              }
            />
            {c.nom}
          </label>
        ))}
      </fieldset>
    </details>
  );
}

function Editeur({ fiche }: { fiche: Fiche }) {
  const { data: missions = [] } = useReferentiel("missions", true);
  const { data: competences = [] } = useReferentiel("competences", true);
  const sauvegarder = useSauvegarderFiche(fiche.id);
  const naviguer = useNavigate();

  const [lignes, setLignes] = useState<LigneEdition[]>(() => versEdition(fiche));
  const [etat, setEtat] = useState<EtatSauvegarde>({ genre: "enregistre", version: fiche.version });
  const version = useRef(fiche.version);
  const revision = useRef(0);
  const [aSupprimer, setASupprimer] = useState<number>();
  const [copie, setCopie] = useState<"ok" | "refusee">();

  // Une version venue d'ailleurs (changement de statut) remplace la version
  // connue ; nos propres sauvegardes l'ont déjà enregistrée et ne recréent pas
  // les lignes, ce qui ferait perdre le focus pendant la saisie.
  useEffect(() => {
    if (fiche.version === version.current) return;
    version.current = fiche.version;
    if (revision.current === 0) setLignes(versEdition(fiche));
    setEtat({ genre: "enregistre", version: fiche.version });
  }, [fiche]);

  const modifier = (suivantes: LigneEdition[]) => {
    revision.current += 1;
    setLignes(suivantes);
  };

  // Sauvegarde automatique : un seul PUT après le délai d'inactivité (F7).
  useEffect(() => {
    if (revision.current === 0) return;
    const enCours = revision.current;
    const minuterie = window.setTimeout(() => {
      setEtat({ genre: "en-cours" });
      sauvegarder.mutate(
        { version: version.current, lignes: lignes.map(versSaisie) },
        {
          onSuccess: (f) => {
            version.current = f.version;
            setEtat({ genre: "enregistre", version: f.version });
            // Les nouvelles lignes reçoivent leur identifiant serveur.
            setLignes((actuelles) =>
              actuelles.map((l, i) => (l.id === null ? { ...l, id: f.lignes[i]?.id ?? null } : l)),
            );
            if (revision.current === enCours) revision.current = 0;
          },
          onError: (e) =>
            setEtat(
              e instanceof ErreurApi && e.code === "conflit-de-version"
                ? { genre: "conflit" }
                : { genre: "erreur", erreur: e },
            ),
        },
      );
    }, DELAI_SAUVEGARDE_MS);
    return () => window.clearTimeout(minuterie);
    // La sauvegarde ne dépend que du contenu édité.
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [lignes]);

  const nomsCompetences = new Map(competences.map((c) => [c.id, c.nom]));
  const missionsProposees = (actuelle: string) =>
    missions.filter((m) => !m.archivee || m.id === actuelle);
  const erreur =
    etat.genre === "erreur" && etat.erreur instanceof ErreurApi ? etat.erreur : undefined;
  const champ = (i: number, nom: string) => erreur?.champ(`lignes[${i}].${nom}`);
  const maj = (i: number, valeurs: Partial<LigneEdition>) =>
    modifier(lignes.map((l, j) => (j === i ? { ...l, ...valeurs } : l)));
  const deplacer = (i: number, pas: -1 | 1) => {
    const j = i + pas;
    if (j < 0 || j >= lignes.length) return;
    const copieLignes = [...lignes];
    const [l] = copieLignes.splice(i, 1);
    if (l) copieLignes.splice(j, 0, l);
    modifier(copieLignes);
  };
  const ajouter = () =>
    modifier([
      ...lignes,
      {
        cle: nouvelleCle(),
        id: null,
        date: lignes.at(-1)?.date ?? fiche.lundi,
        missionId: lignes.at(-1)?.missionId ?? missions.find((m) => !m.archivee)?.id ?? "",
        competenceIds: [],
        dureeHeures: "1",
        etapes: "",
        echangesTuteur: "",
      },
    ]);

  const copier = async () => {
    try {
      await copierPourOneNote(fiche.id);
      setCopie("ok");
    } catch {
      setCopie("refusee");
    }
  };

  const total = lignes.reduce((s, l) => s + (Number(l.dureeHeures.replace(",", ".")) || 0), 0);

  return (
    <>
      <nav aria-label="Fil d'Ariane" className="text-sm text-ink2">
        <Link to="/fiches" className="text-acc underline underline-offset-[3px]">
          Fiches
        </Link>{" "}
        / Semaine {fiche.numero}
      </nav>
      <div className="flex flex-wrap items-start justify-between gap-4">
        <div className="flex flex-col gap-2">
          <h1 className="text-2xl font-semibold tracking-[-0.02em] md:text-[30px]">
            Fiche CRA {type(fiche.type).toLowerCase()} · Semaine {fiche.numero}
          </h1>
          <p className="font-mono text-ink2">{plage(fiche.lundi, fiche.dimanche)}</p>
          <div className="flex flex-wrap items-center gap-2">
            <Pastille
              genre="fiche"
              statut={fiche.statut}
              detail={
                fiche.statut === "transmise" && fiche.transmiseLe
                  ? `le ${date(fiche.transmiseLe)}`
                  : fiche.statut === "validee" && fiche.valideeLe
                    ? `le ${date(fiche.valideeLe)}`
                    : undefined
              }
            />
            {fiche.modifieeApresTransmission && (
              <span className="text-[13px] text-ink2">
                Modifiée après transmission, le {instant(fiche.modifieeApresTransmissionLe)}
              </span>
            )}
          </div>
        </div>
        <MenuStatut fiche={fiche} />
      </div>

      <div role="toolbar" aria-label="Exports" className="flex flex-wrap gap-2">
        <Bouton icone="copie" onClick={() => void copier()}>
          Copier pour OneNote
        </Bouton>
        <a
          href={urlExportFiche(fiche.id, "docx")}
          className="inline-flex min-h-11 items-center gap-2 rounded-md border border-line2 bg-surf px-4 text-sm font-medium"
        >
          <Icone nom="download" /> Word
        </a>
        <a
          href={urlExportFiche(fiche.id, "pdf")}
          className="inline-flex min-h-11 items-center gap-2 rounded-md border border-line2 bg-surf px-4 text-sm font-medium"
        >
          <Icone nom="download" /> PDF
        </a>
      </div>
      {copie === "ok" && <Bandeau ton="ok">Fiche copiée : collez-la dans OneNote.</Bandeau>}
      {copie === "refusee" && (
        <div className="flex flex-col gap-2">
          <Bandeau
            ton="warn"
            actions={
              <a className="text-sm underline" href={urlExportFiche(fiche.id, "html")} download>
                Télécharger le HTML
              </a>
            }
          >
            Le navigateur a refusé le presse-papier. Sélectionnez le tableau ci-dessous puis Ctrl+C.
          </Bandeau>
          <iframe
            title="Aperçu de la fiche"
            sandbox=""
            src={urlExportFiche(fiche.id, "html")}
            className="h-80 w-full rounded-lg border border-line bg-white"
          />
        </div>
      )}

      {etat.genre === "conflit" && (
        <Bandeau
          ton="warn"
          actions={<Bouton onClick={() => window.location.reload()}>Recharger la fiche</Bouton>}
        >
          La fiche a été modifiée ailleurs. Rechargez-la avant de continuer.
        </Bandeau>
      )}
      {erreur && (
        <Bandeau ton="err">
          {erreur.probleme.errors?.length
            ? `${erreur.probleme.errors.length} champ${erreur.probleme.errors.length > 1 ? "s" : ""} à corriger.`
            : erreur.message}
        </Bandeau>
      )}

      <div className="overflow-x-auto rounded-lg border border-line bg-surf">
        <table className="w-full min-w-[1100px] border-collapse">
          <thead>
            <tr className="border-b border-line2 bg-head text-left text-xs font-semibold tracking-[0.02em] text-ink2">
              {fiche.colonnes.map((c) => (
                <th key={c} scope="col" className="h-11 px-2">
                  {c}
                </th>
              ))}
              <th scope="col" className="px-2">
                <span className="sr-only">Actions</span>
              </th>
            </tr>
          </thead>
          <tbody>
            {lignes.map((l, i) => {
              const n = i + 1;
              const erreurs = {
                date: champ(i, "date"),
                mission: champ(i, "missionId"),
                competences: champ(i, "competenceIds"),
                duree: champ(i, "dureeHeures"),
                etapes: champ(i, "etapes"),
                echanges: champ(i, "echangesTuteur"),
              };
              const message = (id: string, texte?: string) =>
                texte && (
                  <p id={id} className="mt-1 text-[13px] text-err">
                    {texte}
                  </p>
                );
              return (
                <tr key={l.cle} className="border-b border-line align-top">
                  <td className="p-2">
                    <input
                      type="date"
                      aria-label={`Date de la ligne ${n}`}
                      value={l.date}
                      min={fiche.lundi}
                      max={fiche.dimanche}
                      onChange={(e) => maj(i, { date: e.target.value })}
                      aria-invalid={erreurs.date ? true : undefined}
                      aria-describedby={erreurs.date ? `${l.cle}-date` : undefined}
                      className={`${CLASSES_CHAMP} font-mono`}
                    />
                    {message(`${l.cle}-date`, erreurs.date)}
                  </td>
                  <td className="p-2">
                    <select
                      aria-label={`Mission de la ligne ${n}`}
                      value={l.missionId}
                      onChange={(e) => maj(i, { missionId: e.target.value })}
                      aria-invalid={erreurs.mission ? true : undefined}
                      aria-describedby={erreurs.mission ? `${l.cle}-mission` : undefined}
                      className={CLASSES_CHAMP}
                    >
                      {!l.missionId && <option value="">Choisir…</option>}
                      {missionsProposees(l.missionId).map((m) => (
                        <option key={m.id} value={m.id}>
                          {m.nom}
                          {m.archivee ? " (archivée)" : ""}
                        </option>
                      ))}
                    </select>
                    {message(`${l.cle}-mission`, erreurs.mission)}
                  </td>
                  <td className="p-2">
                    <Competences
                      ligne={l}
                      competences={competences}
                      noms={nomsCompetences}
                      invalide={!!erreurs.competences}
                      surChangement={(ids) => maj(i, { competenceIds: ids })}
                    />
                    {message(`${l.cle}-competences`, erreurs.competences)}
                  </td>
                  <td className="w-28 p-2">
                    <input
                      type="number"
                      inputMode="decimal"
                      aria-label={`Durée en heures de la ligne ${n}`}
                      step={0.25}
                      min={0.25}
                      max={12}
                      value={l.dureeHeures}
                      onChange={(e) => maj(i, { dureeHeures: e.target.value })}
                      aria-invalid={erreurs.duree ? true : undefined}
                      aria-describedby={erreurs.duree ? `${l.cle}-duree` : undefined}
                      className={`${CLASSES_CHAMP} font-mono`}
                    />
                    {message(`${l.cle}-duree`, erreurs.duree)}
                  </td>
                  <td className="p-2">
                    <textarea
                      aria-label={`Étapes de la ligne ${n}`}
                      value={l.etapes}
                      rows={Math.max(2, l.etapes.split("\n").length)}
                      onChange={(e) => maj(i, { etapes: e.target.value })}
                      aria-invalid={erreurs.etapes ? true : undefined}
                      className={`${CLASSES_CHAMP} py-2.5`}
                    />
                    {message(`${l.cle}-etapes`, erreurs.etapes)}
                  </td>
                  <td className="p-2">
                    <textarea
                      aria-label={`${fiche.colonnes[5] ?? "Échanges"}, ligne ${n}`}
                      value={l.echangesTuteur}
                      rows={Math.max(2, l.echangesTuteur.split("\n").length)}
                      onChange={(e) => maj(i, { echangesTuteur: e.target.value })}
                      aria-invalid={erreurs.echanges ? true : undefined}
                      className={`${CLASSES_CHAMP} py-2.5`}
                    />
                    {message(`${l.cle}-echanges`, erreurs.echanges)}
                  </td>
                  <td className="p-2">
                    <div className="flex gap-1">
                      <button
                        type="button"
                        aria-label={`Monter la ligne ${n}`}
                        disabled={i === 0}
                        onClick={() => deplacer(i, -1)}
                        className="inline-flex size-11 items-center justify-center rounded-md border border-line2 disabled:text-mut"
                      >
                        <Icone nom="chevU" />
                      </button>
                      <button
                        type="button"
                        aria-label={`Descendre la ligne ${n}`}
                        disabled={i === lignes.length - 1}
                        onClick={() => deplacer(i, 1)}
                        className="inline-flex size-11 items-center justify-center rounded-md border border-line2 disabled:text-mut"
                      >
                        <Icone nom="chevD" />
                      </button>
                      <button
                        type="button"
                        aria-label={`Supprimer la ligne ${n}`}
                        onClick={() => setASupprimer(i)}
                        className="inline-flex size-11 items-center justify-center rounded-md border border-err text-err"
                      >
                        <Icone nom="trash" />
                      </button>
                    </div>
                    {aSupprimer === i && (
                      <div
                        role="alertdialog"
                        aria-label={`Supprimer la ligne ${n}`}
                        className="mt-2 flex flex-col gap-2 rounded-md border border-err bg-err-bg p-2 text-sm text-err"
                      >
                        Supprimer la ligne {n} ?
                        <div className="flex gap-2">
                          <Bouton
                            variante="danger"
                            onClick={() => {
                              setASupprimer(undefined);
                              modifier(lignes.filter((_, j) => j !== i));
                            }}
                          >
                            Supprimer
                          </Bouton>
                          <Bouton onClick={() => setASupprimer(undefined)}>Annuler</Bouton>
                        </div>
                      </div>
                    )}
                  </td>
                </tr>
              );
            })}
          </tbody>
        </table>
      </div>

      <div className="flex flex-wrap items-center justify-between gap-3">
        <Bouton icone="plus" onClick={ajouter} disabled={missions.length === 0}>
          Ajouter une ligne
        </Bouton>
        <p className="font-mono">Total : {heures(total)}</p>
        <p role="status" aria-live="polite" className="text-sm text-ink2">
          {etat.genre === "enregistre" && `Enregistré, version ${etat.version}`}
          {etat.genre === "en-cours" && "Enregistrement…"}
          {etat.genre === "erreur" && "Échec de l'enregistrement"}
          {etat.genre === "conflit" && "Conflit de version"}
        </p>
      </div>
      {missions.length === 0 && (
        <Bandeau
          ton="info"
          actions={
            <Bouton onClick={() => void naviguer("?parametres=missions")}>Paramètres</Bouton>
          }
        >
          Ajoutez d'abord une mission dans Paramètres pour saisir des lignes.
        </Bandeau>
      )}
    </>
  );
}

export function FicheEditeurPage() {
  const { id = "" } = useParams();
  const requete = useFiche(id);
  if (requete.isPending) return <Chargement />;
  if (requete.isError)
    return <ErreurChargement erreur={requete.error} reessayer={() => void requete.refetch()} />;
  return <Editeur key={requete.data.id} fiche={requete.data} />;
}
