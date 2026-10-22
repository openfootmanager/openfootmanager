import { readFileSync } from "node:fs";
import { resolve } from "node:path";

import { describe, expect, it } from "vitest";

import { SUPPORTED_LANGUAGES, type SupportedLanguageCode } from "./index";

/**
 * The font subset each locale needs from the bundled Inter. `cjk` is deliberately not bundled:
 * Chinese falls through the stack to the system's CJK face (a 5-10 MB face per weight is not
 * worth shipping), so it is listed here to make that choice explicit.
 */
const SCRIPT_BY_LOCALE: Record<SupportedLanguageCode, "latin" | "latin-ext" | "cyrillic" | "cjk"> =
  {
    en: "latin",
    es: "latin",
    pt: "latin",
    fr: "latin",
    de: "latin",
    it: "latin",
    ru: "cyrillic",
    "pt-BR": "latin",
    "zh-CN": "cjk",
    cs: "latin-ext",
    tr: "latin-ext",
    id: "latin",
  };

const INTER_WEIGHTS = [400, 500, 600, 700];

function appCss(): string {
  return readFileSync(resolve(__dirname, "../App.css"), "utf8");
}

describe("bundled font coverage", () => {
  /**
   * Given every locale the game ships
   * When the font subsets they need are looked up
   * Then each locale names its script, so a 13th locale cannot ship without a decision here
   */
  it("knows the script of every supported locale", () => {
    expect(Object.keys(SCRIPT_BY_LOCALE).sort()).toEqual(
      SUPPORTED_LANGUAGES.map(({ code }) => code).sort(),
    );
  });

  /**
   * Given a locale written in a script other than Latin that the game bundles a face for
   * When App.css is read
   * Then Inter's face for that script is imported at every weight the UI uses, so the text is
   *      not drawn in a system fallback
   */
  it.each(
    [...new Set(Object.values(SCRIPT_BY_LOCALE))].filter((script) => script !== "cjk"),
  )("bundles Inter %s at every UI weight", (script) => {
    const css = appCss();

    for (const weight of INTER_WEIGHTS) {
      expect(css).toContain(`@import "@fontsource/inter/${script}-${weight}.css";`);
    }
  });
});
