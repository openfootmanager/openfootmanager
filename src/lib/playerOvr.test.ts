import { describe, expect, it } from "vitest";

import { averageRating } from "./playerOvr";

describe("averageRating", () => {
  /**
   * Given squads whose mean rating is a whole number, a fraction above one, and nobody at all
   * When their average is written
   * Then it is the mean rounded down, as the backend's teams card writes it (66.67 is 66, not 67),
   *      and 0 for an empty squad
   */
  it("rounds the mean down, as the backend's average does", () => {
    expect(averageRating([80, 70])).toBe(75);
    expect(averageRating([66, 67, 67])).toBe(66);
    expect(averageRating([])).toBe(0);
  });
});
