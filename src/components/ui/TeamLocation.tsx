import { MapPin } from "lucide-react";
import { countryName } from "../../lib/countries";
import { CountryFlag } from "./CountryFlag";

interface TeamLocationProps {
  city: string;
  countryCode: string;
  locale?: string;
  className?: string;
  iconClassName?: string;
  flagClassName?: string;
  textClassName?: string;
}

export function TeamLocation({
  city,
  countryCode,
  locale = "en",
  className = "",
  iconClassName = "w-4 h-4",
  flagClassName = "text-sm leading-none",
  textClassName = "",
}: TeamLocationProps) {
  const label = countryName(countryCode, locale);

  return (
    <span className={["inline-flex items-center gap-1.5", className].filter(Boolean).join(" ")}>
      <MapPin className={iconClassName} />
      <CountryFlag code={countryCode} locale={locale} className={flagClassName} />
      <span className={textClassName}>
        {city}, {label}
      </span>
    </span>
  );
}
