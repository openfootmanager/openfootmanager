import i18n from "i18next";
import { initReactI18next } from "react-i18next";
import en from "./locales/en.json";
import es from "./locales/es.json";
import pt from "./locales/pt.json";
import fr from "./locales/fr.json";
import de from "./locales/de.json";
import ptBR from "./locales/pt-BR.json";
import it from "./locales/it.json";
import tr from "./locales/tr.json";

export const SUPPORTED_LANGUAGES = [
  { code: "en", label: "English" },
  { code: "es", label: "Español" },
  { code: "pt", label: "Português" },
  { code: "fr", label: "Français" },
  { code: "de", label: "Deutsch" },
  { code: "it", label: "Italiano" },
  { code: "tr", label: "Türkçe" },
  { code: "pt-BR", label: "Português (Brasil)" },
] as const;

const SUPPORTED_CODES: string[] = SUPPORTED_LANGUAGES.map(l => l.code);

/** Detect the best initial language from the browser / OS locale */
function detectInitialLanguage(): string {
  const nav = navigator.language; // e.g. "pt-BR", "es-419", "en-US"
  // Exact match first (e.g. pt-BR)
  if (SUPPORTED_CODES.includes(nav)) return nav;
  // Base language match (e.g. "es-419" -> "es")
  const base = nav.split("-")[0];
  if (SUPPORTED_CODES.includes(base)) return base;
  return "en";
}

i18n.use(initReactI18next).init({
  resources: {
    en: { translation: en },
    es: { translation: es },
    pt: { translation: pt },
    fr: { translation: fr },
    de: { translation: de },
    it: { translation: it },
    tr: { translation: tr },
    "pt-BR": { translation: ptBR },
  },
  lng: detectInitialLanguage(),
  fallbackLng: "en",
  interpolation: {
    escapeValue: false, // React already escapes
  },
});

export default i18n;
