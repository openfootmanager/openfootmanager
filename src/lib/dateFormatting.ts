import type { SupportedLanguageCode } from "../i18n";
import { useSettingsStore } from "../store/settingsStore";

/**
 * The BCP-47 tag `Intl` should use for each language we ship.
 *
 * Typed against the language registry, so adding a locale to
 * `SUPPORTED_LANGUAGES` without deciding its date format is a compile error.
 * It was keyed on `zh` while the registry says `zh-CN`, and had no entry for
 * `ru`, `pt-BR` or `id` — none of which showed, because the lookup falls back
 * to the raw code and every one of those happens to be a usable tag on its own.
 * The next language may not be so lucky.
 */
const LANG_LOCALE: Record<SupportedLanguageCode, string> = {
  en: "en-US",
  es: "es-ES",
  // Deliberately unchanged: `pt` mapped here before `pt-BR` was a separate
  // language, and correcting it to pt-PT would change dates for existing
  // Portuguese players. Worth deciding, but not inside a test-wiring change.
  pt: "pt-BR",
  "pt-BR": "pt-BR",
  fr: "fr-FR",
  de: "de-DE",
  it: "it-IT",
  ru: "ru-RU",
  "zh-CN": "zh-CN",
  cs: "cs-CZ",
  tr: "tr-TR",
  id: "id-ID",
  vi: "vi-VN",
};

export function getLocale(lang?: string): string {
  if (!lang) {
    return "en-US";
  }
  return LANG_LOCALE[lang as SupportedLanguageCode] ?? lang;
}

function parseDateInput(dateStr: string): Date | null {
  const dateOnly = dateStr.substring(0, 10);
  if (!/^\d{4}-\d{2}-\d{2}$/.test(dateOnly)) {
    return null;
  }
  const value = new Date(`${dateOnly}T12:00:00`);
  if (Number.isNaN(value.getTime())) {
    return null;
  }
  return value;
}

export function formatMatchDate(dateStr: string, locale?: string): string {
  const date = parseDateInput(dateStr);
  if (!date) {
    return dateStr;
  }
  const resolvedLocale = locale ?? useSettingsStore.getState().settings.language;
  return date.toLocaleDateString(getLocale(resolvedLocale), {
    weekday: "short",
    month: "short",
    day: "numeric",
  });
}

export function formatDate(
  dateStr: string,
  locale?: string,
  opts?: Intl.DateTimeFormatOptions,
): string {
  const date = parseDateInput(dateStr);
  if (!date) {
    return dateStr;
  }
  return date.toLocaleDateString(
    getLocale(locale),
    opts || { year: "numeric", month: "long", day: "numeric" },
  );
}

export function formatDateFull(dateStr: string, locale?: string): string {
  const date = parseDateInput(dateStr);
  if (!date) {
    return dateStr;
  }
  return date.toLocaleDateString(getLocale(locale), {
    weekday: "long",
    year: "numeric",
    month: "long",
    day: "numeric",
  });
}

export function formatDateShort(dateStr: string, locale?: string): string {
  const date = parseDateInput(dateStr);
  if (!date) {
    return dateStr;
  }
  return date.toLocaleDateString(getLocale(locale), {
    month: "short",
    day: "numeric",
  });
}
