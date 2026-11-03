import { beforeAll, describe, expect, it } from "vitest";

import i18n, { SUPPORTED_LANGUAGES, i18nReady } from "../i18n";
import { formatOrdinal } from "./ordinal";

async function ordinalIn(language: string, value: number): Promise<string> {
  await i18n.loadLanguages(language);
  const t = i18n.getFixedT(language);
  return formatOrdinal(value, language, t);
}

describe("formatOrdinal", () => {
  beforeAll(async () => {
    await i18nReady;
  });

  /**
   * Given the game in English
   * When ranks that exercise every suffix rule are formatted
   * Then 1, 2, 3 and 4 take st, nd, rd and th, and 11 to 13 and 111 take th
   */
  it("writes English ordinals with the right suffix", async () => {
    const written = await Promise.all(
      [1, 2, 3, 4, 11, 12, 13, 21, 22, 101, 111].map((n) => ordinalIn("en", n)),
    );

    expect(written).toEqual([
      "1st",
      "2nd",
      "3rd",
      "4th",
      "11th",
      "12th",
      "13th",
      "21st",
      "22nd",
      "101st",
      "111th",
    ]);
  });

  /**
   * Given the game in Portuguese
   * When a rank of 2 is formatted
   * Then it reads "2.º", not "2nd"
   */
  it("writes Portuguese ordinals the Portuguese way", async () => {
    expect(await ordinalIn("pt", 2)).toBe("2.º");
    expect(await ordinalIn("pt-BR", 2)).toBe("2º");
  });

  /**
   * Given each language the game ships
   * When a rank of 2 is formatted
   * Then it contains the number and, outside English, none of English's suffixes
   */
  it("follows every shipped language, never English's suffixes", async () => {
    for (const { code } of SUPPORTED_LANGUAGES) {
      const written = await ordinalIn(code, 2);

      expect(written, code).toContain("2");
      if (code !== "en") expect(written, code).not.toMatch(/\d(st|nd|rd|th)$/);
    }
  });
});
