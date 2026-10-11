import { useMemo, useState } from "react";
import { Link } from "react-router";

import { ErreurApi, type Schemas } from "../../api/client";
import { useTableauDeBord, type Filtre } from "../../api/hooks";
import { Bouton } from "../../components/ui/Bouton";
import {
  Bandeau,
  Carte,
  CarteChiffre,
  Chargement,
  ErreurChargement,
  Segmente,
} from "../../components/ui/divers";
import { Icone } from "../../components/ui/Icone";
import { Pastille, SEMAINE } from "../../components/ui/Pastille";
import {
  annee as libelleAnnee,
  date,
  joursDeLaSemaine,
  plage,
  pourcentage,
  type,
} from "../../lib/format";
import { useEstMobile } from "../../lib/media";
import { useCreerEtOuvrir } from "../fiches/creer";
import { JustificationDialog, LeverDialog } from "../justification/JustificationDialog";

type Semaine = Schemas["SemaineDto"];

const FILTRES: { valeur: Filtre; libelle: string }[] = [
  { valeur: "toutes", libelle: "Toutes" },
  { valeur: "entreprise", libelle: "Entreprise" },
  { valeur: "formation", libelle: "Formation" },
];

const MOIS = [
  "Janvier",
  "Février",
  "Mars",
  "Avril",
  "Mai",
  "Juin",
  "Juillet",
  "Août",
  "Septembre",
  "Octobre",
  "Novembre",
  "Décembre",
];

/** Actions d'une semaine : créer ou ouvrir la fiche, justifier, lever (J-01, J-06). */
function ActionsSemaine({
  semaine,
  justifier,
  lever,
}: {
  semaine: Semaine;
  justifier: (s: Semaine) => void;
  lever: (s: Semaine) => void;
}) {
  const { lancer, mutation } = useCreerEtOuvrir();
  const justifiable = semaine.statut === "en_retard" || semaine.statut === "a_faire";
  return (
    <div className="flex flex-wrap gap-2">
      {semaine.ficheId ? (
        <Bouton href={`/fiches/${semaine.ficheId}`}>Ouvrir la fiche</Bouton>
      ) : (
        justifiable &&
        semaine.type && (
          <Bouton
            variante={semaine.statut === "en_retard" ? "primaire" : "secondaire"}
            disabled={mutation.isPending}
            onClick={() => semaine.type && lancer({ lundi: semaine.lundi, type: semaine.type })}
          >
            Créer la fiche
          </Bouton>
        )
      )}
      {justifiable && <Bouton onClick={() => justifier(semaine)}>Justifier</Bouton>}
      {mutation.error &&
        !(mutation.error instanceof ErreurApi && mutation.error.code === "fiche-existante") && (
          <p role="alert" className="w-full text-[13px] text-err">
            {mutation.error.message}
          </p>
        )}
      {semaine.statut === "justifiee" && (
        <Bouton onClick={() => lever(semaine)}>Lever la justification</Bouton>
      )}
    </div>
  );
}

