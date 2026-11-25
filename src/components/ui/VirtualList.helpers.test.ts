import { describe, expect, it } from "vitest";

import { renderedIndexes, scrollTopToReveal, windowRange } from "./VirtualList.helpers";

describe("windowRange", () => {
  const base = { count: 10_000, step: 64, viewportHeight: 640, overscan: 5 };

  /**
   * Given a long list scrolled to its top
   * When the window is taken
   * Then it holds the rows on screen plus the overscan below, and nothing above the first row
   */
  it("starts at the first row at the top of the list", () => {
    expect(windowRange({ ...base, scrollTop: 0, listTop: 0 })).toEqual({ start: 0, end: 15 });
  });

  /**
   * Given a list scrolled to the middle
   * When the window is taken
   * Then it is the visible rows with the overscan on both sides, however long the list is
   */
  it("covers the visible rows and the overscan on both sides", () => {
    // Row 5,000 is at the top of the viewport.
    const { start, end } = windowRange({ ...base, scrollTop: 5_000 * 64, listTop: 0 });

    expect(start).toBe(4_995);
    expect(end).toBe(5_015);
  });

  /**
   * Given a list that begins below other content in its scroller (a toolbar above it)
   * When the scroller has not yet reached the list
   * Then the window is still the top of the list: the list's own offset is taken off
   */
  it("measures from where the list starts, not from the top of the scroller", () => {
    // The viewport shows the list from its first row down to 540 px into it.
    expect(windowRange({ ...base, scrollTop: 100, listTop: 200 })).toEqual({ start: 0, end: 14 });
  });

  /**
   * Given a list scrolled past its last row
   * When the window is taken
   * Then it is clamped to the list
   */
  it("never reaches past the last row", () => {
    const { start, end } = windowRange({ ...base, count: 20, scrollTop: 10_000_000, listTop: 0 });

    expect(end).toBe(20);
    expect(start).toBeLessThanOrEqual(20);
  });

  /**
   * Given an empty list
   * When the window is taken
   * Then it is empty
   */
  it("is empty for no rows", () => {
    expect(windowRange({ ...base, count: 0, scrollTop: 0, listTop: 0 })).toEqual({
      start: 0,
      end: 0,
    });
  });

  /**
   * Given a list shorter than the viewport
   * When the window is taken
   * Then every row is in it
   */
  it("holds a short list whole", () => {
    expect(windowRange({ ...base, count: 4, scrollTop: 0, listTop: 0 })).toEqual({
      start: 0,
      end: 4,
    });
  });
});

describe("scrollTopToReveal", () => {
  const args = { step: 64, rowHeight: 56, listTop: 100, viewportHeight: 400 };

  /**
   * Given a row already fully on screen
   * When it is to be revealed
   * Then the scroller is left where it is
   */
  it("does nothing for a row that is already visible", () => {
    // Row 2 spans 228..284 in scroller coordinates; the viewport is 100..500.
    expect(scrollTopToReveal({ ...args, index: 2, scrollTop: 100 })).toBeNull();
  });

  /**
   * Given a row above the viewport
   * When it is to be revealed
   * Then the scroller moves up so the row's top is at the viewport's top
   */
  it("scrolls up to a row above the viewport", () => {
    expect(scrollTopToReveal({ ...args, index: 0, scrollTop: 1_000 })).toBe(100);
  });

  /**
   * Given a row below the viewport
   * When it is to be revealed
   * Then the scroller moves down just far enough that the row's bottom meets the viewport's
   */
  it("scrolls down to a row below the viewport", () => {
    // Row 50 spans 3300..3356; the viewport must end at 3356.
    expect(scrollTopToReveal({ ...args, index: 50, scrollTop: 0 })).toBe(3_356 - 400);
  });
});

describe("renderedIndexes", () => {
  /**
   * Given a window of rows 10..14 and no focused row
   * When the indexes to render are listed
   * Then they are exactly the window
   */
  it("is the window when nothing outside it has focus", () => {
    expect(renderedIndexes({ start: 10, end: 14 }, 100, -1)).toEqual([10, 11, 12, 13]);
  });

  /**
   * Given a focused row above the window
   * When the indexes are listed
   * Then it comes first, before the window, so document order matches list order
   */
  it("keeps a focused row above the window ahead of it", () => {
    expect(renderedIndexes({ start: 10, end: 12 }, 100, 3)).toEqual([3, 10, 11]);
  });

  /**
   * Given a focused row below the window
   * When the indexes are listed
   * Then it comes last
   */
  it("keeps a focused row below the window after it", () => {
    expect(renderedIndexes({ start: 10, end: 12 }, 100, 50)).toEqual([10, 11, 50]);
  });

  /**
   * Given a focused row already inside the window, or one past the end of a shrunk list
   * When the indexes are listed
   * Then it is not added twice, and a row that no longer exists is not added at all
   */
  it("does not duplicate a focused row in the window or invent one past the end", () => {
    expect(renderedIndexes({ start: 10, end: 12 }, 100, 11)).toEqual([10, 11]);
    expect(renderedIndexes({ start: 0, end: 3 }, 5, 9)).toEqual([0, 1, 2]);
  });
});
