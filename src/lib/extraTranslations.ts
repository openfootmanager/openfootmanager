import i18n, { SUPPORTED_LANGUAGES, resolveSupportedLanguage } from "../i18n";

const PKG_NS = "pkgTranslations";

/**
 * Apply extra translation bundles from a world package.
 *
 * Each call first clears all previously-registered package bundles so
 * keys from a previous game session do not leak into the current one.
 * Bundles are registered under the dedicated "pkgTranslations" namespace,
 * which i18next checks as a fallback after the built-in "translation"
 * namespace, keeping built-in keys untouched.
 */
export function applyExtraTranslations(
  extra: Record<string, Record<string, unknown>> | undefined,
): void {
  for (const { code } of SUPPORTED_LANGUAGES) {
    i18n.removeResourceBundle(code, PKG_NS);
  }

  if (!extra) return;
  for (const [rawLocale, bundle] of Object.entries(extra)) {
    if (
      typeof bundle !== "object" ||
      bundle === null ||
      Array.isArray(bundle)
    ) {
      continue;
    }
    const locale = resolveSupportedLanguage(rawLocale);
    i18n.addResourceBundle(locale, PKG_NS, bundle, true, true);
  }
}
