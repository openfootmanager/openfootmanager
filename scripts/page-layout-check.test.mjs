// @vitest-environment node
import { readFileSync } from "node:fs";
import { describe, expect, it } from "vitest";
import { HARNESS_PAGES } from "../src/dev/pageHarness/harnessPage";
import { CASES, judge, loadFaces, nodeProblem, REQUIRED_FACES } from "./page-layout-check.mjs";

describe("the Node check", () => {
  /** Given a Node without the built-in WebSocket (before 22), then the check says why it can't run. */
  it("names the Node version it needs", () => {
    expect(nodeProblem(undefined, "v18.19.0")).toBe(
      "page-layout-check needs Node 22 or later for the built-in WebSocket; you have v18.19.0",
    );
  });

  /** Given Node 22 or later, then there is nothing to say. */
  it("accepts a Node with the built-in WebSocket", () => {
    expect(nodeProblem(class {}, "v22.22.2")).toBeNull();
  });
});

const clean = {
  scrollWidth: 960,
  innerWidth: 960,
  unanswered: [],
  pastEdge: [],
  rendered: true,
  faces: [{ family: "Inter", weight: 400, loadedWeights: ["400"] }],
};

describe("the fonts it measures with", () => {
  /**
   * Given a page where a bundled face didn't load (the browser drew that text in a system font)
   * When the case is judged
   * Then it fails, naming the face: a width measured in a fallback font isn't the app's width, and
   * two machines with different fallbacks would disagree.
   */
  it("fails a case measured without a bundled face", () => {
    expect(
      judge({
        ...clean,
        faces: [
          { family: "Inter", weight: 400, loadedWeights: ["400"] },
          { family: "Barlow Condensed", weight: 700, loadedWeights: [] },
        ],
      }),
    ).toEqual(["font not loaded: Barlow Condensed 700 (a system fallback drew it)"]);
  });

  /**
   * Given a family whose requested weight is missing, but another of its weights loaded in its
   * place (the browser picks the nearest weight it has)
   * When the case is judged
   * Then it fails too: a 600 drawn in place of a 700 has other widths.
   */
  it("fails a case where another weight stood in for the one asked for", () => {
    expect(
      judge({
        ...clean,
        faces: [{ family: "Barlow Condensed", weight: 700, loadedWeights: ["600"] }],
      }),
    ).toEqual(["font not loaded: Barlow Condensed 700 (a system fallback drew it)"]);
  });

  /**
   * Given a font request that errors (the dev server refuses the font file, as Vite does for a
   * node_modules outside its allowed folders)
   * When the faces are loaded
   * Then each is reported as not loaded, so the case fails naming it, rather than the error ending
   * the run with nothing said.
   */
  it("reports a face whose request errors as not loaded", async () => {
    const refusing = { load: () => Promise.reject(new Error("403")) };
    const faces = await loadFaces(refusing, REQUIRED_FACES.slice(0, 2));
    expect(faces).toEqual([
      { ...REQUIRED_FACES[0], loadedWeights: [] },
      { ...REQUIRED_FACES[1], loadedWeights: [] },
    ]);
    expect(judge({ ...clean, faces })).toEqual([
      `font not loaded: Inter 400 (a system fallback drew it)`,
      `font not loaded: Inter 500 (a system fallback drew it)`,
    ]);
  });

  /** Given faces that load, then each reports the weights it loaded at. */
  it("reports the weights each face loaded at", async () => {
    const serving = { load: (font) => Promise.resolve([{ weight: font.split(" ")[0] }]) };
    expect(await loadFaces(serving, [{ family: "Inter", weight: 600 }])).toEqual([
      { family: "Inter", weight: 600, loadedWeights: ["600"] },
    ]);
  });

  /**
   * Given the faces App.css bundles (its @fontsource imports)
   * When the check lists the faces it requires
   * Then they are the same families at the same weights, so a face added to the app is checked too.
   */
  it("requires exactly the faces the app bundles", () => {
    const css = readFileSync(new URL("../src/App.css", import.meta.url), "utf-8");
    // The Latin faces are @fontsource imports (no script: latin and latin-ext load by the default
    // sample); the Cyrillic ones are @font-face blocks, named by the file they load.
    const imported = [
      ...css.matchAll(/@import "@fontsource\/([a-z-]+)\/[a-z-]+-(\d+)\.css";/g),
    ].map(([, slug, weight]) => `${slug} ${weight}`);
    const declared = [...css.matchAll(/@font-face\s*\{[^}]*\}/g)].map(([block]) => {
      const [, slug, script, weight] =
        block.match(/files\/([a-z-]+?)-(cyrillic(?:-ext)?)-(\d+)-normal\.woff2/) ?? [];
      return `${slug.replace(/-cyrillic$/, "")} ${weight} ${script}`;
    });
    const bundled = new Set([...imported, ...declared]);
    const required = new Set(
      REQUIRED_FACES.map(
        ({ family, weight, script }) =>
          `${family.toLowerCase().replaceAll(" ", "-")} ${weight}${script ? ` ${script}` : ""}`,
      ),
    );
    expect(bundled.size).toBeGreaterThan(0);
    expect(required).toEqual(bundled);
  });

  /**
   * Given a face scoped by unicode-range (the Cyrillic ones)
   * When the faces are loaded
   * Then it is asked for with a sample in its range, because a face only loads for text it covers,
   * and a Latin sample would report it unloaded.
   */
  it("loads a scoped face with a sample in its script", async () => {
    const asked = [];
    const recording = {
      load: (font, text) => {
        asked.push([font, text]);
        return Promise.resolve([{ weight: font.split(" ")[0] }]);
      },
    };
    await loadFaces(recording, [
      { family: "Inter", weight: 400 },
      { family: "Inter", weight: 400, script: "cyrillic", sample: "Яё" },
    ]);
    expect(asked).toEqual([
      ['400 16px "Inter"', "Aa ÄÖÜß"],
      ['400 16px "Inter"', "Яё"],
    ]);
  });

  /**
   * Given a Cyrillic face whose range does not cover the sample, so that only an unscoped Inter face
   * (one claiming every character) comes back from the load
   * When the faces are loaded
   * Then the Cyrillic face is reported as not loaded: the unscoped face's weight is not proof of it.
   */
  it("does not take an unscoped face of the same family for the scoped one", async () => {
    const fallbackOnly = {
      load: (font) => Promise.resolve([{ weight: font.split(" ")[0], unicodeRange: "U+0-10FFFF" }]),
    };
    const scoped = { family: "Inter", weight: 400, script: "cyrillic", sample: "Яё" };
    expect(await loadFaces(fallbackOnly, [scoped])).toEqual([{ ...scoped, loadedWeights: [] }]);

    const covering = {
      load: (font) =>
        Promise.resolve([
          {
            weight: font.split(" ")[0],
            unicodeRange: "U+301,U+400-45F,U+490-491,U+4B0-4B1,U+2116",
          },
        ]),
    };
    expect(await loadFaces(covering, [scoped])).toEqual([{ ...scoped, loadedWeights: ["400"] }]);
  });

  /** Given a Cyrillic face that did not load, then the case names its script. */
  it("names the script of a scoped face that did not load", () => {
    expect(
      judge({
        ...clean,
        faces: [
          {
            family: "Fira Sans Extra Condensed",
            weight: 700,
            script: "cyrillic",
            loadedWeights: [],
          },
        ],
      }),
    ).toEqual([
      "font not loaded: Fira Sans Extra Condensed 700 cyrillic (a system fallback drew it)",
    ]);
  });
});

