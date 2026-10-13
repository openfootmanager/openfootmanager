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
    const offenders = SUPPORTED_LANGUAGES.flatMap(({ code }) =>
      strings(localeModules[`./locales/${code}.json`])
        .filter(([, value]) => /[€$£]\s?\{\{/.test(value))
        .map(([key]) => `${code}: ${key}`),
    );

    expect(offenders).toEqual([]);
  });
});
