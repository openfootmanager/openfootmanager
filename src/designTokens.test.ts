import { readFileSync } from "node:fs";
import { describe, expect, it } from "vitest";

// Paths are relative to the Vitest root (the repository root); `new URL(..., import.meta.url)`
// throws under jsdom.
const appCss = readFileSync("src/App.css", "utf-8");

/**
 * The custom properties declared in the one block whose line reads exactly `selector {`. A selector
 * that opens no block, or more than one, fails loudly instead of reading the wrong block.
 */
function declarationsIn(selector: string): Map<string, string> {
  const escaped = selector.replace(/[.*+?^${}()|[\]\\]/g, "\\$&");
  const openings = [...appCss.matchAll(new RegExp(`^[ \\t]*${escaped} \\{$`, "gm"))];
  if (openings.length !== 1) {
    throw new Error(`Expected one "${selector} {" block in App.css, found ${openings.length}`);
  }
  const start = openings[0].index ?? 0;
  const body = appCss.slice(appCss.indexOf("{", start) + 1, appCss.indexOf("}", start));
  const declarations = new Map<string, string>();
  for (const match of body.matchAll(/(--[\w-]+)\s*:\s*([^;]+);/g)) {
    declarations.set(match[1], match[2].trim());
  }
  return declarations;
}

const theme = declarationsIn("@theme");
const dark = declarationsIn(".dark");
const highContrast = declarationsIn(".high-contrast");
const highContrastDark = declarationsIn(".high-contrast.dark");
const compact = declarationsIn(".density-compact");

type Rgba = [number, number, number, number];