describe("page layout check", () => {
  /** Given a page that fits, answered every command and rendered, then it passes with no problems. */
  it("passes a page that fits", () => {
    expect(judge(clean)).toEqual([]);
  });

  /** Given a page wider than the window, then it fails: the page scrolls sideways. */
  it("fails a page that scrolls sideways", () => {
    expect(judge({ ...clean, scrollWidth: 1003 })).toEqual([
      "the page scrolls sideways (1003 > 960)",
    ]);
  });

  /** Given elements past the right edge, then each is named. */
  it("names what reaches past the edge", () => {
    expect(judge({ ...clean, pastEdge: ["table", "button"] })).toEqual([
      "past the right edge: table, button",
    ]);
  });

  /** Given a command the fixture didn't answer, or a page that never rendered, then it fails. */
  it("fails an unanswered command or a blank page", () => {
    expect(judge({ ...clean, unanswered: ["get_inbox"], rendered: false })).toEqual([
      "unanswered commands: get_inbox",
      "nothing rendered",
    ]);
  });

  /**
   * Given the cases, then they include the reviewer's worst case for #956 (German, extra large,
   * 960×640) on the style guide's long toolbar and on every harness page.
   */
  it("covers German at extra large in the minimum window", () => {
    const worst = CASES.filter(
      (c) => c.lang === "de" && c.scale === "xlarge" && c.width === 960 && c.height === 640,
    );
    expect(worst.map((c) => c.page ?? "style guide").sort()).toEqual(
      [...Object.keys(HARNESS_PAGES), "style guide"].sort(),
    );
  });
});
