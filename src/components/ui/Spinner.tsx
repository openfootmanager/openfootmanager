import { useTranslation } from "react-i18next";

export interface SpinnerProps {
  /** Size and colour of the ring, e.g. `h-8 w-8 border-4 border-primary-500`. */
  className?: string;
  /** Status text for assistive technology; defaults to `common.loading`. */
  label?: string;
  /** The surrounding control already announces the busy state, so the ring is hidden. */
  decorative?: boolean;
}

const RING = "inline-block rounded-full border-t-transparent motion-safe:animate-spin";

export function Spinner({ className = "", label, decorative = false }: SpinnerProps) {
  const { t } = useTranslation();
  const ring = <span aria-hidden="true" className={`${RING} ${className}`.trim()} />;

  if (decorative) return ring;

  return (
    <span role="status" className="inline-flex">
      {ring}
      <span className="sr-only">{label ?? t("common.loading")}</span>
    </span>
  );
}