function parseColour(value: string): Rgba {
  const hex = value.match(/^#([0-9a-f]{6})$/i);
  if (hex) {
    const n = Number.parseInt(hex[1], 16);
    return [(n >> 16) & 255, (n >> 8) & 255, n & 255, 1];
  }
  const rgb = value.match(/^rgb\(\s*(\d+)\s+(\d+)\s+(\d+)\s*(?:\/\s*([\d.]+))?\s*\)$/);
  if (rgb) return [Number(rgb[1]), Number(rgb[2]), Number(rgb[3]), rgb[4] ? Number(rgb[4]) : 1];
  throw new Error(`Not a colour this test understands: ${value}`);
}

/** A translucent colour as it actually appears over an opaque one. */
function over(top: Rgba, bottom: Rgba): Rgba {
  const alpha = top[3];
  return [0, 1, 2].map((i) => top[i] * alpha + bottom[i] * (1 - alpha)).concat(1) as Rgba;
}

function luminance([r, g, b]: Rgba): number {
  const channel = (c: number) => {
    const s = c / 255;
    return s <= 0.03928 ? s / 12.92 : ((s + 0.055) / 1.055) ** 2.4;
  };
  return 0.2126 * channel(r) + 0.7152 * channel(g) + 0.0722 * channel(b);
}

function contrast(a: Rgba, b: Rgba): number {
  const [high, low] = [luminance(a), luminance(b)].sort((x, y) => y - x);
  return (high + 0.05) / (low + 0.05);
}

type Mode = "light" | "dark" | "high-contrast light" | "high-contrast dark";

/**
 * The blocks that set a token in each mode, winner first. App.tsx toggles .high-contrast in either
 * theme. In dark it competes with .dark at the same specificity and comes later in the file, so it
 * would win, but .high-contrast.dark is more specific and wins over both.
 */
const LAYERS: Record<Mode, Array<Map<string, string>>> = {
  light: [theme],
  dark: [dark, theme],
  "high-contrast light": [highContrast, theme],
  "high-contrast dark": [highContrastDark, highContrast, dark, theme],
};

function tokenIn(mode: Mode, name: string): string {
  const layers = LAYERS[mode];
  for (const layer of layers) {
    const value = layer.get(name);
    if (value) return value;
  }
  throw new Error(`${name} is not defined for ${mode}`);
}

/** The rendered colour of `name` in `mode`, with translucency resolved against the panel. */
function rendered(mode: Mode, name: string): Rgba {
  const panel = parseColour(tokenIn(mode, "--color-panel"));
  return over(parseColour(tokenIn(mode, name)), panel);
}

/** Colours that change with the theme. Chrome, brand, kit and pitch colours are the same in both. */
const THEMED_COLOURS = [
  "page",
  "panel",
  "sunken",
  "fg",
  "fg-muted",
  "fg-subtle",
  "line",
  "line-strong",
  "hover",
  "focus",
  "series-1",
  "series-2",
  "selected",
  "selected-tint",
  "selected-fill",
  ...["positive", "warning", "danger", "info"].flatMap((s) => [s, `${s}-tint`]),
  ...["top", "good", "fair", "low", "bad"].flatMap((t) => [`tone-${t}`, `tone-${t}-tint`]),
].map((name) => `--color-${name}`);

/** Text colour, then every surface it is drawn on. */
const READABLE_PAIRS: Array<[string, string[]]> = [
  ["fg", ["page", "panel", "sunken"]],
  ["fg-muted", ["page", "panel", "sunken"]],
  ["fg-subtle", ["page", "panel", "sunken"]],
  // The chosen option in a list (Select, DatePicker, the comboboxes) and its check mark.
  ["selected", ["panel", "selected-tint"]],
  ...["positive", "warning", "danger", "info"].map((status): [string, string[]] => [
    status,
    ["panel", `${status}-tint`],
  ]),
  ...["top", "good", "fair", "low", "bad"].map((tone): [string, string[]] => [
    `tone-${tone}`,
    ["panel", `tone-${tone}-tint`],
  ]),
];

const MODES = Object.keys(LAYERS) as Mode[];

describe("design tokens", () => {
  /**
   * Given a colour that changes with the theme,
   * when App.css is read,
   * then it has a light value in @theme and a dark value under .dark, so a component using it
   * never needs a `dark:` partner.
   */
  it.each(THEMED_COLOURS)("defines %s for light and for dark", (name) => {
    expect(theme.has(name), `${name} in @theme`).toBe(true);
    expect(dark.has(name), `${name} under .dark`).toBe(true);
  });

  /**
   * Given every text colour and the surfaces it sits on,
   * when each is rendered in light, dark and high-contrast dark (translucent tints composited
   * over the panel first, since a raw rgb(… / 0.45) proves nothing),
   * then each pair reaches WCAG AA 4.5:1.
   */
  it.each(MODES)("keeps every text colour readable in %s", (mode) => {
    const failures: string[] = [];
    for (const [text, surfaces] of READABLE_PAIRS) {
      for (const surface of surfaces) {
        const ratio = contrast(
          rendered(mode, `--color-${text}`),
          rendered(mode, `--color-${surface}`),
        );
        if (ratio < 4.5) failures.push(`${text} on ${surface}: ${ratio.toFixed(2)}`);
      }
    }
    expect(failures).toEqual([]);
  });

  /**
   * Given the focus ring,
   * when it is drawn against the page or a panel in any mode,
   * then it reaches the 3:1 that WCAG 1.4.11 asks of a non-text indicator.
   */
  it.each(MODES)("keeps the focus ring visible in %s", (mode) => {
    for (const surface of ["--color-page", "--color-panel"]) {
      const ratio = contrast(rendered(mode, "--color-focus"), rendered(mode, surface));
      expect(ratio, `focus on ${surface}`).toBeGreaterThanOrEqual(3);
    }
  });

  /**
   * Given on-brand text and the fills it is allowed on,
   * when white text sits on primary-700 (every primary button) or on chrome (the sidebar, match band),
   * then it reaches 4.5:1. The -solid colours carry no text, so they are not on this list.
   */
  it.each(["--color-primary-700", "--color-chrome"])(
    "keeps on-brand text readable on %s",
    (fill) => {
      const ratio = contrast(
        parseColour(tokenIn("light", "--color-on-brand")),
        parseColour(tokenIn("light", fill)),
      );
      expect(ratio).toBeGreaterThanOrEqual(4.5);
    },
  );

  /**
   * Given a control that is on or chosen (a switch, a checked box, a strong segmented option, a
   * count badge), drawn in selected-fill with on-brand text or a check mark on it,
   * when it is rendered in any mode,
   * then the text reaches 4.5:1 on the fill.
   */
  it.each(MODES)("keeps on-brand text readable on the selected fill in %s", (mode) => {
    const ratio = contrast(
      rendered(mode, "--color-on-brand"),
      rendered(mode, "--color-selected-fill"),
    );
    expect(ratio).toBeGreaterThanOrEqual(4.5);
  });

  /**
   * Given the selected fill marking a control as on (a switch's track, a checked box),
   * when it sits on the page, a panel or a sunken track in any mode,
   * then it reaches the 3:1 that WCAG 1.4.11 asks of a control's state, so "on" reads in dark too.
   */
  it.each(MODES)("sets the selected fill apart from every surface in %s", (mode) => {
    const failures = ["page", "panel", "sunken"]
      .map((surface) => ({
        surface,
        ratio: contrast(
          rendered(mode, "--color-selected-fill"),
          rendered(mode, `--color-${surface}`),
        ),
      }))
      .filter(({ ratio }) => ratio < 3)
      .map(({ surface, ratio }) => `selected-fill on ${surface}: ${ratio.toFixed(2)}`);
    expect(failures).toEqual([]);
  });

  /**
   * Given the six role-named type sizes,
   * when they are read,
   * then each is in rem (so ui_scale moves it) and none is below 0.75rem (12px at normal scale).
   */
  it("sets every role size in rem, none under 0.75rem", () => {
    const roles = ["caption", "body", "lead", "heading", "title", "display"];
    for (const role of roles) {
      const value = theme.get(`--text-${role}`);
      expect(value, `--text-${role}`).toMatch(/^[\d.]+rem$/);
      expect(Number.parseFloat(value ?? "0"), `--text-${role}`).toBeGreaterThanOrEqual(0.75);
    }
  });

  /**
   * Given the compact density,
   * when .density-compact applies,
   * then rows, panel headers and the toolbar shrink, and a row stays at least 2rem tall.
   */
  it("shrinks rows, panel headers and the toolbar under compact density", () => {
    for (const name of ["--spacing-row", "--spacing-panel-header", "--spacing-toolbar"]) {
      const comfortable = Number.parseFloat(theme.get(name) ?? "0");
      const dense = Number.parseFloat(compact.get(name) ?? "0");
      expect(dense, name).toBeGreaterThan(0);
      expect(dense, name).toBeLessThan(comfortable);
    }
    expect(Number.parseFloat(compact.get("--spacing-row") ?? "0")).toBeGreaterThanOrEqual(2);
  });

  /**
   * Given the one page width,
   * when it is read,
   * then it is 80rem, the value the maintainer settled (see fix/restore-page-max-width).
   */
  it("keeps the page width at 80rem", () => {
    expect(theme.get("--container-page")).toBe("80rem");
  });
});
