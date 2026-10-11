import type { ButtonHTMLAttributes, ReactNode } from "react";
import { Link } from "react-router";

import { Icone, type NomIcone } from "./Icone";

export type Variante = "primaire" | "secondaire" | "danger" | "contour-danger" | "lien";

const STYLES: Record<Variante, string> = {
  primaire: "bg-acc border-acc text-on-acc font-semibold",
  secondaire: "bg-surf border-line2 text-ink",
  danger: "bg-err border-err text-surf font-semibold",
  "contour-danger": "bg-surf border-err text-err",
  lien: "bg-transparent border-transparent text-acc underline underline-offset-[3px] !px-2",
};

const BASE =
  "inline-flex min-h-11 min-w-11 items-center justify-center gap-2 rounded-md border px-4 text-sm font-medium " +
  "disabled:cursor-not-allowed disabled:border-line disabled:bg-head disabled:text-mut";

type Props = {
  variante?: Variante;
  icone?: NomIcone;
  href?: string;
  children?: ReactNode;
} & ButtonHTMLAttributes<HTMLButtonElement>;

/** Un `<a>` si le bouton navigue, sinon un `<button type="button">` (21.5). */
export function Bouton({
  variante = "secondaire",
  icone,
  href,
  children,
  className,
  ...props
}: Props) {
  const classes = `${BASE} ${STYLES[variante]} ${className ?? ""}`;
  const contenu = (
    <>
      {icone && <Icone nom={icone} />}
      {children}
    </>
  );
  if (href) {
    return (
      <Link to={href} className={classes}>
        {contenu}
      </Link>
    );
  }
  return (
    <button type="button" className={classes} {...props}>
      {contenu}
    </button>
  );
}
