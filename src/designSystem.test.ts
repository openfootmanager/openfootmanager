import { describe, expect, it } from "vitest";

/**
 * The design system's rules, checked in every folder that has been brought up to standard.
 *
 * A folder joins FINISHED_FOLDERS in the same commit that makes it pass, so the list only ever
 * names finished work, never a known violation. When every folder is in, the list goes and the
 * checks cover all of `src/`. The rules themselves are in src/CLAUDE.md §2.
 */
const FINISHED_FOLDERS = [
  "./components/ui/",
  // The dashboard shell. The rest of components/dashboard (its modals) joins with them, and these
  // four entries become the one folder.
  "./components/dashboard/DashboardAlerts.tsx",
  "./components/dashboard/DashboardHeader.tsx",
  "./components/dashboard/DashboardSidebar.tsx",
  "./components/dashboard/DashboardWorkspaceContent.tsx",
  "./pages/Dashboard.tsx",
];

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

const COLOUR_LITERAL =
  /#[0-9a-fA-F]{6}\b|["'`]#[0-9a-fA-F]{3}["'`]|\[#[0-9a-fA-F]{3,8}\]|\b(?:rgba?|hsla?)\(/g;

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

/** A quoted string on one line, or a template literal, which may run over several. */
const CLASS_STRING = /(["'])((?:(?!\1)[^\n])*)\1|`([^`]*)`/g;

/**
 * Rule 5: no text on a -solid fill. Solids are for bars and dots; white on them is under 4.5:1.
 * Checked per class string, since a fill and its text are set together.
 */
function textOnSolid(source: string): string[] {
  const code = withoutComments(source);
  return [...code.matchAll(CLASS_STRING)]
    .map((match) => ({ classes: match[2] ?? match[3], index: match.index }))
    .filter(
      ({ classes }) => /\bbg-[\w-]+-solid\b/.test(classes) && /\btext-on-brand\b/.test(classes),
    )
    .map(({ classes, index }) => `${code.slice(0, index).split("\n").length}: ${classes}`);
}

/** A focus-state ring class: its variant (group 1) and what follows `ring-` (group 2). */
const FOCUS_RING =
  /(?<![\w-])((?:peer-|group-)?focus(?:-visible|-within)?):ring-([\w-]+(?:\/\d+)?)/g;
/**
 * What may follow `ring-` besides `focus`: a width, inset, an offset width, or an offset in the
 * surface the control sits on, so the gap between control and ring matches what is behind it.
 */
const RING_ALLOWED = /^(?:\d+|inset|offset-(?:\d+|page|panel|sunken|chrome))$/;

/**
 * Rule 6: one focus ring. It is drawn on focus-visible, never plain focus (a click would light it),
 * its colour is ring-focus, the themed one that reaches 3:1 everywhere, and its offset is a surface.
 */
function focusRingViolations(source: string): string[] {
  return linesMatching(source, FOCUS_RING).filter((hit) => {
    const [, variant, rest] = /: ((?:peer-|group-)?focus(?:-visible|-within)?):ring-(.+)$/.exec(
      hit,
    ) ?? ["", "", ""];
    const plainFocus = !variant.endsWith("-visible") && !variant.endsWith("-within");
    return plainFocus || !(rest === "focus" || RING_ALLOWED.test(rest));
  });
}

/**
 * Rule 7 runs over these folders: one home per vocabulary. A folder joins when the round-4 PR that
 * redraws it moves its positions, tiers and condition bands onto the ui primitives and the backend's
 * tones (round-4 PR 1b adds components/squad). It never covers components/ui, the vocabulary's
 * home, and it grows to all of src/ by round-4 PR 7.
 */
const VOCABULARY_FOLDERS: string[] = [];

/**
 * The words a colour may be keyed by only in its one home: the positions, the rating tiers' letters
 * and the condition bands. A screen that keys a colour by one of them is a second copy of a rule.
 */
const VOCABULARY_KEYS = [
  ...["Goalkeeper", "Defender", "Midfielder", "Forward", "RightBack", "CenterBack", "LeftBack"],
  ...["RightWingBack", "LeftWingBack", "DefensiveMidfielder", "CentralMidfielder"],
  ...["AttackingMidfielder", "RightMidfielder", "LeftMidfielder", "RightWinger", "LeftWinger"],
  ...["Striker", "S", "A", "B", "Fresh", "Tiring", "Exhausted", "Critical"],
];
const VOCABULARY_COLOUR_MAP = new RegExp(
  // A plain, quoted or computed key (`[Position.Goalkeeper]`), then a string holding a colour class.
  `(?<![\\w-])(?:\\[[\\w.]*?\\.)?["']?(?:${VOCABULARY_KEYS.join("|")})["']?\\]?\\s*:\\s*["'\`][^"'\`\\n]*(?<![\\w-])(?:bg|text|border|ring)-[\\w-]+`,
  "g",
);

/**
 * A rated value judged against a number on a screen, in either order, by order or equality. Known
 * to over-match a has-a-value guard (`potential > 0`, as in InboxMessageDetailPane): when that
 * folder joins, the guard is rewritten as a presence check rather than exempted.
 */
const RATED_THRESHOLD =
  /\b(?:condition|ovr|potential|morale|reputation)\s*(?:===|!==|==|!=|>=|<=|>|<)\s*\d+|\b\d+\s*(?:===|!==|==|!=|>=|<=|>|<)\s*[\w.?]*\b(?:condition|ovr|potential|morale|reputation)\b/g;

/**
 * Helpers whose primitive has landed: importing one is a way around it. Each round-4 PR adds the
 * helper it retires and deletes it (knip then catches a stray copy). A dynamic `import("…")` (the
 * dashboard lazy-loads its tabs that way) isn't matched; deleting the helper makes one fail to
 * build anyway.
 */
const RETIRED_HELPERS: string[] = [];

/**
 * Rule 7: one home per vocabulary. No colour map keyed by a position, a tier letter or a condition
 * band; no rated value compared with a number in a component (the backend picks the tone); no
 * import of a retired helper.
 */
function vocabularyViolations(path: string, source: string, retired = RETIRED_HELPERS): string[] {
  const retiredImport =
    retired.length === 0
      ? []
      : // The module specifier, wherever it sits: an import spread over several lines ends on its
        // `} from "…"` line, and a re-export names the module the same way.
        linesMatching(
          source,
          new RegExp(
            `\\b(?:from|import)\\s*["'][^"'\\n]*\\b(?:${retired.join("|")})\\b[^"'\\n]*["']`,
            "g",
          ),
        );
  return [
    ...linesMatching(source, VOCABULARY_COLOUR_MAP),
    ...(path.endsWith(".tsx") ? linesMatching(source, RATED_THRESHOLD) : []),
    ...retiredImport,
  ];
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

  /**
   * Given a finished folder,
   * when its source is read,
   * then every focus ring is ring-focus, shown on focus-visible, so it is the themed colour that
   * reaches 3:1 in every theme.
   */
  it("draws every focus ring in the focus colour", () => {
    expect(report((_, source) => focusRingViolations(source))).toEqual([]);
  });

  /**
   * Given a folder on rule 7's list,
   * when its source is read,
   * then it keys no colour by a position, tier or band, judges no rated value against a number,
   * and imports no retired helper: each vocabulary has one home.
   */
  it("keeps each vocabulary in its one home", () => {
    const hits = Object.entries(SOURCES)
      .filter(([path]) => VOCABULARY_FOLDERS.some((folder) => path.startsWith(folder)))
      .flatMap(([path, source]) =>
        vocabularyViolations(path, source).map((hit) => `${path}:${hit}`),
      );
    expect(hits).toEqual([]);
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

  /** Given a class string that spans lines in a template literal, then text on a solid is reported. */
  it("catches text on a solid fill across the lines of a template literal", () => {
    expect(
      textOnSolid("const a = 1;\n<span className={`bg-danger-solid\n  text-on-brand`} />"),
    ).toEqual(["2: bg-danger-solid\n  text-on-brand"]);
  });

  /**
   * Given a focus ring in a brand colour, one on plain focus and one on a peer, then each is
   * reported; given the focus colour on focus-visible, then nothing is.
   */
  it("catches a focus ring that is not ring-focus on focus-visible", () => {
    expect(
      focusRingViolations(
        'className="focus-visible:ring-primary-500 focus:ring-2 dark:focus:ring-offset-navy-800 peer-focus:ring-focus"',
      ),
    ).toEqual([
      "1: focus-visible:ring-primary-500",
      "1: focus:ring-2",
      "1: focus:ring-offset-navy-800",
      "1: peer-focus:ring-focus",
    ]);
    expect(
      focusRingViolations(
        'className="focus:outline-none focus-visible:ring-2 focus-visible:ring-focus focus-visible:ring-offset-2 focus-visible:ring-offset-page peer-focus-visible:ring-inset"',
      ),
    ).toEqual([]);
  });

  /**
   * Given a focus ring offset in a brand colour on focus-visible, then it is reported; given an
   * offset in the surface the control sits on (page, panel, sunken, chrome), then it is not.
   */
  it("catches a focus ring offset that is not a surface colour", () => {
    expect(focusRingViolations('className="focus-visible:ring-offset-navy-800"')).toEqual([
      "1: focus-visible:ring-offset-navy-800",
    ]);
    expect(
      focusRingViolations(
        'className="focus-visible:ring-offset-sunken focus-visible:ring-offset-panel focus-visible:ring-offset-chrome focus-visible:ring-offset-1"',
      ),
    ).toEqual([]);
  });

  /** Given a hex, an rgb() and a quoted short hex, then each is reported. */
  it("catches colour literals in code", () => {
    expect(
      colourLiterals('const a = "#10b981"; const b = "rgb(1 2 3)"; const c = "#fff";'),
    ).toEqual(["1: #10b981", "1: rgb(", '1: "#fff"']);
  });

  /** Given short and alpha hex values inside arbitrary classes, then each is reported. */
  it("catches hex values in arbitrary classes", () => {
    expect(colourLiterals('<p className="bg-[#fff] text-[#1234]" />')).toEqual([
      "1: [#fff]",
      "1: [#1234]",
    ]);
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

  /**
   * Given a screen with its own colour per position, per tier letter or per condition band
   * When rule 7 reads it
   * Then each such entry is reported: those colours have one home, the ui primitive.
   */
  it("catches a colour map keyed by a position, a tier or a band", () => {
    const source = [
      "const POSITION_COLOR = {",
      '  Goalkeeper: "bg-amber-500",',
      "  CenterBack: 'text-tone-good',",
      "};",
      'const TIER = { S: "bg-tone-top-tint text-tone-top", A: "bg-tone-good-tint" };',
      'const BAND = { "Tiring": "text-tone-fair" };',
    ].join("\n");
    expect(vocabularyViolations("./components/squad/X.tsx", source)).toEqual([
      '2: Goalkeeper: "bg-amber-500',
      "3: CenterBack: 'text-tone-good",
      '5: S: "bg-tone-top-tint text-tone-top',
      '5: A: "bg-tone-good-tint',
      '6: "Tiring": "text-tone-fair',
    ]);
  });

  /** Given a colour map keyed by a computed key (an enum member), then it is reported too. */
  it("catches a colour map with computed keys", () => {
    const source =
      'const COLOUR = { [Position.Goalkeeper]: "bg-tone-top-tint", [Band.Critical]: "text-tone-bad" };';
    expect(vocabularyViolations("./components/squad/X.tsx", source)).toEqual([
      '1: [Position.Goalkeeper]: "bg-tone-top-tint',
      '1: [Band.Critical]: "text-tone-bad',
    ]);
  });

  /** Given a rated value tested for equality with a number, then it is reported like any comparison. */
  it("catches a rated value compared for equality with a number", () => {
    const source = [
      "if (player.condition === 70) tone = good;",
      "const capped = 99 !== player.ovr;",
    ].join("\n");
    expect(vocabularyViolations("./components/squad/X.tsx", source)).toEqual([
      "1: condition === 70",
      "2: 99 !== player.ovr",
    ]);
  });

  /** Given a map keyed by those words whose values aren't colours, then it is left alone. */
  it("leaves a vocabulary map that holds no colour alone", () => {
    const source = 'const GROUPS = { Goalkeeper: "Goalkeeper", CenterBack: "Defender", S: 90 };';
    expect(vocabularyViolations("./components/squad/X.tsx", source)).toEqual([]);
  });

  /**
   * Given a component comparing a rated value with a number, in either order
   * When rule 7 reads it
   * Then it is reported (the backend picks a rated value's tone); in a .ts helper it isn't this
   * rule's business, since a sort or a filter may compare numbers.
   */
  it("catches a rated value compared with a number in a component", () => {
    const source = [
      "const tone = player.condition >= 70 ? good : bad;",
      "if (75 <= player?.ovr) return;",
      "const old = player.age > 30;",
    ].join("\n");
    expect(vocabularyViolations("./components/squad/X.tsx", source)).toEqual([
      "1: condition >= 70",
      "2: 75 <= player?.ovr",
    ]);
    expect(vocabularyViolations("./lib/sort.ts", source)).toEqual([]);
  });

  /** Given a retired helper, then importing it is reported. */
  it("catches an import of a retired helper", () => {
    const source = 'import { getPositionColor } from "../../lib/positionColors";';
    expect(vocabularyViolations("./components/squad/X.tsx", source, ["positionColors"])).toEqual([
      '1: from "../../lib/positionColors"',
    ]);
    expect(vocabularyViolations("./components/squad/X.tsx", source, [])).toEqual([]);
  });

  /** Given a retired helper imported over several lines, or re-exported, then it is reported. */
  it("catches a retired helper in a multi-line import and a re-export", () => {
    const source = [
      "import {",
      "  getPositionColor,",
      '} from "../../lib/positionColors";',
      'export { POSITION_COLOR } from "../lib/positionColors";',
    ].join("\n");
    expect(vocabularyViolations("./components/squad/X.tsx", source, ["positionColors"])).toEqual([
      '3: from "../../lib/positionColors"',
      '4: from "../lib/positionColors"',
    ]);
  });
});
