import * as Radix from "@radix-ui/react-dialog";
import { useState, type ReactNode } from "react";

import { useEstMobile } from "../../lib/media";
import { Icone } from "./Icone";

/** Boîte modale (bureau) ou feuille montant du bas (mobile, sous 768 px).
 *  Radix gère le focus, Échap et le retour du focus (J-09). */
export function Dialogue({
  titre,
  ouvert,
  surFermeture,
  actions,
  children,
}: {
  titre: string;
  ouvert: boolean;
  surFermeture: () => void;
  actions?: ReactNode;
  children: ReactNode;
}) {
  const mobile = useEstMobile();
  // Sans `Radix.Trigger`, Radix ne sait pas où rendre le focus : on retient
  // l'élément actif à l'ouverture et on y revient à la fermeture (J-09).
  const [origine] = useState(() =>
    typeof document === "undefined" ? null : (document.activeElement as HTMLElement | null),
  );
  const position = mobile
    ? "inset-x-0 bottom-0 max-h-[90vh] rounded-t-2xl shadow-[0_-12px_40px_rgba(0,0,0,0.25)]"
    : "top-1/2 left-1/2 w-[520px] max-w-[calc(100%-32px)] max-h-[90vh] -translate-x-1/2 -translate-y-1/2 rounded-[10px] shadow-[0_12px_40px_rgba(0,0,0,0.25)]";
  return (
    <Radix.Root open={ouvert} onOpenChange={(o) => !o && surFermeture()}>
      <Radix.Portal>
        <Radix.Overlay className="fixed inset-0 z-40 bg-[var(--scrim)]" />
        <Radix.Content
          aria-describedby={undefined}
          onCloseAutoFocus={(e) => {
            e.preventDefault();
            origine?.focus();
          }}
          className={`fixed z-50 flex flex-col overflow-hidden border border-line2 bg-surf text-ink ${position}`}
        >
          {mobile && <div aria-hidden className="mx-auto mt-2 h-[5px] w-10 rounded bg-line2" />}
          <div className="flex items-center justify-between gap-2 px-5 pt-4">
            <Radix.Title className="text-xl font-semibold">{titre}</Radix.Title>
            <Radix.Close
              aria-label="Fermer"
              className="inline-flex size-11 items-center justify-center rounded-md text-ink2"
            >
              <Icone nom="close" />
            </Radix.Close>
          </div>
          <div className="flex flex-col gap-3.5 overflow-y-auto px-5 pt-4 pb-5">{children}</div>
          {actions && (
            <div
              className={`flex gap-2 border-t border-line bg-head px-5 py-3.5 ${
                mobile ? "flex-col-reverse [&>*]:w-full" : "justify-end"
              }`}
            >
              {actions}
            </div>
          )}
        </Radix.Content>
      </Radix.Portal>
    </Radix.Root>
  );
}