/** Calendrier mensuel : une ligne par semaine, lundi en premier (T-06). */
function CalendrierMensuel({ semaines, aujourdhui }: { semaines: Semaine[]; aujourdhui: string }) {
  const moisDisponibles = useMemo(() => {
    const ensemble = new Set<string>();
    for (const s of semaines)
      for (const j of joursDeLaSemaine(s.lundi)) ensemble.add(j.slice(0, 7));
    return [...ensemble].sort();
  }, [semaines]);
  const [mois, setMois] = useState(aujourdhui.slice(0, 7));
  const index = moisDisponibles.indexOf(mois);
  const lignes = semaines.filter((s) => joursDeLaSemaine(s.lundi).some((j) => j.startsWith(mois)));
  const [a, m] = mois.split("-").map(Number);
  const titre = `${MOIS[(m ?? 1) - 1]} ${a}`;

  return (
    <Carte
      titre={titre}
      actions={
        <div className="flex items-center gap-1">
          <button
            type="button"
            aria-label="Mois précédent"
            disabled={index <= 0}
            onClick={() => setMois(moisDisponibles[index - 1] ?? mois)}
            className="inline-flex size-11 items-center justify-center rounded-md border border-line2 disabled:text-mut"
          >
            <Icone nom="chevL" />
          </button>
          <button
            type="button"
            aria-label="Mois suivant"
            disabled={index < 0 || index >= moisDisponibles.length - 1}
            onClick={() => setMois(moisDisponibles[index + 1] ?? mois)}
            className="inline-flex size-11 items-center justify-center rounded-md border border-line2 disabled:text-mut"
          >
            <Icone nom="chevR" />
          </button>
          <Link to="/calendrier" className="ml-2 text-sm text-acc underline underline-offset-[3px]">
            Calendrier d'alternance
          </Link>
        </div>
      }
    >
      <div className="overflow-x-auto">
        <table className="w-full min-w-[640px] border-collapse text-sm">
          <thead>
            <tr className="border-b border-line2 bg-head text-left text-xs font-semibold tracking-[0.02em] text-ink2">
              <th scope="col" className="h-11 px-2">
                Sem.
              </th>
              {["Lun", "Mar", "Mer", "Jeu", "Ven", "Sam", "Dim"].map((j) => (
                <th key={j} scope="col" className="px-2 text-center">
                  {j}
                </th>
              ))}
              <th scope="col" className="px-2">
                Statut de la semaine
              </th>
            </tr>
          </thead>
          <tbody>
            {lignes.map((s) => (
              <tr key={s.lundi} className="border-b border-line">
                <th scope="row" className="px-2 font-mono font-medium">
                  S{s.numero}
                </th>
                {joursDeLaSemaine(s.lundi).map((j) => {
                  const estAujourdhui = j === aujourdhui;
                  const horsMois = !j.startsWith(mois);
                  return (
                    <td key={j} className="p-1 text-center">
                      <span
                        aria-current={estAujourdhui ? "date" : undefined}
                        className={`inline-flex size-10 flex-col items-center justify-center rounded-md border-2 font-mono ${
                          estAujourdhui ? "border-acc font-semibold" : "border-transparent"
                        } ${horsMois ? "text-mut" : ""}`}
                      >
                        {Number(j.slice(8, 10))}
                        <span aria-hidden className="text-[11px] leading-none">
                          {SEMAINE[s.statut].glyphe}
                        </span>
                      </span>
                    </td>
                  );
                })}
                <td className="px-2">
                  <Pastille
                    genre="semaine"
                    statut={s.statut}
                    detail={s.joursRetard ? `${s.joursRetard} j` : undefined}
                  />
                </td>
              </tr>
            ))}
          </tbody>
        </table>
      </div>
    </Carte>
  );
}

/** Frise de l'année : une case par semaine, avec un nom accessible complet (T-09). */
function Frise({ semaines, annee }: { semaines: Semaine[]; annee: number }) {
  return (
    <Carte titre={`Année scolaire ${libelleAnnee(annee)}`}>
      <ol className="grid grid-cols-[repeat(auto-fill,minmax(44px,1fr))] gap-1">
        {semaines.map((s) => {
          const a = SEMAINE[s.statut];
          const nom = `Semaine ${s.numero}, ${plage(s.lundi, s.dimanche)}, ${type(s.typeAttendu).toLowerCase()}, ${a.libelle.toLowerCase()}`;
          return (
            <li key={s.lundi}>
              <Link
                to={`/calendrier?semaine=${s.lundi}`}
                aria-label={nom}
                title={nom}
                className={`flex min-h-11 flex-col items-center justify-center rounded border font-mono text-xs ${a.classes}`}
              >
                <span>{s.numero}</span>
                <span aria-hidden>{a.glyphe}</span>
              </Link>
            </li>
          );
        })}
      </ol>
      <p className="mt-3 flex flex-wrap gap-x-4 gap-y-1 text-[13px] text-ink2">
        {(["faite", "en_retard", "a_faire", "justifiee", "a_venir", "non_attendue"] as const).map(
          (s) => (
            <span key={s}>
              <span aria-hidden className="font-mono">
                {SEMAINE[s].glyphe}
              </span>{" "}
              {SEMAINE[s].libelle}
            </span>
          ),
        )}
      </p>
    </Carte>
  );
}

