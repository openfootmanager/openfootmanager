import { describe, expect, it } from "vitest";

import { boxesPastEdge, isSidewaysScroller } from "./layoutProbe";

describe("isSidewaysScroller", () => {
  /** Given a wide table's wrapper (scrolls across, not down), then it is a sideways scroller. */
  it("counts a box that scrolls across but not down", () => {
    expect(isSidewaysScroller({ overflowX: "auto", scrollHeight: 300, clientHeight: 300 })).toBe(
      true,
    );
  });

  /**
   * Given the workspace, which scrolls down (overflow-y: auto makes overflow-x compute to auto
   * too), then it is not one, so overflow inside it is still reported.
   */
  it("does not count a vertical scroller", () => {
    expect(isSidewaysScroller({ overflowX: "auto", scrollHeight: 2400, clientHeight: 600 })).toBe(
      false,
    );
  });

  /** Given a box that doesn't clip, then it is not a scroller. */
  it("does not count a box that doesn't scroll", () => {
    expect(isSidewaysScroller({ overflowX: "visible", scrollHeight: 300, clientHeight: 300 })).toBe(
      false,
    );
  });
});

const box = (
  label: string,
  right: number,
  extra: Partial<{ width: number; inScroller: boolean }> = {},
) => ({
  label,
  right,
  width: 100,
  inScroller: false,
  ...extra,
});

describe("boxesPastEdge", () => {
  /** Given a 656px window, then a box ending at 700 is reported and one ending at 656 is not. */
  it("reports a box that reaches past the window's right edge", () => {
    expect(boxesPastEdge([box("table", 700), box("toolbar", 656)], 656)).toEqual(["table"]);
  });

  /** Given a box past the edge by a sub-pixel rounding, then it is not reported. */
  it("tolerates a sub-pixel overhang", () => {
    expect(boxesPastEdge([box("tab", 656.6)], 656)).toEqual([]);
  });

  /** Given an empty box past the edge, then it is not reported: nothing is drawn there. */
  it("ignores an empty box", () => {
    expect(boxesPastEdge([box("spacer", 900, { width: 0 })], 656)).toEqual([]);
  });

  /** Given a box past the edge inside a container that scrolls sideways on purpose, then it is not reported. */
  it("ignores content inside a sideways scroller", () => {
    expect(boxesPastEdge([box("row", 900, { inScroller: true })], 656)).toEqual([]);
  });
});
