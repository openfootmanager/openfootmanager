import { describe, expect, it } from "vitest";

import { LOCALE_FILES, NON_ENGLISH_LOCALES, type LocaleTree } from "./i18nTestHelpers";

// A finishing place is written either as the bare number (`{{position}}`) or as the language's own
// ordinal form of it (`{{ordinal}}`, from `formatOrdinal` in `resolveParamValues`): which one reads
// naturally differs by language ("21st place", "на 21-м месте", "第21位"). Both name the same
// value, so a locale may use either; the backend sends only `position`, never an English suffix.
const PLACE_ALIASES: Record<string, string> = { ordinal: "position" };

function placeholders(text: string): Set<string> {
  return new Set(
    [...text.matchAll(/\{\{\s*([^{}]+?)\s*\}\}/g)].map(
      (match) => PLACE_ALIASES[match[1].trim()] ?? match[1].trim(),
    ),
  );
}

function placeholderDifferences(
  reference: LocaleTree,
  candidate: LocaleTree,
  locale: string,
  path: string[] = [],
): string[] {
  return Object.entries(reference).flatMap(([segment, englishValue]) => {
    const nextPath = [...path, segment];
    const key = nextPath.join(".");
    const translatedValue = candidate[segment];

    if (englishValue !== null && typeof englishValue === "object" && !Array.isArray(englishValue)) {
      return placeholderDifferences(
        englishValue as LocaleTree,
        translatedValue !== null &&
          typeof translatedValue === "object" &&
          !Array.isArray(translatedValue)
          ? (translatedValue as LocaleTree)
          : {},
        locale,
        nextPath,
      );
    }
    if (typeof englishValue !== "string") return [];

    const expected = placeholders(englishValue);
    const actual = placeholders(typeof translatedValue === "string" ? translatedValue : "");
    const missing = [...expected].filter((name) => !actual.has(name));
    const extra = [...actual].filter((name) => !expected.has(name));
    return missing.length > 0 || extra.length > 0
      ? [`${key}: missing [${missing.join(", ")}], extra [${extra.join(", ")}]`]
      : [];
  });
}

describe("locale interpolation placeholders", () => {
  it("checks missing and invented placeholders in both directions", () => {
    const en = { message: "{{first}} and {{second}}" };
    expect(placeholderDifferences(en, { message: "{{first}} and {{third}}" }, "fr")).toEqual([
      "message: missing [second], extra [third]",
    ]);
  });

  /**
   * Given an English template that writes the place as its ordinal
   * When a locale writes it as the bare number, or as its own ordinal
   * Then both are the same value and match; leaving the place out, or inventing a placeholder, is
   *      still a difference.
   */
  it("treats the place as the same value whether it is written bare or as an ordinal", () => {
    const tree = (text: string) => ({
      be: { msg: { seasonReview: { body: { topFour: text } } } },
    });
    const en = tree("{{ordinal}} {{points}}");

    expect(placeholderDifferences(en, tree("{{position}} {{points}}"), "ru")).toEqual([]);
    expect(placeholderDifferences(en, tree("{{ordinal}} {{points}}"), "fr")).toEqual([]);
    expect(placeholderDifferences(en, tree("{{points}}"), "ru")).toEqual([
      "be.msg.seasonReview.body.topFour: missing [position], extra []",
    ]);
    expect(placeholderDifferences(en, tree("{{position}} {{surprise}}"), "fr")).toEqual([
      "be.msg.seasonReview.body.topFour: missing [points], extra [surprise]",
    ]);
  });

  it("matches every locale's placeholder set to English", () => {
    const differences = Object.fromEntries(
      Object.entries(NON_ENGLISH_LOCALES)
        .map(([locale, translations]) => [
          locale,
          placeholderDifferences(LOCALE_FILES.en, translations, locale),
        ])
        .filter(([, mismatches]) => (mismatches as string[]).length > 0),
    );
    expect(differences).toEqual({});
  });
});
