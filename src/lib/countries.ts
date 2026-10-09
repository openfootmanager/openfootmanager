/**
 * Country / nationality utilities powered by i18n-iso-countries.
 *
 * The app still accepts ISO alpha-2 codes for most countries, but also supports
 * football-specific identities where the sport diverges from ISO country data.
 */
import countries, { type LocaleData } from "i18n-iso-countries";
import { hasFlag } from "country-flag-icons";
import enLocale from "i18n-iso-countries/langs/en.json";
import esLocale from "i18n-iso-countries/langs/es.json";
import ptLocale from "i18n-iso-countries/langs/pt.json";
import frLocale from "i18n-iso-countries/langs/fr.json";
import deLocale from "i18n-iso-countries/langs/de.json";
import itLocale from "i18n-iso-countries/langs/it.json";
import ruLocale from "i18n-iso-countries/langs/ru.json";
import zhLocale from "i18n-iso-countries/langs/zh.json";
import csLocale from "i18n-iso-countries/langs/cs.json";
import trLocale from "i18n-iso-countries/langs/tr.json";
import idLocale from "i18n-iso-countries/langs/id.json";
import viLocale from "i18n-iso-countries/langs/vi.json";
import type { SupportedLanguageCode } from "../i18n";

/**
 * The packs `i18n-iso-countries` ships for the languages the game offers: the
 * one place a pack is named. `SupportedLocale` and the registration both come
 * from here, so a pack cannot be listed without being registered, nor
 * registered without being listed. The vocabulary differs from the game's
 * language codes (`zh`, not `zh-CN`; no `pt-BR`).
 */
export const COUNTRY_PACKS = {
  en: enLocale,
  es: esLocale,
  pt: ptLocale,
  fr: frLocale,
  de: deLocale,
  it: itLocale,
  ru: ruLocale,
  zh: zhLocale,
  tr: trLocale,
  id: idLocale,
  cs: csLocale,
  vi: viLocale,
} satisfies Record<string, LocaleData>;

type SupportedLocale = keyof typeof COUNTRY_PACKS;

const SUPPORTED_LOCALES = Object.keys(COUNTRY_PACKS) as SupportedLocale[];

for (const pack of Object.values(COUNTRY_PACKS)) {
  countries.registerLocale(pack);
}

/**
 * Which country-name pack serves each language the game ships.
 *
 * A `Record` over `SupportedLanguageCode`, so shipping a new language fails to
 * compile until someone decides which pack it uses, instead of silently
 * falling back to English (#614). Brazilian Portuguese deliberately reuses
 * the Portuguese pack: the library has no `pt-BR` one, and the football
 * identities below are written identically in both.
 */
export const LIBRARY_LOCALE_FOR_LANGUAGE: Record<SupportedLanguageCode, SupportedLocale> = {
  en: "en",
  es: "es",
  pt: "pt",
  fr: "fr",
  de: "de",
  it: "it",
  ru: "ru",
  "pt-BR": "pt",
  "zh-CN": "zh",
  cs: "cs",
  tr: "tr",
  id: "id",
  vi: "vi",
};

interface FootballIdentityDefinition {
  code: string;
  names: Record<SupportedLocale, string>;
  aliases: string[];
  flagCode?: string;
  selectable?: boolean;
}

