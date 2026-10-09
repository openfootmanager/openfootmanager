import { describe, expect, it } from "vitest";
import { isRatedMatch } from "./matchRating";

describe("isRatedMatch", () => {
  /**
   * Given the stored zero that stands for "no rating computed"
   * When it is checked
   * Then it is not a rating
   */
  it("treats zero as unrated", () => {
    expect(isRatedMatch(0)).toBe(false);
  });

  /**
   * Given a stored value that cannot be a rating
   * When it is checked
   * Then it is unrated
   */
  it("treats negative and non-finite values as unrated", () => {
    expect(isRatedMatch(-1)).toBe(false);
    expect(isRatedMatch(Number.NaN)).toBe(false);
    expect(isRatedMatch(Number.POSITIVE_INFINITY)).toBe(false);
  });

  /**
   * Given a real rating
   * When it is checked
   * Then it is rated
   */
  it("treats a positive rating as rated", () => {
    expect(isRatedMatch(6.8)).toBe(true);
  });
});
