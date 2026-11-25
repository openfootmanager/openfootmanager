import { describe, expect, it } from "vitest";

import { SUPPORTED_LANGUAGES, type SupportedLanguageCode } from "../i18n";
import { formatDate, formatDateFull, formatDateShort, getLocale } from "./dateFormatting";

describe("getLocale", () => {
  // Every code in SUPPORTED_LANGUAGES, with its expected tag written out. The
  // type on LANG_LOCALE catches a *missing* language; only this catches a wrong
  // tag, which is the failure that actually reached users — the map was keyed on
  // `zh` where the registry says `zh-CN`, and the raw-code fallback hid it.
  const EXPECTED_TAGS: Record<SupportedLanguageCode, string> = {
    en: "en-GB",
    es: "es-ES",
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
  };

  it("maps every language the game ships", () => {
    for (const { code } of SUPPORTED_LANGUAGES) {
      expect(getLocale(code), `${code} resolves to the wrong tag`).toBe(EXPECTED_TAGS[code]);
    }
  });

  it("covers the registry exactly, so a new language cannot slip past this test", () => {
    expect(Object.keys(EXPECTED_TAGS).sort()).toEqual(
      SUPPORTED_LANGUAGES.map((language) => language.code).sort(),
    );
  });

  it("returns input for unknown codes", () => {
    expect(getLocale("ja")).toBe("ja");
  });

  it("returns the English tag for undefined", () => {
    expect(getLocale(undefined)).toBe("en-GB");
  });
});

describe("English dates read day-month-year", () => {
  /**
   * Given a day, in English (the maintainer's ruling: English dates are day-month-year everywhere)
   * When it is written in full, in the long form with a weekday, in the short form, and with the
   *      options the profile passes
   * Then the day comes before the month in each, as on the design boards
   */
  it("puts the day before the month in every English form", () => {
    expect(formatDate("2028-06-30", "en")).toBe("30 June 2028");
    expect(
      formatDate("2028-06-30", "en", { day: "numeric", month: "short", year: "numeric" }),
    ).toBe("30 Jun 2028");
    expect(formatDateShort("2028-06-30", "en")).toBe("30 Jun");
    expect(formatDateFull("2028-06-30", "en")).toMatch(/^Friday,? 30 June 2028$/);
  });

  /** Given no language, then the same English order: the fallback and the language agree. */
  it("writes a day with no language the same way", () => {
    expect(formatDate("2028-06-30")).toBe("30 June 2028");
  });

  /** Given other languages, then their orders are left as they were. */
  it("leaves the other languages' orders alone", () => {
    expect(formatDate("2028-06-30", "de")).toBe("30. Juni 2028");
    expect(formatDate("2028-06-30", "es")).toBe("30 de junio de 2028");
    expect(formatDate("2028-06-30", "pt-BR")).toBe("30 de junho de 2028");
  });
});
