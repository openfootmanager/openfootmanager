import { describe, expect, it } from "vitest";
import { resolveSupportedLanguage } from "./index";

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
    expect(resolveSupportedLanguage("es-419")).toBe("es");
    expect(resolveSupportedLanguage("en-US")).toBe("en");
  });

  it("falls back to English for unsupported locales", () => {
    expect(resolveSupportedLanguage("nl-NL")).toBe("en");
    expect(resolveSupportedLanguage("zh-Hant-TW")).toBe("en");
  });
});
