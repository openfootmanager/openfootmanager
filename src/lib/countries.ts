/**
 * Country / nationality utilities powered by i18n-iso-countries.
 *
 * All nationalities are stored as ISO 3166-1 alpha-2 codes (e.g. "GB", "ES").
 * This module resolves codes → localised names and flag emoji.
 */
import countries from "i18n-iso-countries";
import enLocale from "i18n-iso-countries/langs/en.json";
import esLocale from "i18n-iso-countries/langs/es.json";
import ptLocale from "i18n-iso-countries/langs/pt.json";
import frLocale from "i18n-iso-countries/langs/fr.json";
import deLocale from "i18n-iso-countries/langs/de.json";
import itLocale from "i18n-iso-countries/langs/it.json";
import trLocale from "i18n-iso-countries/langs/tr.json";

// Register locales we support
countries.registerLocale(enLocale);
countries.registerLocale(esLocale);
countries.registerLocale(ptLocale);
countries.registerLocale(frLocale);
countries.registerLocale(deLocale);
countries.registerLocale(itLocale);
countries.registerLocale(trLocale);

/**
 * Convert an ISO alpha-2 code to a flag emoji.
 * Works well on macOS/Linux/Android/iOS.
 * On Windows, renders as two-letter regional indicator pair (still identifiable).
 */
export function countryFlag(alpha2: string): string {
  if (!alpha2 || alpha2.length !== 2) return "";
  const code = alpha2.toUpperCase();
  return String.fromCodePoint(
    ...[...code].map((c) => 0x1f1e6 + c.charCodeAt(0) - 65)
  );
}

type NavigatorWithUserAgentData = Navigator & {
  userAgentData?: {
    platform?: string;
  };
};

function getPlatformName(): string {
  if (typeof navigator === "undefined") {
    return "";
  }

  const navigatorWithUserAgentData = navigator as NavigatorWithUserAgentData;

  return navigatorWithUserAgentData.userAgentData?.platform ?? navigator.platform ?? navigator.userAgent ?? "";
}

/**
 * Determine whether to use a country code badge instead of a flag emoji.
 * Windows doesn't support flag emojis well, so we use a country code badge instead.
 */
export function shouldUseCountryCodeBadge(platform = getPlatformName()): boolean {
  return /win/i.test(platform);
}

export function countryMarker(alpha2: string, platform = getPlatformName()): string {
  if (!alpha2 || alpha2.length !== 2) return "";

  return shouldUseCountryCodeBadge(platform)
    ? alpha2.toUpperCase()
    : countryFlag(alpha2);
}

function getBaseLocale(locale: string): string {
  if (!locale) return "en";
  // Convert 'pt-BR' to 'pt'
  return locale.split('-')[0].toLowerCase();
}

/**
 * Get the localised country name for an ISO alpha-2 code.
 * Falls back to English if the locale doesn't have a translation.
 */
export function countryName(alpha2: string, locale = "en"): string {
  if (!alpha2) return "";
  const baseLocale = getBaseLocale(locale);
  const name = countries.getName(alpha2.toUpperCase(), baseLocale);
  if (name) return name;
  // Fallback to English
  return countries.getName(alpha2.toUpperCase(), "en") ?? alpha2;
}

/**
 * Get all country entries as { code, name } sorted by name in the given locale.
 */
export function allCountries(locale = "en"): { code: string; name: string }[] {
  const baseLocale = getBaseLocale(locale);
  const obj = countries.getNames(baseLocale, { select: "official" });
  
  // If we couldn't find the names for the requested locale, fallback to English
  if (!obj || Object.keys(obj).length === 0) {
    const fallbackObj = countries.getNames("en", { select: "official" });
    return Object.entries(fallbackObj)
      .map(([code, name]) => ({ code, name }))
      .sort((a, b) => a.name.localeCompare(b.name, "en"));
  }

  return Object.entries(obj)
    .map(([code, name]) => ({ code, name }))
    .sort((a, b) => a.name.localeCompare(b.name, baseLocale));
}

/**
 * Validate that a string is a valid ISO alpha-2 country code.
 */
export function isValidCountryCode(code: string): boolean {
  if (!code || code.length !== 2) return false;
  return countries.isValid(code.toUpperCase());
}

/**
 * Map from old demonym-style nationality strings to ISO alpha-2 codes.
 * Used for backward compatibility with older save files.
 */
const DEMONYM_TO_CODE: Record<string, string> = {
  English: "GB",
  British: "GB",
  Scottish: "GB",
  Welsh: "GB",
  Spanish: "ES",
  German: "DE",
  French: "FR",
  Italian: "IT",
  Dutch: "NL",
  Portuguese: "PT",
  Brazilian: "BR",
  Argentine: "AR",
  Colombian: "CO",
  Belgian: "BE",
  Swedish: "SE",
  Norwegian: "NO",
  Danish: "DK",
  Croatian: "HR",
  Serbian: "RS",
  Swiss: "CH",
  Austrian: "AT",
};

/**
 * Normalise a nationality value: if it's already an alpha-2 code, return it;
 * if it's a demonym string from an old save, convert it.
 */
export function normaliseNationality(value: string): string {
  if (!value) return "";
  const upper = value.toUpperCase();
  // Already a valid 2-letter code?
  if (upper.length === 2 && countries.isValid(upper)) return upper;
  // Try demonym map
  return DEMONYM_TO_CODE[value] ?? value;
}

export { countries };
