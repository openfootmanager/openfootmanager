import { describe, expect, it } from "vitest";

import en from "../../i18n/locales/en.json";

/**
 * `frontendKeyCoverage.test.ts` parses the TypeScript AST for literal `t("…")` calls, so it cannot
 * see a key assembled from a template literal. `ReportBugModal` builds several that way — one per
 * step, one per field, one per frequency — which means a typo in any of them would ship as a raw
 * key printed on screen, with every gate green.
 *
 * This lists them by hand. It is duplication, and it is the only thing standing between a renamed
 * key and a modal that says `reportBug.previewTitle` to the player.
 */
const DYNAMIC_KEYS = [
  // `t(`reportBug.${step}Title`)` and `…Intro`
  "reportBug.describeTitle",
  "reportBug.describeIntro",
  "reportBug.previewTitle",
  "reportBug.previewIntro",
  "reportBug.doneTitle",
  "reportBug.doneIntro",
  "reportBug.sentTitle",
  "reportBug.sentIntro",
  // `t(`reportBug.${key}`)` and `…Placeholder`
  "reportBug.whatHappened",
  "reportBug.whatHappenedPlaceholder",
  "reportBug.expected",
  "reportBug.expectedPlaceholder",
  "reportBug.steps",
  "reportBug.stepsPlaceholder",
  // `t(`reportBug.frequency.${value}`)`
  "reportBug.frequency.everyTime",
  "reportBug.frequency.sometimes",
  "reportBug.frequency.once",
];

function lookup(key: string): unknown {
  return key
    .split(".")
    .reduce<unknown>(
      (node, part) =>
        typeof node === "object" && node !== null
          ? (node as Record<string, unknown>)[part]
          : undefined,
      en,
    );
}

describe("ReportBugModal dynamic translation keys", () => {
  it.each(DYNAMIC_KEYS)("%s exists in en.json", (key) => {
    const value = lookup(key);

    expect(typeof value).toBe("string");
    expect(value).not.toBe("");
  });
});
