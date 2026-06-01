import i18n from "i18next";
import { initReactI18next } from "react-i18next";
import resourcesToBackend from "i18next-resources-to-backend";

export const SUPPORTED_LANGUAGES = [
  { code: "en", labelKey: "settings.languages.en" },
  { code: "es", labelKey: "settings.languages.es" },
  { code: "pt", labelKey: "settings.languages.pt" },
  { code: "fr", labelKey: "settings.languages.fr" },
  { code: "de", labelKey: "settings.languages.de" },
  { code: "it", labelKey: "settings.languages.it" },
  { code: "ru", labelKey: "settings.languages.ru" },
  { code: "pt-BR", labelKey: "settings.languages.ptBR" },
  { code: "zh-CN", labelKey: "settings.languages.zhCN" },
] as const;

const SUPPORTED_CODES = new Map(
  SUPPORTED_LANGUAGES.map((language) => [
    language.code.toLowerCase(),
    language.code,
  ]),
);

const SIMPLIFIED_CHINESE_LOCALES = new Set(["zh", "zh-cn", "zh-sg", "zh-my"]);

type TranslationResource = Record<string, unknown>;

const localeModules = import.meta.glob<{ default: TranslationResource }>(
  "./locales/*.json",
);

const SUPPORTED_LANGUAGE_CODES = SUPPORTED_LANGUAGES.map(
  ({ code }) => code,
);

function localeModulePath(language: string): string {
  return `./locales/${language}.json`;
}

function localeBackendLoader(language: string): Promise<TranslationResource> {
  const resolvedLanguage = resolveSupportedLanguage(language);
  const loader = localeModules[localeModulePath(resolvedLanguage)];

  if (!loader) {
    return Promise.reject(
      new Error(`Unsupported locale module: ${resolvedLanguage}`),
    );
  }

  return loader().then((module) => module.default);
}

export function resolveSupportedLanguage(locale: string): string {
  const normalized = locale.trim().replace(/_/g, "-").toLowerCase();
  const exactMatch = SUPPORTED_CODES.get(normalized);
  if (exactMatch) return exactMatch;

  if (
    SIMPLIFIED_CHINESE_LOCALES.has(normalized) ||
    normalized.startsWith("zh-hans")
  ) {
    return "zh-CN";
  }

  const base = normalized.split("-")[0];
  return SUPPORTED_CODES.get(base) ?? "en";
}

/**
 * Detect the best initial language from the runtime locale.
 *
 * Browser environments: uses `navigator.language` (for example: "pt-BR", "es-419", "en-US").
 * Non-browser environments (SSR/tests/Node): falls back to `"en"` when `navigator`
 * is unavailable or does not expose a valid language string.
 *
 * Any provided locale is normalized and mapped by `resolveSupportedLanguage`,
 * which handles region/script variants and unsupported values.
 */
function detectInitialLanguage(): string {
  const navLanguage =
    typeof navigator !== "undefined" && typeof navigator.language === "string"
      ? navigator.language
      : "en";
  return resolveSupportedLanguage(navLanguage);
}

export async function changeAppLanguage(locale: string): Promise<string> {
  const resolvedLanguage = resolveSupportedLanguage(locale);
  await i18n.changeLanguage(resolvedLanguage);
  return resolvedLanguage;
}

export const i18nReady = i18n
  .use(resourcesToBackend(localeBackendLoader))
  .use(initReactI18next)
  .init({
    resources: {},
    partialBundledLanguages: true,
    supportedLngs: SUPPORTED_LANGUAGE_CODES,
    lng: detectInitialLanguage(),
    fallbackLng: "en",
    defaultNS: "translation",
    ns: ["translation"],
    interpolation: {
      escapeValue: false,
    },
    react: {
      useSuspense: false,
    },
  });

export default i18n;
