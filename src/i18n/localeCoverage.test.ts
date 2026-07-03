import { describe, expect, it } from "vitest";

import {
  collectMissingKeys,
  collectUntranslatedKeys,
  type LocaleTree,
} from "./i18nTestHelpers";
import INTENTIONAL_SAME from "./INTENTIONAL_SAME.json";
import cs from "./locales/cs.json";
import de from "./locales/de.json";
import en from "./locales/en.json";
import es from "./locales/es.json";
import fr from "./locales/fr.json";
import itLocale from "./locales/it.json";
import ptBR from "./locales/pt-BR.json";
import pt from "./locales/pt.json";
import ru from "./locales/ru.json";
import tr from "./locales/tr.json";
import zhCN from "./locales/zh-CN.json";

const LOCALES: Record<string, LocaleTree> = {
  cs,
  de,
  es,
  fr,
  it: itLocale,
  pt,
  "pt-BR": ptBR,
  ru,
  tr,
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

  it("has no untranslated strings (only explicitly allowed same-language exceptions)", () => {
    const intentionalSame = INTENTIONAL_SAME as Record<string, string[]>;
    const globalExceptions = new Set(intentionalSame["global"] ?? []);

    const violationsByLocale = Object.entries(LOCALES).reduce<
      Record<string, string[]>
    >((accumulator, [localeCode, translations]) => {
      const localeExceptions = new Set(intentionalSame[localeCode] ?? []);
      const untranslated = collectUntranslatedKeys(en, translations);
      const violations = untranslated.filter(
        (key) => !globalExceptions.has(key) && !localeExceptions.has(key),
      );

      if (violations.length > 0) {
        accumulator[localeCode] = violations;
      }

      return accumulator;
    }, {});

    expect(violationsByLocale).toEqual({});
  });
});
