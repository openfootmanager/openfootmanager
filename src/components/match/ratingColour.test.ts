import { describe, expect, it } from "vitest";

import { ratingBubbleColour, ratingColour } from "./ratingColour";

describe("ratingBubbleColour", () => {
  /**
   * Given a mark in each performance band, When its pitch bubble is coloured,
   * Then each band gets its own fill, matching the bands the text tones use.
   */
  it.each([
    [8.4, "bg-accent-700"],
    [7.1, "bg-primary-700"],
    [6.0, "bg-gray-600"],
    [5.5, "bg-orange-700"],
    [4.2, "bg-red-700"],
    [null, "bg-gray-500"],
  ])("fills a %s mark with %s", (rating, fill) => {
    expect(ratingBubbleColour(rating)).toBe(fill);
  });

  /**
   * Given marks exactly on each band edge, When coloured, Then the bubble and
   * the text tone agree on which band the mark is in.
   */
  it.each([
    [8, "bg-accent-700", "text-accent-700 dark:text-accent-400"],
    [7, "bg-primary-700", "text-green-700 dark:text-green-400"],
    [6, "bg-gray-600", "text-gray-600 dark:text-gray-300"],
    [5, "bg-orange-700", "text-yellow-700 dark:text-yellow-400"],
  ])("puts a %s mark in the same band for bubble and text", (rating, fill, tone) => {
    expect(ratingBubbleColour(rating)).toBe(fill);
    expect(ratingColour(rating)).toBe(tone);
  });

  /**
   * Given a mark just under a band edge that displays as the edge (5.96 shows as
   * "6.0"), When coloured, Then it takes the band of the mark the player sees.
   */
  it("colours a mark by the value it displays, not the unrounded one", () => {
    expect(ratingBubbleColour(5.96)).toBe("bg-gray-600");
    expect(ratingColour(5.96)).toBe("text-gray-600 dark:text-gray-300");
    expect(ratingBubbleColour(5.94)).toBe("bg-orange-700");
  });
});
