import * as FlagIcons from "country-flag-icons/react/3x2";
import {
  countryName,
  isValidCountryCode,
  normaliseNationality,
  resolveCountryFlagCode,
} from "../../lib/countries";

type FlagComponent = (props: React.SVGProps<SVGSVGElement>) => React.JSX.Element;

const flagIcons = FlagIcons as Record<string, FlagComponent>;

interface CountryFlagProps {
  code: string;
  locale?: string;
  className?: string;
  title?: string;
  /**
   * Render the flag as decoration, with no accessible name of its own.
   *
   * Use this wherever the country is already named in adjacent text. A flag
   * that labels itself next to its own label reads out as "England England",
   * which is noise: the flag adds nothing a screen reader user cannot already
   * hear. Left off, the flag names itself — correct when it stands alone.
   */
  decorative?: boolean;
}

export function CountryFlag({
  code,
  locale = "en",
  className = "",
  title,
  decorative = false,
}: CountryFlagProps) {
  const normalisedCode = normaliseNationality(code).toUpperCase();

  if (!isValidCountryCode(normalisedCode)) {
    return null;
  }

  const flagCode = resolveCountryFlagCode(normalisedCode);
  const FlagIcon = flagCode ? flagIcons[flagCode.replace(/-/g, "_")] : null;

  const accessibleLabel =
    title ?? countryName(normalisedCode, locale) ?? normalisedCode;
  const classes = [
    "inline-flex",
    "items-center",
    "justify-center",
    "shrink-0",
    className,
  ]
    .filter(Boolean)
    .join(" ");

  // A `title` is announced by screen readers and shown as a tooltip, so leaving
  // one on a decorative flag puts back exactly the country name that hiding the
  // flag was meant to keep out of the reading.
  const describedLabel = decorative ? undefined : accessibleLabel;

  if (!FlagIcon) {
    return (
      <span
        role={decorative ? undefined : "img"}
        aria-hidden={decorative || undefined}
        aria-label={describedLabel}
        title={describedLabel}
        className={[
          classes,
          "rounded border border-white/15 bg-black/10 px-1 py-0.5 font-heading text-[0.65em] font-bold leading-none tracking-wide",
        ].join(" ")}
      >
        {normalisedCode}
      </span>
    );
  }

  return (
    <span className={classes} title={describedLabel}>
      <FlagIcon
        role={decorative ? undefined : "img"}
        aria-hidden={decorative || undefined}
        aria-label={describedLabel}
        focusable="false"
        className="h-[1em] w-[1.5em] rounded-[2px] shadow-sm"
      />
    </span>
  );
}