const FOOTBALL_IDENTITIES: Record<string, FootballIdentityDefinition> = {
  ENG: {
    code: "ENG",
    names: {
      en: "England",
      es: "Inglaterra",
      pt: "Inglaterra",
      fr: "Angleterre",
      de: "England",
      it: "Inghilterra",
      ru: "Англия",
      zh: "英格兰",
      cs: "Anglie",
      tr: "İngiltere",
      id: "Inggris",
      vi: "Anh",
    },
    aliases: ["english", "england"],
    flagCode: "GB-ENG",
    selectable: true,
  },
  SCO: {
    code: "SCO",
    names: {
      en: "Scotland",
      es: "Escocia",
      pt: "Escócia",
      fr: "Écosse",
      de: "Schottland",
      it: "Scozia",
      ru: "Шотландия",
      zh: "苏格兰",
      cs: "Skotsko",
      tr: "İskoçya",
      id: "Skotlandia",
      vi: "Scotland",
    },
    aliases: ["scottish", "scotland"],
    flagCode: "GB-SCT",
    selectable: true,
  },
  WAL: {
    code: "WAL",
    names: {
      en: "Wales",
      es: "Gales",
      pt: "País de Gales",
      fr: "Pays de Galles",
      de: "Wales",
      it: "Galles",
      ru: "Уэльс",
      zh: "威尔士",
      cs: "Wales",
      tr: "Galler",
      id: "Wales",
      vi: "Xứ Wales",
    },
    aliases: ["welsh", "wales"],
    flagCode: "GB-WLS",
    selectable: true,
  },
  NIR: {
    code: "NIR",
    names: {
      en: "Northern Ireland",
      es: "Irlanda del Norte",
      pt: "Irlanda do Norte",
      fr: "Irlande du Nord",
      de: "Nordirland",
      it: "Irlanda del Nord",
      ru: "Северная Ирландия",
      zh: "北爱尔兰",
      cs: "Severní Irsko",
      tr: "Kuzey İrlanda",
      id: "Irlandia Utara",
      vi: "Bắc Ireland",
    },
    aliases: ["northern irish", "northern ireland"],
    flagCode: "GB-NIR",
    selectable: true,
  },
  IE: {
    code: "IE",
    names: {
      en: "Republic of Ireland",
      es: "República de Irlanda",
      pt: "República da Irlanda",
      fr: "République d'Irlande",
      de: "Republik Irland",
      it: "Repubblica d'Irlanda",
      ru: "Республика Ирландия",
      zh: "爱尔兰共和国",
      cs: "Irská republika",
      tr: "İrlanda Cumhuriyeti",
      id: "Republik Irlandia",
      vi: "Cộng hòa Ireland",
    },
    aliases: ["irish", "republic of ireland", "ireland"],
    flagCode: "IE",
    selectable: true,
  },
};

const ALIAS_TO_CODE = Object.values(FOOTBALL_IDENTITIES).reduce<Record<string, string>>(
  (map, identity) => {
    for (const alias of identity.aliases) {
      map[alias] = identity.code;
    }
    return map;
  },
  {
    british: "GB",
    uk: "GB",
    "united kingdom": "GB",
    "great britain": "GB",
  },
);

/**
 * The country-name pack for a language code, e.g. `pt-BR` -> `pt`, `zh-CN` -> `zh`.
 *
 * Regional variants the game does not list (`es-MX`) fall back to their base
 * language; anything unknown is English.
 */
function getBaseLocale(locale: string): SupportedLocale {
  if (!locale) return "en";
  const wanted = locale.trim().replace(/_/g, "-").toLowerCase();
  const exact = (Object.keys(LIBRARY_LOCALE_FOR_LANGUAGE) as SupportedLanguageCode[]).find(
    (code) => code.toLowerCase() === wanted,
  );
  if (exact) return LIBRARY_LOCALE_FOR_LANGUAGE[exact];

  const base = wanted.split("-")[0];
  return (SUPPORTED_LOCALES as string[]).includes(base) ? (base as SupportedLocale) : "en";
}

function getFootballIdentity(code: string): FootballIdentityDefinition | undefined {
  return FOOTBALL_IDENTITIES[code.toUpperCase()];
}

function getFootballIdentityName(code: string, locale: string): string | null {
  const identity = getFootballIdentity(code);
  if (!identity) {
    return null;
  }

  return identity.names[getBaseLocale(locale)] ?? identity.names.en;
}

/**
 * Get the localised country name for an ISO alpha-2 code.
 * Falls back to English if the locale doesn't have a translation.
 */
