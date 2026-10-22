import { readFileSync } from "node:fs";

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
  return readFileSync("src/App.css", "utf-8");
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
  it.each([...new Set(Object.values(SCRIPT_BY_LOCALE))].filter((script) => script !== "cjk"))(
    "bundles Inter %s at every UI weight",
    (script) => {
      const css = appCss();

      for (const weight of INTER_WEIGHTS) {
        expect(css).toContain(`@import "@fontsource/inter/${script}-${weight}.css";`);
      }
    },
  );

  /**
   * Given Barlow Condensed, the heading face, has no Cyrillic glyphs
   * When a Russian heading is drawn
   * Then the heading stack names a bundled condensed Cyrillic face right after Barlow, so it does
   *      not fall back to the much wider Inter and overflow the sidebar and tab bars
   */
  it("gives headings a bundled condensed Cyrillic face behind Barlow Condensed", () => {
    const css = appCss();
    const headingStack = css.match(/--font-heading:\s*([^;]+);/)?.[1] ?? "";

    expect(headingStack).toMatch(/^"Barlow Condensed",\s*"Fira Sans Extra Condensed"/);
    for (const weight of [600, 700]) {
      expect(css).toContain(
        `@import "@fontsource/fira-sans-extra-condensed/cyrillic-${weight}.css";`,
      );
    }
  });
});
