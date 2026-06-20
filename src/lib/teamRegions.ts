import type { TFunction } from "i18next";

/**
 * Localized region label, falling back to a humanized form of the region id
 * (or an explicit fallback name) when no translation exists.
 */
export function buildRegionLabel(
  t: TFunction,
  regionId: string,
  fallbackName?: string,
): string {
  return t(`teamSelect.regionLabels.${regionId}`, {
    defaultValue:
      fallbackName ??
      regionId
        .split("-")
        .map((part) => part.charAt(0).toUpperCase() + part.slice(1))
        .join(" "),
  });
}

/** Coarse confederation bucket for a country, used when no competition pins a
 * team to an authored region. */
export function inferRegionId(countryCode: string): string {
  switch (countryCode) {
    case "BR":
    case "AR":
    case "UY":
    case "CL":
    case "CO":
    case "PE":
    case "EC":
    case "VE":
    case "PY":
    case "BO":
      return "south-america";
    case "US":
    case "CA":
    case "MX":
      return "north-america";
    case "CR":
    case "PA":
    case "HN":
    case "GT":
    case "SV":
    case "NI":
      return "central-america";
    case "AU":
    case "NZ":
      return "oceania";
    case "JP":
    case "KR":
    case "CN":
    case "SA":
    case "QA":
      return "asia";
    default:
      return "europe";
  }
}
