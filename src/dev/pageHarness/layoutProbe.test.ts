import { describe, expect, it } from "vitest";

import { boxesPastEdge } from "./layoutProbe";

const box = (label: string, right: number, extra: Partial<{ width: number; inScroller: boolean }> = {}) => ({
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
