// @vitest-environment jsdom
import { describe, expect, it } from "vitest";
import { collectRegions, judge } from "./page-layout-check.mjs";

/** An element laid out as `scroll` tall inside `visible`: jsdom does no layout, so the heights are set. */
function sized(element, scroll, visible) {
  Object.defineProperty(element, "scrollHeight", { value: scroll, configurable: true });
  Object.defineProperty(element, "clientHeight", { value: visible, configurable: true });
  return element;
}

/** A dashboard page: `<main>`, the workspace (marked) and a roster inside it that may scroll itself. */
function page({ workspace, roster, workspaceStyle = "" }) {
  document.body.innerHTML = `<main><div data-workspace style="${workspaceStyle}"><section aria-label="Roster" style="overflow-y:auto"></section></div></main>`;
  sized(document.querySelector("[data-workspace]"), ...workspace);
  sized(document.querySelector("section"), ...roster);
}

const clean = {
  scrollWidth: 960,
  innerWidth: 960,
  scrollHeight: 640,
  innerHeight: 640,
  unanswered: [],
  pastEdge: [],
  rendered: true,
  faces: [],
};

/** What the page measures for a case that asks for `fitSelector`, judged as the check judges it. */
function judged(fitSelector) {
  const regions = collectRegions(document, getComputedStyle, fitSelector);
  return {
    regions,
    problems: judge({
      ...clean,
      page: "tactics",
      regionsFit: true,
      regionsOverflow: regions.overflow,
      regionsMissing: regions.missing,
    }),
  };
}

describe("collecting the regions that scroll", () => {
  /**
   * Given a page whose workspace holds more than its window shows (1180 of 640) while the document
   *       itself fits
   * When its regions are collected for a case that asks the workspace to fit
   * Then the workspace is named with its scroll height against its visible height, and the case fails.
   */
  it("names a workspace that overflows, whose document does not", () => {
    page({ workspace: [1180, 640], roster: [400, 400] });

    const { regions, problems } = judged("[data-workspace]");

    expect(regions.overflow).toEqual(["div (1180 > 640)"]);
    expect(problems).toEqual(["a region scrolls down inside the page: div (1180 > 640)"]);
  });

  /**
   * Given a workspace that cannot scroll (overflow hidden, as the Tactics tab's is) holding more than
   *       it shows (900 of 640): its pitch is clipped
   * When its regions are collected for a case that asks the workspace to fit
   * Then the clipped content counts as overflow and the case fails, though nothing there scrolls:
   *      a clipped pitch must not pass because it cannot scroll.
   */
  it("counts content a workspace clips as overflow", () => {
    page({ workspace: [900, 640], roster: [400, 400], workspaceStyle: "overflow:hidden" });

    const { regions, problems } = judged("[data-workspace]");

    expect(regions.all).toEqual([]);
    expect(regions.overflow).toEqual(["div (900 > 640)"]);
    expect(problems).toEqual(["a region scrolls down inside the page: div (900 > 640)"]);
  });

  /**
   * Given a fitting workspace holding a roster bounded to a scroll of its own (2000 in a 400 box)
   * When the regions are collected for a case that asks the workspace to fit
   * Then the roster is listed for diagnostics but does not fail the case: a component may scroll,
   *      the workspace may not.
   */
  it("lets a bounded roster scroll inside a workspace that fits", () => {
    page({ workspace: [640, 640], roster: [2000, 400] });

    const { regions, problems } = judged("[data-workspace]");

    expect(regions.all).toEqual(['section[aria-label="Roster"] (2000 > 400)']);
    expect(regions.overflow).toEqual([]);
    expect(problems).toEqual([]);
  });

  /**
   * Given nothing that scrolls
   * When the regions are collected
   * Then none are listed and the case passes.
   */
  it("passes a page where nothing scrolls", () => {
    page({ workspace: [640, 640], roster: [400, 400] });

    const { regions, problems } = judged("[data-workspace]");

    expect(regions.all).toEqual([]);
    expect(problems).toEqual([]);
  });

  /**
   * Given a case that names a region the page does not draw (a mistyped selector, a marker removed)
   * When the regions are collected
   * Then the case fails saying so, rather than passing because there was nothing to measure.
   */
  it("fails a case whose region is not on the page", () => {
    page({ workspace: [640, 640], roster: [400, 400] });

    const { problems } = judged("[data-no-such-region]");

    expect(problems).toEqual(["no visible region on the page matches the case's fit selector"]);
  });

  /**
   * Given a case whose region is on the page but has no height (display: none, so its scroll and
   *       client heights are both 0)
   * When the regions are collected
   * Then the case fails as having no visible region, rather than passing because 0 does not exceed 0.
   */
  it("fails a case whose region is on the page but not shown", () => {
    page({ workspace: [0, 0], roster: [0, 0], workspaceStyle: "display:none" });

    const { problems } = judged("[data-workspace]");

    expect(problems).toEqual(["no visible region on the page matches the case's fit selector"]);
  });

  /**
   * Given scrolling regions that carry an accessible label (the Tactics instructions rail, a nav) or
   *       classes with characters CSS needs escaped (`md:h-screen`, `@container`)
   * When their names are collected
   * Then each name is a selector that identifies the element (`nav[aria-label="..."]`,
   *      `div.md\\:h-screen`), not a label glued onto the tag, so it can be pasted into the page.
   */
  it("names each region by a selector that identifies it", () => {
    document.body.innerHTML = `<main>
      <nav aria-label='Team "instructions"' style="overflow-y:auto"></nav>
      <div class="md:h-screen @container" style="overflow-y:auto"></div>
    </main>`;
    sized(document.querySelector("nav"), 900, 300);
    sized(document.querySelector("div"), 700, 400);

    const { all } = collectRegions(document, getComputedStyle, null);

    expect(all).toEqual([
      'nav[aria-label="Team \\"instructions\\""] (900 > 300)',
      "div.md\\:h-screen.\\@container (700 > 400)",
    ]);
  });
});
