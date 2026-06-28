import type { TFunction } from "i18next";

/** Translates a backend-supplied tab name (e.g. "Squad") to a localized label. */
export function getBlockerTabLabel(t: TFunction, tab: string): string {
  const key = `dashboard.${tab.charAt(0).toLowerCase()}${tab.slice(1)}`;
  const resolved = t(key);
  return resolved === key ? tab : resolved;
}
