import { useState } from "react";

import { ErreurApi, type Schemas } from "../../api/client";
import { useJustifier, useLeverJustification, useReferentiel } from "../../api/hooks";
import { Bouton } from "../../components/ui/Bouton";
import { Dialogue } from "../../components/ui/Dialogue";
import { Bandeau, Champ, CLASSES_CHAMP } from "../../components/ui/divers";
import { plage, type } from "../../lib/format";

type Semaine = Schemas["SemaineDto"];

/** Justifier une semaine à faire ou en retard avec un motif (J-01 à J-04, J-09). */
export function JustificationDialog({
  semaine,
  surFermeture,
}: {
  semaine: Semaine;
  surFermeture: () => void;
}) {
  const { data: motifs } = useReferentiel("motifs");
  const justifier = useJustifier();
  const [motifId, setMotifId] = useState<string>("");
  const [commentaire, setCommentaire] = useState("");
  const erreur = justifier.error instanceof ErreurApi ? justifier.error : undefined;
  const erreurCommentaire = erreur?.champ("commentaire");
  const erreurMotif = erreur?.champ("motifId");

  const valider = () => {
    if (!semaine.type) return;
    justifier.mutate(
      {
        lundi: semaine.lundi,
        type: semaine.type,
        motifId,
        commentaire: commentaire.trim() || null,
      },
      { onSuccess: surFermeture },
    );
  };

  return (
    <Dialogue
      titre={`Justifier la semaine ${semaine.numero}`}
      ouvert
      surFermeture={surFermeture}
      actions={
        <>
          <Bouton onClick={surFermeture}>Annuler</Bouton>
          <Bouton variante="primaire" onClick={valider} disabled={!motifId || justifier.isPending}>
            Justifier la semaine
          </Bouton>
        </>
      }
    >
      <p className="text-ink2">
        {type(semaine.type)} · {plage(semaine.lundi, semaine.dimanche)}. La semaine sortira du
        compteur de retards.
      </p>
      <fieldset className="flex flex-col gap-2">
        <legend className="mb-1 text-sm font-medium">Motif</legend>
        {motifs?.map((m) => (
          <label
            key={m.id}
            className={`flex min-h-11 items-center gap-3 rounded-md border px-3 ${
              motifId === m.id ? "border-acc bg-acc-bg" : "border-line2"
            }`}
          >
            <input
              type="radio"
              name="motif"
              value={m.id}
              checked={motifId === m.id}
              onChange={() => setMotifId(m.id)}
              className="size-5 accent-[var(--acc)]"
              autoFocus={m === motifs[0]}
            />
            {m.nom}
            {m.code === "autre" && (
              <span className="text-[13px] text-ink2">(commentaire obligatoire)</span>
            )}
          </label>
        ))}
        {erreurMotif && <p className="text-[13px] text-err">{erreurMotif}</p>}
      </fieldset>
      <Champ id="commentaire" libelle="Commentaire" erreur={erreurCommentaire}>
        <textarea
          id="commentaire"
          value={commentaire}
          onChange={(e) => setCommentaire(e.target.value)}
          aria-invalid={erreurCommentaire ? true : undefined}
          aria-describedby={erreurCommentaire ? "commentaire-erreur" : undefined}
          rows={3}
          maxLength={1000}
          className={`${CLASSES_CHAMP} py-2.5`}
          ref={(el) => {
            if (el && erreurCommentaire) el.focus();
          }}
        />
      </Champ>
      {erreur && !erreurCommentaire && !erreurMotif && (
        <Bandeau ton="err">{erreur.message}</Bandeau>
      )}
    </Dialogue>
  );
}

/** Lever une justification : la semaine retrouve son statut calculé (J-06). */
export function LeverDialog({
  semaine,
  surFermeture,
}: {
  semaine: Semaine;
  surFermeture: () => void;
}) {
  const lever = useLeverJustification();
  return (
    <Dialogue
      titre="Lever la justification"
      ouvert
      surFermeture={surFermeture}
      actions={
        <>
          <Bouton onClick={surFermeture}>Annuler</Bouton>
          <Bouton
            variante="danger"
            disabled={lever.isPending}
            onClick={() =>
              semaine.justificationId &&
              lever.mutate(semaine.justificationId, { onSuccess: surFermeture })
            }
          >
            Lever la justification
          </Bouton>
        </>
      }
    >
      <p>
        La semaine {semaine.numero} ({plage(semaine.lundi, semaine.dimanche)}) retrouvera son statut
        calculé. La justification reste visible dans le journal.
      </p>
      {lever.error && <Bandeau ton="err">{lever.error.message}</Bandeau>}
    </Dialogue>
  );
}
