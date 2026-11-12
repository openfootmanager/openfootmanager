import { describe, expect, it } from "vitest";

import { countries, countryName, englishCountryTable, normaliseNationality } from "./countries";

/**
 * The English country-name table the MCP server reads (#1388).
 *
 * `src-tauri/src/mcp_server/backend_text` turns a `country` or `nationality` param into the
 * country's name the way the screen does (`countryName`, after `normaliseNationality`), and it has
 * no country library of its own. `countryNames.en.json` is that library's English side, written by
 * `englishCountryTable()` from the very tables the screen reads. The Rust golden test
 * (`src/utils/backendTextGolden.json`) holds the two resolvers to the same text for the cases it
 * lists; this file holds the table itself to the functions it stands for.
 *
 * To change it (after a country or a football identity is added, or the library is updated),
 * regenerate and commit the file:
 *
 *   rm src/lib/countryNames.en.json && npm exec --no -- vitest run src/lib/countryNames.test.ts
 *
 * (A run with the file missing writes it; with `CI` set, vitest never writes a snapshot, so a
 * missing or different file fails the job.)
 */

const table = englishCountryTable();

describe("the English country-name table", () => {
  /**
   * Given the tables the screen reads
   * When the table is built from them
   * Then `countryNames.en.json` is that table, exactly
   */
  it("is what englishCountryTable writes", async () => {
    await expect(`${JSON.stringify(table, null, 2)}\n`).toMatchFileSnapshot(
      "./countryNames.en.json",
    );
  });

  /**
   * Given every code the table lists
   * When the screen names it in English
   * Then the table has that name
   */
  it.each(Object.entries(table.codes))("names %s as the screen does", (code, name) => {
    expect(countryName(code, "en")).toBe(name);
  });

  /**
   * Given every ISO country the library knows and every football identity
   * When the table is read
   * Then each is in it: the table cannot be missing a country the screen names
   */
  it("lists every ISO country and football identity the screen knows", () => {
    const iso = Object.keys(countries.getAlpha2Codes());

    expect(iso.filter((code) => !(code in table.codes))).toEqual([]);
    for (const identity of ["ENG", "SCO", "WAL", "NIR", "IE"]) {
      expect(table.codes[identity]).toBe(countryName(identity, "en"));
    }
  });

  /**
   * Given every alias, in lower case and written in capitals and mixed case
   * When the screen normalises it
   * Then it is the code the table lists, whatever the case
   */
  it.each(Object.entries(table.aliases))("reads the alias %s as %s in any case", (alias, code) => {
    for (const written of [alias, alias.toUpperCase(), alias[0].toUpperCase() + alias.slice(1)]) {
      expect(normaliseNationality(written)).toBe(code);
    }
  });

  /**
   * Given every demonym as the old saves wrote it
   * When the screen normalises it
   * Then it is the code the table lists
   */
  it.each(Object.entries(table.demonyms))(
    "reads the demonym %s as %s, exactly as written",
    (demonym, code) => {
      expect(normaliseNationality(demonym)).toBe(code);
    },
  );

  /**
   * Given a demonym in lower case
   * When the screen normalises it
   * Then it is left as written: demonyms are read exactly. (Not every one: `argentine` and `swiss`
   *      come out as AR and CH, because a country name the library lists in another language spells
   *      them that way. The backend writes English names and the old saves' demonyms as they were,
   *      so the table carries no other language's names.)
   */
  it.each(["brazilian", "swedish"])("does not read the lower-case demonym %s", (lower) => {
    expect(normaliseNationality(lower)).toBe(lower);
    expect(lower in table.demonyms).toBe(false);
  });

  /**
   * Given every English country name the library lists
   * When the screen normalises it, in any case
   * Then it is the code the table lists
   */
  it.each(Object.entries(table.names))("reads the name %s as %s in any case", (name, code) => {
    expect(normaliseNationality(name)).toBe(code);
    expect(normaliseNationality(name.toUpperCase())).toBe(code);
  });
});
