import { useState } from "react";

import type { Schemas } from "../../api/client";
import { urlBilanCsv } from "../../api/exports";
import { useBilan, type Filtre } from "../../api/hooks";
import {
  Carte,
  CarteChiffre,
  Chargement,
  ErreurChargement,
  Segmente,
} from "../../components/ui/divers";
import { Icone } from "../../components/ui/Icone";
import { heures, pourcentage } from "../../lib/format";

function LienCsv({ tableau, annee, filtre }: { tableau: string; annee: number; filtre: Filtre }) {
  return (
    <a
      href={urlBilanCsv(tableau, annee, filtre)}
      className="inline-flex min-h-11 items-center gap-2 text-sm text-acc underline underline-offset-[3px]"
    >
      <Icone nom="download" /> CSV
    </a>
  );
}

function Tableau({ colonnes, lignes }: { colonnes: string[]; lignes: (string | number)[][] }) {
  return (
    <div className="overflow-x-auto">
      <table className="w-full border-collapse text-[15px]">
        <thead>
          <tr className="border-b border-line2 bg-head text-left text-xs font-semibold tracking-[0.02em] text-ink2">
            {colonnes.map((c, i) => (
              <th key={c} scope="col" className={`h-11 px-2 ${i > 0 ? "text-right" : ""}`}>
                {c}
              </th>
            ))}
          </tr>
        </thead>
        <tbody>
          {lignes.map((l) => (
            <tr key={String(l[0])} className="border-b border-line">
              {l.map((v, i) =>
                i === 0 ? (
                  <th key={i} scope="row" className="px-2 py-2 text-left font-normal">
                    {v}
                  </th>
                ) : (
                  <td key={i} className="px-2 text-right font-mono">
                    {v}
                  </td>
                ),
              )}
            </tr>
          ))}
        </tbody>
      </table>
    </div>
  );
}

const enHeures = (q: Schemas["QuantiteDto"]) => [q.nom, heures(q.valeur)];

/** Bilan et statistiques (B-01 à B-08) : toutes les valeurs viennent du serveur. */
export function BilanPage() {
  const [annee, setAnnee] = useState<number | undefined>(undefined);
  const [filtre, setFiltre] = useState<Filtre>("toutes");
  const requete = useBilan(annee, filtre);
  if (requete.isPending) return <Chargement />;
  if (requete.isError)
    return <ErreurChargement erreur={requete.error} reessayer={() => void requete.refetch()} />;
  const b = requete.data;
  const csv = (tableau: string) => <LienCsv tableau={tableau} annee={b.annee} filtre={filtre} />;

  return (
    <>
      <div className="flex flex-wrap items-end justify-between gap-4">
        <h1 className="text-2xl font-semibold tracking-[-0.02em] md:text-[30px]">Bilan</h1>
        <div className="flex flex-wrap gap-3">
          <Segmente
            libelle="Année scolaire"
            options={[
              { valeur: 2026, libelle: "2026-2027" },
              { valeur: 2027, libelle: "2027-2028" },
            ]}
            valeur={b.annee}
            onChange={setAnnee}
          />
          <Segmente
            libelle="Type de fiche"
            options={[
              { valeur: "toutes" as const, libelle: "Toutes" },
              { valeur: "entreprise" as const, libelle: "Entreprise" },
              { valeur: "formation" as const, libelle: "Formation" },
            ]}
            valeur={filtre}
            onChange={setFiltre}
          />
        </div>
      </div>

      {b.totaux.attendues === 0 && (
        <p role="status" className="rounded-lg border border-line bg-surf p-4 text-ink2">
          Aucune semaine attendue pour l'instant : le bilan se remplira avec les premières fiches.
        </p>
      )}

      <div className="flex flex-wrap gap-4">
        <CarteChiffre libelle="Semaines attendues" valeur={b.totaux.attendues} />
        <CarteChiffre
          libelle="Fiches faites"
          valeur={b.totaux.faites}
          legende={pourcentage(b.totaux.tauxFaites)}
        />
        <CarteChiffre libelle="Justifiées" valeur={b.totaux.justifiees} />
        <CarteChiffre libelle="En retard" valeur={b.totaux.enRetard} />
      </div>
      <div className="flex flex-wrap gap-4">
        <CarteChiffre
          libelle="Ponctualité"
          valeur={pourcentage(b.ponctualite.taux)}
          progression={
            b.ponctualite.taux === null || b.ponctualite.taux === undefined
              ? undefined
              : { valeur: b.ponctualite.taux, libelle: "Taux de ponctualité" }
          }
          legende={`${b.ponctualite.ponctuelles} fiche${b.ponctualite.ponctuelles > 1 ? "s" : ""} à l'heure sur ${b.ponctualite.echues} échue${b.ponctualite.echues > 1 ? "s" : ""}`}
        />
        <CarteChiffre
          libelle="Délai moyen de clôture"
          valeur={
            b.ponctualite.delaiMoyenJours === null || b.ponctualite.delaiMoyenJours === undefined
              ? "—"
              : `${b.ponctualite.delaiMoyenJours.toFixed(1).replace(".", ",")} jour`
          }
          legende="Après le dimanche ; zéro ou moins : à l'heure"
        />
        <CarteChiffre libelle="Total d'heures" valeur={heures(b.totalHeures)} />
      </div>

      <div className="grid gap-7 md:grid-cols-2">
        <Carte titre="Retards et justifications par mois" actions={csv("retards-par-mois")}>
          <Tableau
            colonnes={["Mois", "En retard", "Justifiées", "Faites"]}
            lignes={b.parMois.map((m) => [m.libelle, m.enRetard, m.justifiees, m.faites])}
          />
        </Carte>
        <Carte titre="Justifiées par motif" actions={csv("justifications-par-motif")}>
          {b.parMotif.length === 0 ? (
            <p className="text-ink2">Aucune semaine justifiée.</p>
          ) : (
            <Tableau
              colonnes={["Motif", "Semaines"]}
              lignes={b.parMotif.map((m) => [m.nom, m.valeur])}
            />
          )}
        </Carte>
        <Carte titre="Temps par mission" actions={csv("temps-par-mission")}>
          <Tableau colonnes={["Mission", "Heures"]} lignes={b.parMission.map(enHeures)} />
        </Carte>
        <Carte titre="Temps par compétence" actions={csv("temps-par-competence")}>
          <p className="mb-2 text-[13px] text-ink2">
            Une ligne compte pour chacune de ses compétences : la somme peut dépasser le total
            d'heures.
          </p>
          <Tableau colonnes={["Compétence", "Heures"]} lignes={b.parCompetence.map(enHeures)} />
        </Carte>
      </div>
      <p>{csv("totaux")}</p>
    </>
  );
}
