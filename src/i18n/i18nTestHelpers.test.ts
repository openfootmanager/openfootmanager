import { describe, expect, it } from "vitest";

import { collectOrphanKeys } from "./i18nTestHelpers";

/**
 * The locale-fixture assertion in `localeCoverage.test.ts` passes whenever the
 * trees agree — including for a helper that never reports anything. These cases
 * pin the reporting itself, so the gate cannot quietly stop biting.
 */
describe("collectOrphanKeys", () => {
  it("says nothing when the locale matches English", () => {
    const en = { menu: { save: "Save" } };

    expect(collectOrphanKeys(en, { menu: { save: "Speichern" } })).toEqual([]);
  });

  it("reports a leaf English does not have, under a table it does", () => {
    const en = { menu: { save: "Save" } };
    const locale = { menu: { save: "Speichern", fossil: "Fossil" } };

    expect(collectOrphanKeys(en, locale)).toEqual(["menu.fossil"]);
  });

  it("reports every leaf beneath a table English does not have", () => {
    const locale = { gone: { a: "x", deeper: { b: "y" } } };

    expect(collectOrphanKeys({}, locale)).toEqual(["gone.a", "gone.deeper.b"]);
  });

  it("reports a candidate-only table that has no leaves to name", () => {
    expect(collectOrphanKeys({}, { gone: {} })).toEqual(["gone"]);
    expect(collectOrphanKeys({}, { gone: { alsoEmpty: {} } })).toEqual([
      "gone.alsoEmpty",
    ]);
  });

  it("does not mistake an Object.prototype member for an English key", () => {
    // `reference[key]` finds these on the prototype chain, so a plain lookup
    // would treat both as present in English and let the orphan through.
    expect(collectOrphanKeys({}, { constructor: "x", toString: "y" })).toEqual([
      "constructor",
      "toString",
    ]);
  });

  it("ignores keys English has that the locale is missing", () => {
    // That direction is `collectMissingKeys`; reporting it here would double up.
    const en = { menu: { save: "Save", load: "Load" } };

    expect(collectOrphanKeys(en, { menu: { save: "Speichern" } })).toEqual([]);
  });
});
