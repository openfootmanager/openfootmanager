import { describe, expect, it } from "vitest";

import type { RatingTier } from "../services/squadService";
import { isEliteTier, ratingTierClass, ratingTierLetter, type RatingTierRole } from "./ratingTier";

const TIERS: RatingTier[] = ["S", "A", "B", "Rest"];
const ROLES: RatingTierRole[] = ["text", "bar", "badge", "hero"];

describe("ratingTierClass", () => {
  /**
   * Given each tier and each place a colour lands
   * When the class is asked for
   * Then every tier has its own look in every role, so no two tiers read the same.
   */
  it("gives every tier a distinct class in every role", () => {
    for (const role of ROLES) {
      const classes = TIERS.map((tier) => ratingTierClass(tier, role));
      expect(new Set(classes).size, role).toBe(TIERS.length);
    }
  });

  /**
   * Given the top tier and the second
   * When their text classes are asked for
   * Then S is the accent and A the primary, and each carries its dark-theme partner.
   */
  it("makes S the accent and A the primary, each with a dark partner", () => {
    expect(ratingTierClass("S", "text")).toBe("text-accent-500 dark:text-accent-400");
    expect(ratingTierClass("A", "text")).toBe("text-primary-500 dark:text-primary-400");
  });
});

describe("ratingTierLetter", () => {
  /**
   * Given each tier
   * When its letter is asked for
   * Then S, A and B name themselves and the rest has none.
   */
  it("is the tier's own letter, and none for the rest", () => {
    expect(TIERS.map(ratingTierLetter)).toEqual(["S", "A", "B", null]);
  });
});

describe("isEliteTier", () => {
  /**
   * Given each tier
   * When it is asked whether it is elite
   * Then S and A are, and B and the rest are not.
   */
  it("is true for S and A only", () => {
    expect(TIERS.map(isEliteTier)).toEqual([true, true, false, false]);
  });
});