export function TableauDeBordPage() {
  const [annee, setAnnee] = useState<number | undefined>(undefined);
  const [filtre, setFiltre] = useState<Filtre>("toutes");
  const requete = useTableauDeBord(annee, filtre);
  const [aJustifier, setAJustifier] = useState<Semaine>();
  const [aLever, setALever] = useState<Semaine>();
  const mobile = useEstMobile();

  if (requete.isPending) return <Chargement />;
  if (requete.isError)
    return <ErreurChargement erreur={requete.error} reessayer={() => void requete.refetch()} />;
  const t = requete.data;
  const i = t.indicateurs;
  const courante = t.semaines.find((s) => s.lundi === t.semaineCourante);
  const justifiees = t.semaines.filter((s) => s.statut === "justifiee");
  const plusAncien = t.retards[0];
  const actions = { justifier: setAJustifier, lever: setALever };

  return (
    <>
      <div className="flex flex-wrap items-end justify-between gap-4">
        <div>
          <h1 className="text-2xl font-semibold tracking-[-0.02em] md:text-[30px]">
            Tableau de bord
          </h1>
          <p className="text-ink2">
            Année scolaire {libelleAnnee(t.annee)}
            {courante &&
              ` · semaine ${courante.numero}, ${plage(courante.lundi, courante.dimanche)}`}
          </p>
        </div>
        <div className="flex flex-wrap gap-3">
          <Segmente
            libelle="Année scolaire"
            options={[
              { valeur: 2026, libelle: "2026-2027" },
              { valeur: 2027, libelle: "2027-2028" },
            ]}
            valeur={t.annee}
            onChange={setAnnee}
          />
          <Segmente
            libelle="Type de fiche"
            options={FILTRES}
            valeur={filtre}
            onChange={setFiltre}
          />
        </div>
      </div>

      {plusAncien && (
        <Bandeau
          ton="err"
          actions={
            <>
              <Bouton onClick={() => setAJustifier(plusAncien)}>Justifier</Bouton>
            </>
          }
        >
          {t.retards.length === 1 ? "1 fiche en retard." : `${t.retards.length} fiches en retard.`}{" "}
          Semaine {plusAncien.numero} ({type(plusAncien.type).toLowerCase()}) depuis{" "}
          {plusAncien.joursRetard} jours.
        </Bandeau>
      )}

      <div className="flex flex-wrap gap-4">
        <CarteChiffre
          libelle="Fiches faites"
          valeur={`${i.faites} sur ${i.comptees}`}
          progression={
            i.tauxFaites === null || i.tauxFaites === undefined
              ? undefined
              : { valeur: i.tauxFaites, libelle: "Taux de fiches faites" }
          }
          legende={
            <>
              {pourcentage(i.tauxFaites)}
              {i.justifiees > 0 &&
                ` · ${i.justifiees} semaine${i.justifiees > 1 ? "s" : ""} justifiée${i.justifiees > 1 ? "s" : ""} non comptée${i.justifiees > 1 ? "s" : ""}`}
            </>
          }
        />
        <CarteChiffre
          libelle="En retard"
          valeur={i.enRetard}
          legende={
            plusAncien
              ? `le plus ancien : ${plusAncien.joursRetard} jours`
              : "Aucune fiche en retard."
          }
        />
        <CarteChiffre
          libelle="À faire cette semaine"
          valeur={i.aFaireCetteSemaine ? 1 : 0}
          legende={
            i.aFaireCetteSemaine
              ? `${type(i.aFaireCetteSemaine.type)} · échéance dimanche ${date(courante?.dimanche)}`
              : "Rien à rendre cette semaine."
          }
        />
      </div>

      <div className={`grid gap-7 ${mobile ? "" : "grid-cols-[minmax(0,1.6fr)_minmax(0,1fr)]"}`}>
        <CalendrierMensuel semaines={t.semaines} aujourdhui={t.aujourdhui} />
        <Carte titre={`Retards ${t.retards.length}`}>
          {t.retards.length === 0 && <p className="text-ink2">Aucune fiche en retard.</p>}
          <ul className="flex flex-col gap-4">
            {[
              ...t.retards,
              ...(courante?.statut === "a_faire" ? [courante] : []),
              ...justifiees,
            ].map((s) => (
              <li
                key={`${s.lundi}-${s.statut}`}
                className="flex flex-col gap-2 border-b border-line pb-4 last:border-0"
              >
                <div className="flex flex-wrap items-center justify-between gap-2">
                  <span className="font-medium">
                    Semaine {s.numero} ·{" "}
                    <span className="font-mono">{plage(s.lundi, s.dimanche)}</span>
                  </span>
                  <Pastille genre="semaine" statut={s.statut} />
                </div>
                <span className="text-[13px] text-ink2">
                  {type(s.type)}
                  {s.joursRetard ? ` · ${s.joursRetard} jours de retard` : ""}
                </span>
                <ActionsSemaine semaine={s} {...actions} />
              </li>
            ))}
          </ul>
        </Carte>
      </div>

      <Frise semaines={t.semaines} annee={t.annee} />

      {aJustifier && (
        <JustificationDialog semaine={aJustifier} surFermeture={() => setAJustifier(undefined)} />
      )}
      {aLever && <LeverDialog semaine={aLever} surFermeture={() => setALever(undefined)} />}
    </>
  );
}
