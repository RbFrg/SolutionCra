import { useState } from "react";
import { useSearchParams } from "react-router";

import { ErreurApi, type Schemas } from "../../api/client";
import { useAppliquerPlan, usePlan } from "../../api/hooks";
import { Bouton } from "../../components/ui/Bouton";
import { Dialogue } from "../../components/ui/Dialogue";
import {
  Bandeau,
  Carte,
  Champ,
  Chargement,
  CLASSES_CHAMP,
  ErreurChargement,
  Segmente,
} from "../../components/ui/divers";
import { Pastille } from "../../components/ui/Pastille";
import { date, jourMois, plage, type } from "../../lib/format";
import { useEstMobile } from "../../lib/media";

type TypeAttendu = Schemas["TypeAttenduDto"];

/** Calendrier d'alternance : une ligne par semaine, application d'un type à une plage (P-01 à P-08). */
export function CalendrierPage() {
  const [annee, setAnnee] = useState<number | undefined>(undefined);
  const requete = usePlan(annee);
  const appliquer = useAppliquerPlan();
  const mobile = useEstMobile();
  const [parametres] = useSearchParams();
  const [selection, setSelection] = useState<Set<string>>(() => {
    const s = parametres.get("semaine");
    return new Set(s ? [s] : []);
  });
  const [typeAttendu, setTypeAttendu] = useState<TypeAttendu>("formation");
  const [libelle, setLibelle] = useState("");
  const [aConfirmer, setAConfirmer] = useState<string[]>();

  if (requete.isPending) return <Chargement />;
  if (requete.isError)
    return <ErreurChargement erreur={requete.error} reessayer={() => void requete.refetch()} />;
  const plan = requete.data;
  const choisies = [...selection].sort();
  const du = choisies[0];
  const au = choisies.at(-1);
  const erreur =
    appliquer.error instanceof ErreurApi && appliquer.error.code !== "confirmation-requise"
      ? appliquer.error
      : undefined;

  const basculer = (lundi: string) =>
    setSelection((s) => {
      const n = new Set(s);
      if (n.has(lundi)) n.delete(lundi);
      else n.add(lundi);
      return n;
    });

  const lancer = (confirmer: boolean) => {
    if (!du || !au) return;
    appliquer.mutate(
      { du, au, typeAttendu, libelle: libelle.trim() || null, confirmer },
      {
        onSuccess: () => {
          setSelection(new Set());
          setAConfirmer(undefined);
        },
        onError: (e) => {
          if (e instanceof ErreurApi && e.code === "confirmation-requise") {
            setAConfirmer(e.probleme.semainesPasseesTouchees ?? []);
          }
        },
      },
    );
  };

  const panneau = (
    <Carte titre="Appliquer à la sélection">
      <div className="flex flex-col gap-4">
        <p aria-live="polite" className="text-ink2">
          {choisies.length === 0
            ? "Cochez une ou plusieurs semaines."
            : `${choisies.length} semaine${choisies.length > 1 ? "s" : ""} sélectionnée${choisies.length > 1 ? "s" : ""}, du ${date(du)} au ${date(au)}.`}
          {choisies.length > 1 && " Le type s'applique à toutes les semaines de la plage."}
        </p>
        <Segmente
          libelle="Type attendu"
          options={[
            { valeur: "entreprise" as const, libelle: "Entreprise" },
            { valeur: "formation" as const, libelle: "Formation" },
            { valeur: "aucune" as const, libelle: "Aucune" },
          ]}
          valeur={typeAttendu}
          onChange={setTypeAttendu}
        />
        <Champ
          id="libelle"
          libelle="Libellé (facultatif)"
          aide="Par exemple « Vacances scolaires ». 100 caractères au plus."
          erreur={erreur?.champ("libelle")}
        >
          <input
            id="libelle"
            value={libelle}
            maxLength={100}
            onChange={(e) => setLibelle(e.target.value)}
            className={CLASSES_CHAMP}
          />
        </Champ>
        <Bouton
          variante="primaire"
          disabled={choisies.length === 0 || appliquer.isPending}
          onClick={() => lancer(false)}
        >
          Appliquer
        </Bouton>
        {erreur && <Bandeau ton="err">{erreur.message}</Bandeau>}
      </div>
    </Carte>
  );

  return (
    <>
      <div className="flex flex-wrap items-end justify-between gap-4">
        <div>
          <h1 className="text-2xl font-semibold tracking-[-0.02em] md:text-[30px]">
            Calendrier d'alternance
          </h1>
          <p className="text-ink2">Une semaine sans ligne de plan attend une fiche entreprise.</p>
        </div>
        <Segmente
          libelle="Année scolaire"
          options={[
            { valeur: 2026, libelle: "2026-2027" },
            { valeur: 2027, libelle: "2027-2028" },
          ]}
          valeur={plan.annee}
          onChange={setAnnee}
        />
      </div>
      <div className={`grid gap-7 ${mobile ? "" : "grid-cols-[minmax(0,1fr)_340px] items-start"}`}>
        <div className="overflow-x-auto rounded-lg border border-line bg-surf">
          <table className="w-full min-w-[640px] border-collapse">
            <thead>
              <tr className="border-b border-line2 bg-head text-left text-xs font-semibold tracking-[0.02em] text-ink2">
                <th scope="col" className="h-11 w-14 px-3">
                  <span className="sr-only">Sélection</span>
                </th>
                <th scope="col" className="px-2">
                  Sem.
                </th>
                <th scope="col" className="px-2">
                  Dates
                </th>
                <th scope="col" className="px-2">
                  Type attendu
                </th>
                <th scope="col" className="px-2">
                  Libellé
                </th>
                <th scope="col" className="px-2">
                  Statut actuel
                </th>
              </tr>
            </thead>
            <tbody>
              {plan.semaines.map((s) => {
                const coche = selection.has(s.lundi);
                return (
                  <tr
                    key={s.lundi}
                    className={`border-b border-line ${coche ? "bg-acc-bg" : ""} ${s.lundi <= plan.aujourdhui && plan.aujourdhui <= s.dimanche ? "font-semibold" : ""}`}
                  >
                    <td className="px-3">
                      <label className="inline-flex size-11 items-center justify-center">
                        <input
                          type="checkbox"
                          checked={coche}
                          onChange={() => basculer(s.lundi)}
                          aria-label={`Semaine ${s.numero}, ${plage(s.lundi, s.dimanche)}`}
                          className="size-[22px] accent-[var(--acc)]"
                        />
                      </label>
                    </td>
                    <th scope="row" className="px-2 text-left font-mono font-medium">
                      S{s.numero}
                    </th>
                    <td className="px-2 font-mono text-sm">
                      {jourMois(s.lundi)}–{jourMois(s.dimanche)}
                    </td>
                    <td className="px-2">{type(s.typeAttendu)}</td>
                    <td className="px-2 text-ink2">{s.libelle}</td>
                    <td className="px-2 py-2">
                      <Pastille genre="semaine" statut={s.statut} />
                    </td>
                  </tr>
                );
              })}
            </tbody>
          </table>
        </div>
        <div className={mobile ? "" : "sticky top-24"}>{panneau}</div>
      </div>

      {aConfirmer && (
        <Dialogue
          titre="Modifier des semaines passées"
          ouvert
          surFermeture={() => setAConfirmer(undefined)}
          actions={
            <>
              <Bouton onClick={() => setAConfirmer(undefined)}>Annuler</Bouton>
              <Bouton
                variante="primaire"
                onClick={() => lancer(true)}
                disabled={appliquer.isPending}
              >
                Confirmer la modification
              </Bouton>
            </>
          }
        >
          <p>
            Ces semaines sont déjà commencées ou passées ; leur statut sera recalculé et la
            modification inscrite au journal :
          </p>
          <ul className="list-disc pl-5 font-mono">
            {aConfirmer.map((l) => (
              <li key={l}>{date(l)}</li>
            ))}
          </ul>
        </Dialogue>
      )}
    </>
  );
}
