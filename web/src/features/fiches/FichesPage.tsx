import { useState } from "react";
import { Link } from "react-router";

import { ErreurApi, type Schemas } from "../../api/client";
import { useFiches, usePlan, useSupprimerFiche } from "../../api/hooks";
import { Bouton } from "../../components/ui/Bouton";
import { Dialogue } from "../../components/ui/Dialogue";
import {
  Bandeau,
  Champ,
  Chargement,
  CLASSES_CHAMP,
  ErreurChargement,
  Segmente,
} from "../../components/ui/divers";
import { Icone } from "../../components/ui/Icone";
import { Pastille } from "../../components/ui/Pastille";
import { date, heures, plage, type } from "../../lib/format";
import { useEstMobile } from "../../lib/media";
import { useCreerEtOuvrir } from "./creer";

type Resume = Schemas["FicheResumeDto"];
type FiltreStatut = "tous" | Schemas["StatutFicheDto"];

function NouvelleFicheDialog({ surFermeture }: { surFermeture: () => void }) {
  const { data: plan } = usePlan(undefined);
  const { lancer, mutation } = useCreerEtOuvrir();
  const courante = plan?.semaines.find(
    (s) => s.lundi <= plan.aujourdhui && plan.aujourdhui <= s.dimanche,
  );
  const [lundi, setLundi] = useState<string>();
  const [typeFiche, setTypeFiche] = useState<Schemas["TypeFicheDto"]>();
  const choisie = plan?.semaines.find((s) => s.lundi === (lundi ?? courante?.lundi));
  const typeChoisi =
    typeFiche ?? (choisie?.typeAttendu === "formation" ? "formation" : "entreprise");
  const erreur = mutation.error instanceof ErreurApi ? mutation.error : undefined;

  return (
    <Dialogue
      titre="Nouvelle fiche"
      ouvert
      surFermeture={surFermeture}
      actions={
        <>
          <Bouton onClick={surFermeture}>Annuler</Bouton>
          <Bouton
            variante="primaire"
            disabled={!choisie || mutation.isPending}
            onClick={() => choisie && lancer({ lundi: choisie.lundi, type: typeChoisi })}
          >
            Créer la fiche
          </Bouton>
        </>
      }
    >
      <Champ
        id="semaine"
        libelle="Semaine"
        aide="Le type proposé est celui du calendrier d'alternance."
      >
        <select
          id="semaine"
          value={choisie?.lundi ?? ""}
          onChange={(e) => {
            setLundi(e.target.value);
            setTypeFiche(undefined);
          }}
          className={CLASSES_CHAMP}
        >
          {plan?.semaines.map((s) => (
            <option key={s.lundi} value={s.lundi}>
              Semaine {s.numero} · {plage(s.lundi, s.dimanche)} · {type(s.typeAttendu)}
            </option>
          ))}
        </select>
      </Champ>
      <Segmente
        libelle="Type de fiche"
        options={[
          { valeur: "entreprise" as const, libelle: "Entreprise" },
          { valeur: "formation" as const, libelle: "Formation" },
        ]}
        valeur={typeChoisi}
        onChange={setTypeFiche}
      />
      {erreur && erreur.code !== "fiche-existante" && <Bandeau ton="err">{erreur.message}</Bandeau>}
    </Dialogue>
  );
}

function SupprimerDialog({ fiche, surFermeture }: { fiche: Resume; surFermeture: () => void }) {
  const supprimer = useSupprimerFiche();
  return (
    <Dialogue
      titre="Supprimer la fiche"
      ouvert
      surFermeture={surFermeture}
      actions={
        <>
          <Bouton onClick={surFermeture}>Annuler</Bouton>
          <Bouton
            variante="danger"
            disabled={supprimer.isPending}
            onClick={() => supprimer.mutate(fiche.id, { onSuccess: surFermeture })}
          >
            Supprimer définitivement
          </Bouton>
        </>
      }
    >
      <p>
        La fiche {type(fiche.type).toLowerCase()} de la semaine {fiche.numero} (
        {plage(fiche.lundi, fiche.dimanche)}) et ses {fiche.nbLignes} ligne
        {fiche.nbLignes > 1 ? "s" : ""} seront supprimées. L'action est définitive ; le journal en
        garde la trace.
      </p>
      {supprimer.error && <Bandeau ton="err">{supprimer.error.message}</Bandeau>}
    </Dialogue>
  );
}

