import { describe, expect, it } from "vitest";

import { SUPPORTED_LANGUAGES } from "./index";

const localeModules = import.meta.glob<Record<string, unknown>>("./locales/*.json", {
  eager: true,
  import: "default",
});

function strings(node: unknown, path = ""): [string, string][] {
  if (typeof node === "string") return [[path, node]];
  if (node === null || typeof node !== "object") return [];
  return Object.entries(node).flatMap(([key, child]) =>
    strings(child, path ? `${path}.${key}` : key),
  );
}

describe("money in translations", () => {
  /**
   * Given a string that takes an amount as a parameter
   * When any locale writes a currency symbol in front of it
   * Then the test fails: the amount arrives already formatted, with the player's own currency
   *      symbol, so a literal "€" would print "€€12K", or the wrong currency, to a player on pounds
   *      or dollars.
   */
  it("never writes a currency symbol before a money parameter", () => {
    // A locale missing from the glob would scan as empty and pass: prove the scan saw every
    // language, and a believable amount of text, before trusting an empty result.
    const scanned = SUPPORTED_LANGUAGES.map(({ code }) => ({
      code,
      entries: strings(localeModules[`./locales/${code}.json`]),
    }));
    for (const { code, entries } of scanned) {
      expect(entries.length, `${code} was not scanned`).toBeGreaterThan(1000);
    }

    const offenders = scanned.flatMap(({ code, entries }) =>
      entries.filter(([, value]) => /\p{Sc}\s?\{\{/u.test(value)).map(([key]) => `${code}: ${key}`),
    );

    expect(offenders).toEqual([]);
  });
});
