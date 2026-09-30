import { describe, expect, it } from "vitest";

import { LOCALE_FILES, NON_ENGLISH_LOCALES, type LocaleTree } from "./i18nTestHelpers";

// English adds an ordinal suffix (1st, 2nd, ...) to these messages. The listed
// languages express ordinal position without that interpolation. Keep the
// exception tied to each key and locale so a different omission still fails.
const REVIEW_ORDINAL_LOCALES = ["cs", "de", "es", "fr", "id", "it", "pt", "pt-BR", "tr"];
const ALLOWED_SUFFIX_OMISSIONS: Record<string, readonly string[]> = {
  "be.msg.seasonReview.body.topFour": REVIEW_ORDINAL_LOCALES,
  "be.msg.seasonReview.body.midTable": REVIEW_ORDINAL_LOCALES,
  "be.msg.seasonReview.body.lowerHalf": REVIEW_ORDINAL_LOCALES,
  "be.msg.seasonPayout.ledgerDescription": ["id"],
};

function placeholders(text: string): Set<string> {
  return new Set([...text.matchAll(/\{\{\s*([^{}]+?)\s*\}\}/g)].map((match) => match[1].trim()));
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
    const missing = [...expected].filter(
      (name) =>
        !actual.has(name) &&
        !(name === "suffix" && ALLOWED_SUFFIX_OMISSIONS[key]?.includes(locale)),
    );
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

  it("allows only the documented ordinal omission", () => {
    const en = {
      be: { msg: { seasonReview: { body: { topFour: "{{position}}{{suffix}} {{points}}" } } } },
    };
    const allowed = {
      be: { msg: { seasonReview: { body: { topFour: "{{position}} {{points}}" } } } },
    };
    const wrongLocale = placeholderDifferences(en, allowed, "ru");
    expect(placeholderDifferences(en, allowed, "fr")).toEqual([]);
    expect(wrongLocale).toEqual(["be.msg.seasonReview.body.topFour: missing [suffix], extra []"]);
    expect(
      placeholderDifferences(
        en,
        { be: { msg: { seasonReview: { body: { topFour: "{{position}} {{surprise}}" } } } } },
        "fr",
      ),
    ).toEqual(["be.msg.seasonReview.body.topFour: missing [points], extra [surprise]"]);
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