export function FichesPage() {
  const requete = useFiches();
  const mobile = useEstMobile();
  const [filtreType, setFiltreType] = useState<"toutes" | "entreprise" | "formation">("toutes");
  const [filtreStatut, setFiltreStatut] = useState<FiltreStatut>("tous");
  const [nouvelle, setNouvelle] = useState(false);
  const [aSupprimer, setASupprimer] = useState<Resume>();

  const fiches = (requete.data ?? []).filter(
    (f) =>
      (filtreType === "toutes" || f.type === filtreType) &&
      (filtreStatut === "tous" || f.statut === filtreStatut),
  );

  return (
    <>
      <div className="flex flex-wrap items-end justify-between gap-4">
        <h1 className="text-2xl font-semibold tracking-[-0.02em] md:text-[30px]">Fiches</h1>
        <Bouton variante="primaire" icone="plus" onClick={() => setNouvelle(true)}>
          Nouvelle fiche
        </Bouton>
      </div>
      <div className="flex flex-wrap gap-3">
        <Segmente
          libelle="Type de fiche"
          options={[
            { valeur: "toutes" as const, libelle: "Toutes" },
            { valeur: "entreprise" as const, libelle: "Entreprise" },
            { valeur: "formation" as const, libelle: "Formation" },
          ]}
          valeur={filtreType}
          onChange={setFiltreType}
        />
        <label className="flex items-center gap-2 text-sm font-medium">
          Statut
          <select
            value={filtreStatut}
            onChange={(e) => setFiltreStatut(e.target.value as FiltreStatut)}
            className={`${CLASSES_CHAMP} w-auto`}
          >
            <option value="tous">Tous</option>
            <option value="brouillon">Brouillon</option>
            <option value="terminee">Terminée</option>
            <option value="transmise">Transmise</option>
            <option value="validee">Validée</option>
          </select>
        </label>
      </div>

      {requete.isPending && <Chargement />}
      {requete.isError && (
        <ErreurChargement erreur={requete.error} reessayer={() => void requete.refetch()} />
      )}
      {requete.isSuccess && fiches.length === 0 && (
        <Bandeau
          ton="info"
          actions={<Bouton onClick={() => setNouvelle(true)}>Nouvelle fiche</Bouton>}
        >
          Aucune fiche pour ces filtres.
        </Bandeau>
      )}

      {fiches.length > 0 && mobile && (
        <ul className="flex flex-col gap-3">
          {fiches.map((f) => (
            <li key={f.id}>
              <article className="flex flex-col gap-2 rounded-lg border border-line bg-surf p-4">
                <div className="flex flex-wrap items-center justify-between gap-2">
                  <Link
                    to={`/fiches/${f.id}`}
                    className="min-h-11 content-center text-lg font-semibold text-acc underline underline-offset-[3px]"
                  >
                    Semaine {f.numero} · {type(f.type)}
                  </Link>
                  <Pastille genre="fiche" statut={f.statut} />
                </div>
                <span className="font-mono text-sm text-ink2">
                  {plage(f.lundi, f.dimanche)} · {heures(f.totalHeures)}
                </span>
                {f.modifieeApresTransmission && (
                  <span className="text-[13px] text-ink2">Modifiée après transmission</span>
                )}
                <Bouton variante="contour-danger" icone="trash" onClick={() => setASupprimer(f)}>
                  Supprimer
                </Bouton>
              </article>
            </li>
          ))}
        </ul>
      )}

      {fiches.length > 0 && !mobile && (
        <div className="overflow-x-auto rounded-lg border border-line bg-surf">
          <table className="w-full min-w-[760px] border-collapse text-[15px]">
            <thead>
              <tr className="border-b border-line2 bg-head text-left text-xs font-semibold tracking-[0.02em] text-ink2">
                <th scope="col" className="h-11 px-3">
                  Semaine
                </th>
                <th scope="col" className="px-3">
                  Type
                </th>
                <th scope="col" className="px-3">
                  Mission principale
                </th>
                <th scope="col" className="px-3">
                  Durée
                </th>
                <th scope="col" className="px-3">
                  Statut
                </th>
                <th scope="col" className="px-3">
                  Version
                </th>
                <th scope="col" className="px-3">
                  <span className="sr-only">Actions</span>
                </th>
              </tr>
            </thead>
            <tbody>
              {fiches.map((f) => (
                <tr key={f.id} className="h-[60px] border-b border-line hover:bg-head">
                  <th scope="row" className="px-3 text-left font-medium">
                    <Link
                      to={`/fiches/${f.id}`}
                      className="text-acc underline underline-offset-[3px]"
                    >
                      S{f.numero}
                    </Link>{" "}
                    <span className="font-mono text-sm text-ink2">
                      {plage(f.lundi, f.dimanche)}
                    </span>
                  </th>
                  <td className="px-3">{type(f.type)}</td>
                  <td className="max-w-[280px] truncate px-3" title={f.missionPrincipale ?? ""}>
                    {f.missionPrincipale ?? <span className="text-mut">Aucune ligne</span>}
                  </td>
                  <td className="px-3 font-mono">{heures(f.totalHeures)}</td>
                  <td className="px-3">
                    <Pastille genre="fiche" statut={f.statut} />
                    {f.transmiseLe && (
                      <span className="ml-2 text-[13px] text-ink2">le {date(f.transmiseLe)}</span>
                    )}
                    {f.modifieeApresTransmission && (
                      <span className="block text-[13px] text-ink2">
                        Modifiée après transmission
                      </span>
                    )}
                  </td>
                  <td className="px-3 font-mono text-sm text-mut">v{f.version}</td>
                  <td className="px-3 text-right">
                    <button
                      type="button"
                      aria-label={`Supprimer la fiche de la semaine ${f.numero}`}
                      onClick={() => setASupprimer(f)}
                      className="inline-flex size-11 items-center justify-center rounded-md border border-err text-err"
                    >
                      <Icone nom="trash" />
                    </button>
                  </td>
                </tr>
              ))}
            </tbody>
          </table>
        </div>
      )}

      {nouvelle && <NouvelleFicheDialog surFermeture={() => setNouvelle(false)} />}
      {aSupprimer && (
        <SupprimerDialog fiche={aSupprimer} surFermeture={() => setASupprimer(undefined)} />
      )}
    </>
  );
}
