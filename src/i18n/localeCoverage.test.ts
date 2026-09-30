import { describe, expect, it } from "vitest";

import {
  collectMissingKeys,
  collectOrphanKeys,
  collectUntranslatedKeys,
  LOCALE_FILES,
  NON_ENGLISH_LOCALES,
} from "./i18nTestHelpers";
import { SUPPORTED_LANGUAGES } from "./index";
import INTENTIONAL_SAME from "./INTENTIONAL_SAME.json";

const en = LOCALE_FILES.en;
const LOCALES = NON_ENGLISH_LOCALES;

describe("supported languages", () => {
  // The registry had no gate at all. A thirteenth language could be added to
  // SUPPORTED_LANGUAGES and shipped with a one-key file, and every locale suite
  // still passed: the picker offered it and the app rendered English. Both
  // directions are asserted, because a file with no registry entry is dead
  // weight nobody can select, and a registry entry with no file is a language
  // the picker offers and cannot load.
  it("names exactly the locales that have a file", () => {
    const registered = SUPPORTED_LANGUAGES.map((language) => language.code).sort();
    const onDisk = Object.keys(LOCALE_FILES).sort();

    expect(onDisk).toEqual(registered);
  });
});

describe("locale coverage", () => {
  it("keeps every supported locale aligned with English translation keys", () => {
    const missingKeysByLocale = Object.entries(LOCALES).reduce<Record<string, string[]>>(
      (accumulator, [localeCode, translations]) => {
        const missingKeys = collectMissingKeys(en, translations);

        if (missingKeys.length > 0) {
          accumulator[localeCode] = missingKeys;
        }

        return accumulator;
      },
      {},
    );

    expect(missingKeysByLocale).toEqual({});
  });

  it("carries no keys that English does not have", () => {
    const orphanKeysByLocale = Object.entries(LOCALES).reduce<Record<string, string[]>>(
      (accumulator, [localeCode, translations]) => {
        const orphanKeys = collectOrphanKeys(en, translations);

        if (orphanKeys.length > 0) {
          accumulator[localeCode] = orphanKeys;
        }

        return accumulator;
      },
      {},
    );

    expect(orphanKeysByLocale).toEqual({});
  });

  it("has no untranslated strings (only explicitly allowed same-language exceptions)", () => {
    const intentionalSame = INTENTIONAL_SAME as Record<string, string[]>;
    const globalExceptions = new Set(intentionalSame.global ?? []);

    const violationsByLocale = Object.entries(LOCALES).reduce<Record<string, string[]>>(
      (accumulator, [localeCode, translations]) => {
        const localeExceptions = new Set(intentionalSame[localeCode] ?? []);
        const untranslated = collectUntranslatedKeys(en, translations);
        const violations = untranslated.filter(
          (key) => !globalExceptions.has(key) && !localeExceptions.has(key),
        );

        if (violations.length > 0) {
          accumulator[localeCode] = violations;
        }

        return accumulator;
      },
      {},
    );

    expect(violationsByLocale).toEqual({});
  });
});
