import i18n from "i18next";
import { initReactI18next } from "react-i18next";

export const SUPPORTED_LANGUAGES = [
  { code: "en", label: "English" },
  { code: "es", label: "Español" },
  { code: "pt", label: "Português" },
  { code: "fr", label: "Français" },
  { code: "de", label: "Deutsch" },
  { code: "it", label: "Italiano" },
  { code: "pt-BR", label: "Português (Brasil)" },
] as const;

type SupportedLanguageCode = (typeof SUPPORTED_LANGUAGES)[number]["code"];
type TranslationResource = Record<string, unknown>;

const SUPPORTED_CODES: SupportedLanguageCode[] = SUPPORTED_LANGUAGES.map(
  (language) => language.code,
);
const localeLoaders = import.meta.glob<{ default: TranslationResource }>(
  "./locales/*.json",
);

/** Detect the best initial language from the browser / OS locale */
function detectInitialLanguage(): string {
  const nav = navigator.language; // e.g. "pt-BR", "es-419", "en-US"
  // Exact match first (e.g. pt-BR)
  if (SUPPORTED_CODES.includes(nav as SupportedLanguageCode)) return nav;
  // Base language match (e.g. "es-419" -> "es")
  const base = nav.split("-")[0];
  if (SUPPORTED_CODES.includes(base as SupportedLanguageCode)) return base;
  return "en";
}

function normalizeLanguageCode(language: string): SupportedLanguageCode {
  if (SUPPORTED_CODES.includes(language as SupportedLanguageCode)) {
    return language as SupportedLanguageCode;
  }

  const baseLanguage = language.split("-")[0] as SupportedLanguageCode;

  if (SUPPORTED_CODES.includes(baseLanguage)) {
    return baseLanguage;
  }

  return "en";
}

async function loadLanguageBundle(
  language: SupportedLanguageCode,
): Promise<TranslationResource> {
  const loader = localeLoaders[`./locales/${language}.json`];

  if (!loader) {
    throw new Error(`Missing locale loader for ${language}`);
  }

  const module = await loader();
  return module.default;
}

async function ensureLanguageResources(
  language: string,
): Promise<SupportedLanguageCode> {
  const normalizedLanguage = normalizeLanguageCode(language);

  if (!i18n.hasResourceBundle(normalizedLanguage, "translation")) {
    i18n.addResourceBundle(
      normalizedLanguage,
      "translation",
      await loadLanguageBundle(normalizedLanguage),
      true,
      true,
    );
  }

  return normalizedLanguage;
}

const originalChangeLanguage = i18n.changeLanguage.bind(i18n);

export const i18nReady = (async () => {
  const fallbackLanguage: SupportedLanguageCode = "en";
  const initialLanguage = normalizeLanguageCode(detectInitialLanguage());
  const resources: Record<string, { translation: TranslationResource }> = {
    [fallbackLanguage]: {
      translation: await loadLanguageBundle(fallbackLanguage),
    },
  };

  if (initialLanguage !== fallbackLanguage) {
    resources[initialLanguage] = {
      translation: await loadLanguageBundle(initialLanguage),
    };
  }

  await i18n.use(initReactI18next).init({
    resources,
    lng: initialLanguage,
    fallbackLng: fallbackLanguage,
    interpolation: {
      escapeValue: false, // React already escapes
    },
  });

  return i18n;
})();

export async function changeAppLanguage(language: string): Promise<string> {
  await i18nReady;
  const normalizedLanguage = await ensureLanguageResources(
    language || detectInitialLanguage(),
  );
  await originalChangeLanguage(normalizedLanguage);
  return normalizedLanguage;
}

export default i18n;
