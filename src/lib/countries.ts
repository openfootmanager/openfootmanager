/**
 * Country / nationality utilities powered by i18n-iso-countries.
 *
 * The app still accepts ISO alpha-2 codes for most countries, but also supports
 * football-specific identities where the sport diverges from ISO country data.
 */
import countries from "i18n-iso-countries";
import { hasFlag } from "country-flag-icons";
import enLocale from "i18n-iso-countries/langs/en.json";
import esLocale from "i18n-iso-countries/langs/es.json";
import ptLocale from "i18n-iso-countries/langs/pt.json";
import frLocale from "i18n-iso-countries/langs/fr.json";
import deLocale from "i18n-iso-countries/langs/de.json";
import itLocale from "i18n-iso-countries/langs/it.json";
import ruLocale from "i18n-iso-countries/langs/ru.json";
import zhLocale from "i18n-iso-countries/langs/zh.json";

const SUPPORTED_LOCALES = ["en", "es", "pt", "fr", "de", "it", "ru", "zh"] as const;

type SupportedLocale = typeof SUPPORTED_LOCALES[number];

const REGISTERED_LOCALES = [
  enLocale,
  esLocale,
  ptLocale,
  frLocale,
  deLocale,
  itLocale,
  ruLocale,
  zhLocale,
];

for (const locale of REGISTERED_LOCALES) {
  countries.registerLocale(locale);
}

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

function getBaseLocale(locale: string): string {
  if (!locale) return "en";
  // Convert 'pt-BR' to 'pt'
  return locale.split('-')[0].toLowerCase();
}

function getFootballIdentity(code: string): FootballIdentityDefinition | undefined {
  return FOOTBALL_IDENTITIES[code.toUpperCase()];
}

function getFootballIdentityName(code: string, locale: string): string | null {
  const identity = getFootballIdentity(code);
  if (!identity) {
    return null;
  }

  const baseLocale = getBaseLocale(locale) as SupportedLocale;
  return identity.names[baseLocale] ?? identity.names.en;
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
 */
export function allNationalities(locale = "en"): { code: string; name: string }[] {
  const footballCodes = new Set(
    Object.values(FOOTBALL_IDENTITIES)
      .filter((identity) => identity.selectable)
      .map((identity) => identity.code),
  );

  const isoNationalities = allCountries(locale)
    .filter(({ code }) => code !== "GB" && !footballCodes.has(code))
    .map(({ code }) => ({ code, name: countryName(code, locale) }));

  const footballNationalities = Object.values(FOOTBALL_IDENTITIES)
    .filter((identity) => identity.selectable)
    .map((identity) => ({
      code: identity.code,
      name: countryName(identity.code, locale),
    }));

  return [...footballNationalities, ...isoNationalities]
    .sort((a, b) => a.name.localeCompare(b.name, getBaseLocale(locale)));
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
