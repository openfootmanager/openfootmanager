import { describe, expect, it } from "vitest";

// Every non-test source file, as text.
const SOURCES = import.meta.glob(
  ["../**/*.{ts,tsx}", "!../**/*.test.{ts,tsx}", "!../**/generated/**"],
  {
    query: "?raw",
    import: "default",
    eager: true,
  },
) as Record<string, string>;

/**
 * A backend `*_key` field (`feedback.headline_key`, `status.label_key`) names text the backend
 * chose. It goes through `resolveBackendText(key, fallback, params)`, which falls back to the
 * given text when a locale has no entry, never straight into `t(...)`, which would draw the raw
 * key. A `defaultValue` that is itself the key is the same leak.
 *
 * `name_key` of a competition is the one named exception: `competitionDisplayName` passes the
 * stored name as the default, so a missing key shows the name.
 */
const EXCEPTIONS: Record<string, string> = {
  "../lib/competitionName.ts": "name_key with the stored name as its default",
};

/** `t(` or `i18n.t(` whose first argument ends in a snake_case `_key` field. */
const DIRECT_KEY_CALL = /\b(?:i18n\.)?t\(\s*[A-Za-z0-9_$.?!]*\b[a-z0-9]+_key\b/;
/** `defaultValue:` given a `*_key` field: the raw key as the fallback. */
const KEY_AS_DEFAULT = /defaultValue:\s*[A-Za-z0-9_$.?!]*\b[a-z0-9]+_key\b/;

function offenders(): string[] {
  return Object.entries(SOURCES).flatMap(([path, source]) => {
    if (EXCEPTIONS[path]) return [];
    return source.split("\n").flatMap((line, index) => {
      const code = line.replace(/\/\/.*$/, "");
      return DIRECT_KEY_CALL.test(code) || KEY_AS_DEFAULT.test(code)
        ? [`${path.replace("../", "src/")}:${index + 1}: ${line.trim()}`]
        : [];
    });
  });
}

describe("backend key fields reach the player through the resolver", () => {
  /**
   * Given every source file
   * When it asks `t(...)` for a backend `*_key` field, or gives one as a `defaultValue`
   * Then it is listed: a missing key there would be drawn raw, as the digest once did.
   */
  it("has no t(<backend *_key>) call and no *_key default value", () => {
    expect(offenders()).toEqual([]);
  });

  /**
   * Given the shapes the guard exists for
   * When they are matched
   * Then each is caught, and the sanctioned forms are not.
   */
  it.each([
    ["{t(feedback.headline_key, { a: 1 })}", true],
    ["{t(report.rating_key)} (~1)", true],
    ["i18n.t(status.label_key)", true],
    ["t(x, { defaultValue: feedback.headline_key })", true],
    ["resolveBackendText(feedback.headline_key, '', {})", false],
    ["t(labelKey)", false],
    ['t("scouting.confidence")', false],
    ["const x = status.label_key;", false],
  ])("matches %s: %s", (line, matched) => {
    const hit = DIRECT_KEY_CALL.test(line) || KEY_AS_DEFAULT.test(line);
    expect(hit).toBe(matched);
  });
});
