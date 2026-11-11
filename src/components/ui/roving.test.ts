import { describe, expect, it } from "vitest";

import { rovingTarget } from "./roving";

describe("rovingTarget", () => {
  /** Given four items and the second focused, then the arrows move one step and wrap. */
  it("moves one step with the arrows and wraps at both ends", () => {
    expect(rovingTarget("ArrowRight", 1, 4)).toBe(2);
    expect(rovingTarget("ArrowLeft", 1, 4)).toBe(0);
    expect(rovingTarget("ArrowRight", 3, 4)).toBe(0);
    expect(rovingTarget("ArrowLeft", 0, 4)).toBe(3);
  });

  /** Given Home or End, then the first or the last item. */
  it("jumps to the ends with Home and End", () => {
    expect(rovingTarget("Home", 2, 4)).toBe(0);
    expect(rovingTarget("End", 1, 4)).toBe(3);
  });

  /**
   * Given a radio group, which the up and down arrows also move (WAI-ARIA radio group), then they
   * do; a tab list, which only uses left and right, leaves them alone.
   */
  it("moves with up and down only when asked to", () => {
    expect(rovingTarget("ArrowDown", 1, 4, { upDown: true })).toBe(2);
    expect(rovingTarget("ArrowUp", 1, 4, { upDown: true })).toBe(0);
    expect(rovingTarget("ArrowDown", 1, 4)).toBeNull();
  });

  /**
   * Given a row that must not wrap (the match speed, where wrapping from Slow lands on Max and runs
   * the match to half time), then the arrows stop at both ends.
   */
  it("stops at both ends when wrapping is off", () => {
    expect(rovingTarget("ArrowRight", 3, 4, { wrap: false })).toBe(3);
    expect(rovingTarget("ArrowLeft", 0, 4, { wrap: false })).toBe(0);
    expect(rovingTarget("ArrowRight", 1, 4, { wrap: false })).toBe(2);
  });

  /** Given any other key, then nothing moves. */
  it("ignores other keys", () => {
    expect(rovingTarget("Enter", 1, 4)).toBeNull();
  });
});
