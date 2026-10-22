// @vitest-environment node
import { readFileSync } from "node:fs";
import { describe, expect, it } from "vitest";
import { CASES, judge, nodeProblem, REQUIRED_FACES } from "./page-layout-check.mjs";

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
   * Given the faces App.css bundles (its @fontsource imports)
   * When the check lists the faces it requires
   * Then they are the same families at the same weights, so a face added to the app is checked too.
   */
  it("requires exactly the faces the app bundles", () => {
    const css = readFileSync(new URL("../src/App.css", import.meta.url), "utf-8");
    const bundled = new Set(
      [...css.matchAll(/@import "@fontsource\/([a-z-]+)\/[a-z-]+-(\d+)\.css";/g)].map(
        ([, slug, weight]) => `${slug} ${weight}`,
      ),
    );
    const required = new Set(
      REQUIRED_FACES.map(
        ({ family, weight }) => `${family.toLowerCase().replaceAll(" ", "-")} ${weight}`,
      ),
    );
    expect(bundled.size).toBeGreaterThan(0);
    expect(required).toEqual(bundled);
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
    expect(worst.map((c) => c.page ?? "style guide").sort()).toEqual([
      "home",
      "squad",
      "style guide",
    ]);
  });
});
