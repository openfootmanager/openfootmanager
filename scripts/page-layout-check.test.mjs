// @vitest-environment node
import { describe, expect, it } from "vitest";
import { HARNESS_PAGES } from "../src/dev/pageHarness/harnessPage";
import { CASES, judge, nodeProblem } from "./page-layout-check.mjs";

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

const clean = { scrollWidth: 960, innerWidth: 960, unanswered: [], pastEdge: [], rendered: true };

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
