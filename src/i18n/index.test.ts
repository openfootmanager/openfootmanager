import { afterAll, describe, expect, it } from "vitest";
import i18n, { changeAppLanguage, i18nReady, resolveSupportedLanguage } from "./index";

describe("resolveSupportedLanguage", () => {
  it("maps Simplified Chinese locale variants to zh-CN", () => {
    expect(resolveSupportedLanguage("zh")).toBe("zh-CN");
    expect(resolveSupportedLanguage("zh-CN")).toBe("zh-CN");
    expect(resolveSupportedLanguage("zh-Hans")).toBe("zh-CN");
    expect(resolveSupportedLanguage("zh-Hans-CN")).toBe("zh-CN");
    expect(resolveSupportedLanguage("ZH_hans_cn")).toBe("zh-CN");
  });

  it("keeps existing exact and base language matching behavior", () => {
    expect(resolveSupportedLanguage("PT-BR")).toBe("pt-BR");
    expect(resolveSupportedLanguage("ru-RU")).toBe("ru");
    expect(resolveSupportedLanguage("es-419")).toBe("es");
    expect(resolveSupportedLanguage("en-US")).toBe("en");
  });

  it("falls back to English for unsupported locales", () => {
    expect(resolveSupportedLanguage("nl-NL")).toBe("en");
    expect(resolveSupportedLanguage("zh-Hant-TW")).toBe("en");
  });
});

describe("i18n lazy loading", () => {
  afterAll(async () => {
    await changeAppLanguage("en");
  });

  it("initializes with the active language resources instead of all locales", async () => {
    await i18nReady;

    expect(i18n.hasResourceBundle("en", "translation")).toBe(true);
    expect(i18n.hasResourceBundle("pt-BR", "translation")).toBe(false);
    expect(i18n.hasResourceBundle("zh-CN", "translation")).toBe(false);
  });

  it("loads a locale bundle on demand when the app language changes", async () => {
    await i18nReady;

    await changeAppLanguage("ru-RU");

    expect(i18n.language).toBe("ru");
    expect(i18n.hasResourceBundle("ru", "translation")).toBe(true);
  });
});
