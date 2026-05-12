import { describe, expect, it } from "vitest";

import { collectMissingKeys, type LocaleTree } from "./i18nTestHelpers";
import de from "./locales/de.json";
import en from "./locales/en.json";
import es from "./locales/es.json";
import fr from "./locales/fr.json";
import itLocale from "./locales/it.json";
import ptBR from "./locales/pt-BR.json";
import pt from "./locales/pt.json";
import zhCN from "./locales/zh-CN.json";

const LOCALES: Record<string, LocaleTree> = {
  de,
  es,
  fr,
  it: itLocale,
  pt,
  "pt-BR": ptBR,
  "zh-CN": zhCN,
};

describe("locale coverage", () => {
  it("keeps every supported locale aligned with English translation keys", () => {
    const missingKeysByLocale = Object.entries(LOCALES).reduce<
      Record<string, string[]>
    >((accumulator, [localeCode, translations]) => {
      const missingKeys = collectMissingKeys(en, translations);

      if (missingKeys.length > 0) {
        accumulator[localeCode] = missingKeys;
      }

      return accumulator;
    }, {});

    expect(missingKeysByLocale).toEqual({});
  });
});
