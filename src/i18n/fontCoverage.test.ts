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

/** The @font-face block of `family` at `weight` whose file is `file`, or null. */
function faceBlock(css: string, family: string, weight: number, file: string): string | null {
  const blocks = css.match(/@font-face\s*\{[^}]*\}/g) ?? [];
  return (
    blocks.find(
      (block) =>
        block.includes(`font-family: "${family}"`) &&
        block.includes(`font-weight: ${weight};`) &&
        block.includes(file),
    ) ?? null
  );
}

/** The first code point of every range in a block's unicode-range. */
function rangeStarts(block: string): number[] {
  const range = block.match(/unicode-range:\s*([^;]+);/)?.[1] ?? "";
  return [...range.matchAll(/U\+([0-9A-Fa-f]+)/g)].map((match) => Number.parseInt(match[1], 16));
}

/** The block declares a unicode-range, and none of it reaches below the combining accent at U+0301. */
function expectCyrillicOnly(block: string): void {
  const starts = rangeStarts(block);
  expect(starts.length).toBeGreaterThan(0);
  expect(Math.min(...starts)).toBeGreaterThanOrEqual(0x0301);
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
   * Given the Latin scripts the game bundles Inter for
   * When App.css is read
   * Then Inter's face for each is imported at every weight the UI uses
   */
  it.each(["latin", "latin-ext"])("bundles Inter %s at every UI weight", (script) => {
    const css = appCss();

    for (const weight of INTER_WEIGHTS) {
      expect(css).toContain(`@import "@fontsource/inter/${script}-${weight}.css";`);
    }
  });

  /**
   * Given Russian is written in Cyrillic, which the Latin faces do not cover
   * When App.css is read
   * Then Inter's Cyrillic face is declared at every UI weight, limited to Cyrillic code points, so
   *      the text is not drawn in a system fallback
   */
  it("declares Inter cyrillic at every UI weight, scoped to Cyrillic", () => {
    const css = appCss();

    for (const weight of INTER_WEIGHTS) {
      const block = faceBlock(css, "Inter", weight, `inter-cyrillic-${weight}-normal.woff2`);
      expect(block, `Inter cyrillic ${weight}`).not.toBeNull();
      expectCyrillicOnly(block ?? "");
    }
  });

  /**
   * Given Barlow Condensed, the heading face, has no Cyrillic glyphs
   * When a Russian heading is drawn
   * Then the heading stack names a bundled condensed Cyrillic face right after Barlow, and that
   *      face claims Cyrillic code points only, so Latin headings stay Barlow Condensed
   */
  it("gives headings a Cyrillic-only condensed face behind Barlow Condensed", () => {
    const css = appCss();
    const headingStack = css.match(/--font-heading:\s*([^;]+);/)?.[1] ?? "";

    expect(headingStack).toMatch(/^"Barlow Condensed",\s*"Fira Sans Extra Condensed"/);
    for (const weight of [600, 700]) {
      const block = faceBlock(
        css,
        "Fira Sans Extra Condensed",
        weight,
        `fira-sans-extra-condensed-cyrillic-${weight}-normal.woff2`,
      );
      expect(block, `Fira cyrillic ${weight}`).not.toBeNull();
      expectCyrillicOnly(block ?? "");
    }
  });
});