export function countryName(alpha2: string, locale = "en"): string {
  if (!alpha2) return "";
  const normalisedCode = normaliseNationality(alpha2).toUpperCase();
  const footballIdentityName = getFootballIdentityName(normalisedCode, locale);

  if (footballIdentityName) {
    return footballIdentityName;
  }

  const baseLocale = getBaseLocale(locale);
  const name = countries.getName(normalisedCode, baseLocale);
  if (name) return name;
  // Fallback to English
  return countries.getName(normalisedCode, "en") ?? alpha2;
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
 * Get selectable nationalities for football-facing UI.
 * This excludes legacy GB while surfacing the UK football nations explicitly.
 *
 * When `allowedCodes` is given (the backend's combined nation catalog, the
 * single source of truth for what the importer accepts — see #270), the list is
 * restricted to those codes so the UI never offers a nationality that would fail
 * import. When omitted, every ISO nationality is offered (graceful fallback).
 */
export function allNationalities(
  locale = "en",
  allowedCodes?: readonly string[] | null,
): { code: string; name: string }[] {
  const allow =
    allowedCodes && allowedCodes.length > 0
      ? new Set(allowedCodes.map((code) => code.toUpperCase()))
      : null;
  const isAllowed = (code: string): boolean => !allow || allow.has(code.toUpperCase());

  const footballCodes = new Set(
    Object.values(FOOTBALL_IDENTITIES)
      .filter((identity) => identity.selectable)
      .map((identity) => identity.code),
  );

  const isoNationalities = allCountries(locale)
    .filter(({ code }) => code !== "GB" && !footballCodes.has(code) && isAllowed(code))
    .map(({ code }) => ({ code, name: countryName(code, locale) }));

  const footballNationalities = Object.values(FOOTBALL_IDENTITIES)
    .filter((identity) => identity.selectable && isAllowed(identity.code))
    .map((identity) => ({
      code: identity.code,
      name: countryName(identity.code, locale),
    }));

  return [...footballNationalities, ...isoNationalities].sort((a, b) =>
    a.name.localeCompare(b.name, getBaseLocale(locale)),
  );
}

/**
 * Validate that a string is a valid ISO alpha-2 country code.
 */
export function isValidCountryCode(code: string): boolean {
  if (!code) return false;

  const upper = code.toUpperCase();
  if (getFootballIdentity(upper)) {
    return true;
  }

  if (upper.length !== 2) return false;
  return countries.isValid(upper);
}

/**
 * Resolve a nationality value to a valid ISO alpha-2 code that has an SVG flag asset.
 */
export function resolveCountryFlagCode(value: string): string | null {
  const normalisedCode = normaliseNationality(value).toUpperCase();

  const footballIdentity = getFootballIdentity(normalisedCode);
  if (footballIdentity?.flagCode) {
    return footballIdentity.flagCode;
  }

  if (!isValidCountryCode(normalisedCode)) {
    return null;
  }

  return hasFlag(normalisedCode) ? normalisedCode : null;
}

/**
 * Map from old demonym-style nationality strings to ISO alpha-2 codes.
 * Used for backward compatibility with older save files.
 */
const DEMONYM_TO_CODE: Record<string, string> = {
  English: "ENG",
  British: "GB",
  Scottish: "SCO",
  Welsh: "WAL",
  Irish: "IE",
  "Northern Irish": "NIR",
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
  const trimmed = value.trim();
  const upper = trimmed.toUpperCase();
  if (getFootballIdentity(upper)) return upper;
  // Already a valid 2-letter code?
  if (upper.length === 2 && countries.isValid(upper)) return upper;
  const alias = ALIAS_TO_CODE[trimmed.toLowerCase()];
  if (alias) return alias;
  // Try demonym map
  const demonymCode = DEMONYM_TO_CODE[trimmed];
  if (demonymCode) return demonymCode;

  if (/^[A-Za-z]{3}$/.test(trimmed)) {
    return trimmed;
  }

  for (const locale of SUPPORTED_LOCALES) {
    const alpha2 = countries.getAlpha2Code(trimmed, locale);
    if (alpha2) {
      return alpha2;
    }
  }

  return trimmed;
}

export { countries };
