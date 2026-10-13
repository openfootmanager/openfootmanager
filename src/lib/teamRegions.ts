import type { TFunction } from "i18next";

/**
 * Localized region label, falling back to a humanized form of the region id
 * (or an explicit fallback name) when no translation exists.
 */
export function buildRegionLabel(t: TFunction, regionId: string, fallbackName?: string): string {
  return t(`teamSelect.regionLabels.${regionId}`, {
    defaultValue:
      fallbackName ??
      regionId
        .split("-")
        .map((part) => part.charAt(0).toUpperCase() + part.slice(1))
        .join(" "),
  });
}
