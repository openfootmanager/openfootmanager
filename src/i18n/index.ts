import i18n from "i18next";
import { initReactI18next } from "react-i18next";
import en from "./locales/en.json";
import es from "./locales/es.json";
import pt from "./locales/pt.json";
import fr from "./locales/fr.json";
import de from "./locales/de.json";
import ptBR from "./locales/pt-BR.json";
import it from "./locales/it.json";
import zhCN from "./locales/zh-CN.json";

export const SUPPORTED_LANGUAGES = [
  { code: "en", labelKey: "settings.languages.en" },
  { code: "es", labelKey: "settings.languages.es" },
  { code: "pt", labelKey: "settings.languages.pt" },
  { code: "fr", labelKey: "settings.languages.fr" },
  { code: "de", labelKey: "settings.languages.de" },
  { code: "it", labelKey: "settings.languages.it" },
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

/** Detect the best initial language from the browser / OS locale */
function detectInitialLanguage(): string {
  const nav = navigator.language; // e.g. "pt-BR", "es-419", "en-US"
  return resolveSupportedLanguage(nav);
}

i18n.use(initReactI18next).init({
  resources: {
    en: { translation: en },
    es: { translation: es },
    pt: { translation: pt },
    fr: { translation: fr },
    de: { translation: de },
    it: { translation: it },
    "pt-BR": { translation: ptBR },
    "zh-CN": { translation: zhCN },
  },
  lng: detectInitialLanguage(),
  fallbackLng: "en",
  interpolation: {
    escapeValue: false, // React already escapes
  },
});

export default i18n;
