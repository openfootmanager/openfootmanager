import i18n from "../i18n";

const SUPPORTED_LOCALES = [
  "en",
  "de",
  "es",
  "fr",
  "it",
  "pt",
  "pt-BR",
  "ru",
  "zh-CN",
];

function normalizeLocale(locale: string): string {
  return (
    SUPPORTED_LOCALES.find((l) => l.toLowerCase() === locale.toLowerCase()) ??
    locale
  );
}

export function applyExtraTranslations(
  extra: Record<string, Record<string, unknown>> | undefined,
): void {
  if (!extra) return;
  for (const [rawLocale, bundle] of Object.entries(extra)) {
    if (
      typeof bundle !== "object" ||
      bundle === null ||
      Array.isArray(bundle)
    ) {
      continue;
    }
    const locale = normalizeLocale(rawLocale);
    i18n.addResourceBundle(locale, "translation", bundle, true, true);
  }
}
