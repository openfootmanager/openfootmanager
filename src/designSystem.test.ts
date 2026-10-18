import { describe, expect, it } from "vitest";

/**
 * The design system's rules, checked in every folder that has been brought up to standard.
 *
 * A folder joins FINISHED_FOLDERS in the same commit that makes it pass, so the list only ever
 * names finished work, never a known violation. When every folder is in, the list goes and the
 * checks cover all of `src/`. The rules themselves are in src/CLAUDE.md §2.
 */
const FINISHED_FOLDERS = ["./components/ui/"];

/**
 * SVG artwork whose colours are data, not design: a club's crest or kit, a player's generated
 * portrait. They are the only places a colour literal may appear, and their SVG `fontSize` is in
 * viewBox units, which scale with the drawing, not text sizes.
 */
const ARTWORK_FILES: Record<string, string> = {
  "./components/ui/GeneratedCrest.tsx": "paints a club's colours, or ones derived from its name",
  "./components/ui/GeneratedAvatar.tsx": "paints a player's generated portrait colours",
  "./components/ui/JerseyIcon.tsx": "paints a club's kit, with shading over the kit colours",
};

const SOURCES = import.meta.glob(["./**/*.{ts,tsx}", "!./**/*.test.{ts,tsx}"], {
  query: "?raw",
  import: "default",
  eager: true,
}) as Record<string, string>;

/** Source with comments blanked out, keeping line numbers. URLs inside strings survive. */
function withoutComments(source: string): string {
  return source
    .replace(/\/\*[\s\S]*?\*\//g, (block) => block.replace(/[^\n]/g, " "))
    .replace(/(^|[^:"'`])\/\/.*$/gm, "$1");
}

function linesMatching(source: string, pattern: RegExp): string[] {
  return withoutComments(source)
    .split("\n")
    .flatMap((line, index) => {
      const hits = line.match(pattern);
      return hits ? hits.map((hit) => `${index + 1}: ${hit}`) : [];
    });
}

const SIZE_CLASS = /(?<![\w-])text-(?:xs|sm|base|lg|[2-9]?xl)(?![\w-])/g;
const ARBITRARY_SIZE = /(?<![\w-])text-\[(?:length:)?[\d.]+(?:px|rem|em)\]/g;
const INLINE_FONT_SIZE = /\bfontSize\s*[:=]/g;

/** Rule 1: role sizes only. No Tailwind size names, no arbitrary sizes, no inline font sizes. */
function typeViolations(source: string, isArtwork = false): string[] {
  return [
    ...linesMatching(source, SIZE_CLASS),
    ...linesMatching(source, ARBITRARY_SIZE),
    ...(isArtwork ? [] : linesMatching(source, INLINE_FONT_SIZE)),
  ];
}

const COLOUR_LITERAL = /#[0-9a-fA-F]{6}\b|["'`]#[0-9a-fA-F]{3}["'`]|\b(?:rgba?|hsla?)\(/g;

/** Rule 2: no colour literal in code. Colours come from tokens, or from data in a listed file. */
function colourLiterals(source: string): string[] {
  return linesMatching(source, COLOUR_LITERAL);
}

const PALETTE_CLASS =
  /(?<![\w-])(?:bg|text|border|ring|ring-offset|outline|fill|stroke|from|via|to|divide|placeholder|decoration|shadow|accent|caret)-(?:white|black|gray|slate|zinc|neutral|stone|red|orange|amber|yellow|lime|green|emerald|teal|cyan|sky|blue|indigo|violet|purple|fuchsia|pink|rose)(?![\w])(?:-\d{2,3})?(?:\/\d+)?/g;

/** Rule 3: no raw Tailwind palette colour. Role, tone and brand tokens only. */
function paletteClasses(source: string): string[] {
  return linesMatching(source, PALETTE_CLASS);
}

/** Rule 4: one modal shell. Only ui/Dialog draws a full-screen overlay. */
function overlayViolations(path: string, source: string): string[] {
  if (path === "./components/ui/Dialog.tsx") return [];
  return linesMatching(source, /\bfixed inset-0\b/g);
}

const CLASS_STRING = /(["'`])((?:(?!\1)[^\n])*)\1/g;

/**
 * Rule 5: no text on a -solid fill. Solids are for bars and dots; white on them is under 4.5:1.
 * Checked per class string, since a fill and its text are set together.
 */
function textOnSolid(source: string): string[] {
  return withoutComments(source)
    .split("\n")
    .flatMap((line, index) =>
      [...line.matchAll(CLASS_STRING)]
        .map((match) => match[2])
        .filter(
          (classes) => /\bbg-[\w-]+-solid\b/.test(classes) && /\btext-on-brand\b/.test(classes),
        )
        .map((classes) => `${index + 1}: ${classes}`),
    );
}

function finishedFiles(): Array<[string, string]> {
  return Object.entries(SOURCES).filter(([path]) =>
    FINISHED_FOLDERS.some((folder) => path.startsWith(folder)),
  );
}

function report(check: (path: string, source: string) => string[]): string[] {
  return finishedFiles().flatMap(([path, source]) =>
    check(path, source).map((hit) => `${path}:${hit}`),
  );
}

describe("design system rules in finished folders", () => {
  /**
   * Given a finished folder,
   * when its source is read,
   * then text sizes are role tokens only (text-caption … text-display), so ui_scale moves all of
   * them and nothing is drawn under 12px.
   */
  it("uses role type sizes only", () => {
    expect(report((path, source) => typeViolations(source, path in ARTWORK_FILES))).toEqual([]);
  });

  /**
   * Given a finished folder,
   * when its source is read,
   * then it has no colour literal, except in SVG artwork whose colours are data.
   */
  it("has no colour literals outside artwork files", () => {
    expect(report((path, source) => (path in ARTWORK_FILES ? [] : colourLiterals(source)))).toEqual(
      [],
    );
  });

  /**
   * Given a finished folder,
   * when its source is read,
   * then it uses no raw Tailwind palette colour (gray-500, red-400, white …), only role, tone and
   * brand tokens, which switch with the theme by themselves.
   */
  it("uses no raw palette colours", () => {
    expect(report((_, source) => paletteClasses(source))).toEqual([]);
  });

  /**
   * Given a finished folder,
   * when its source is read,
   * then no file but ui/Dialog draws a full-screen overlay.
   */
  it("draws overlays only through Dialog", () => {
    expect(report(overlayViolations)).toEqual([]);
  });

  /**
   * Given a finished folder,
   * when its source is read,
   * then no class string puts on-brand text on a -solid fill.
   */
  it("puts no text on a solid fill", () => {
    expect(report((_, source) => textOnSolid(source))).toEqual([]);
  });

  /** Given the artwork list, then every file on it exists, so a rename can't leave a stale exemption. */
  it("lists only files that exist as artwork files", () => {
    expect(Object.keys(ARTWORK_FILES).filter((path) => !(path in SOURCES))).toEqual([]);
  });
});

/** Each check is proved against a fixture that breaks it and one that keeps it. */
describe("the rule checks themselves", () => {
  /** Given a size class, an arbitrary px size and an inline font size, then each is reported. */
  it("catches a Tailwind size, an arbitrary size and an inline font size", () => {
    expect(typeViolations('<p className="text-sm text-[10px]" style={{ fontSize: 9 }} />')).toEqual(
      ["1: text-sm", "1: text-[10px]", "1: fontSize:"],
    );
  });

  /** Given an SVG font size in an artwork file, then it is not a text size and is not reported. */
  it("leaves SVG artwork font sizes alone", () => {
    expect(typeViolations('<text fontSize="15">AB</text>', true)).toEqual([]);
  });

  /** Given role sizes and alignment or colour classes that start with text-, then none is reported. */
  it("accepts role sizes and other text- classes", () => {
    expect(
      typeViolations('<p className="text-caption text-left text-fg text-center text-heading" />'),
    ).toEqual([]);
  });

  /** Given white text on a danger-solid fill, then it is reported; on the brand fill it is not. */
  it("catches text on a solid fill and accepts it on the brand fill", () => {
    expect(textOnSolid('<span className="bg-danger-solid text-on-brand" />')).toEqual([
      "1: bg-danger-solid text-on-brand",
    ]);
    expect(textOnSolid('<span className="bg-primary-700 text-on-brand" />')).toEqual([]);
    expect(
      textOnSolid('<span className="bg-danger-solid" /><b className="text-on-brand" />'),
    ).toEqual([]);
  });

  /** Given a hex, an rgb() and a quoted short hex, then each is reported. */
  it("catches colour literals in code", () => {
    expect(
      colourLiterals('const a = "#10b981"; const b = "rgb(1 2 3)"; const c = "#fff";'),
    ).toEqual(["1: #10b981", "1: rgb(", '1: "#fff"']);
  });

  /** Given issue references in comments and a CSS variable, then nothing is reported. */
  it("ignores issue references in comments and CSS variables", () => {
    expect(
      colourLiterals('// fixes #272 and #305\n/* see #282 */\nconst c = "var(--color-panel)";'),
    ).toEqual([]);
  });

  /** Given raw palette classes, with and without variants and opacity, then each is reported. */
  it("catches raw palette classes", () => {
    expect(
      paletteClasses('className="bg-white dark:text-gray-400 hover:border-red-500 bg-black/50"'),
    ).toEqual(["1: bg-white", "1: text-gray-400", "1: border-red-500", "1: bg-black/50"]);
  });

  /** Given role, tone and brand tokens, then nothing is reported. */
  it("accepts role, tone and brand tokens", () => {
    expect(
      paletteClasses(
        'className="bg-panel text-fg-muted border-line bg-tone-good-tint bg-primary-700 text-on-brand"',
      ),
    ).toEqual([]);
  });

  /** Given a full-screen overlay outside Dialog, then it is reported; inside Dialog it is not. */
  it("catches an overlay outside Dialog only", () => {
    const overlay = '<div className="fixed inset-0 bg-overlay" />';
    expect(overlayViolations("./components/tactics/TacticsTab.tsx", overlay)).toEqual([
      "1: fixed inset-0",
    ]);
    expect(overlayViolations("./components/ui/Dialog.tsx", overlay)).toEqual([]);
  });
});
