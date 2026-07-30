import { describe, expect, it } from "vitest";
import { formatVal } from "../../lib/helpers";
import { formatSignedAmount, getFacilityUpgradeCost } from "./FinancesTab.helpers";

describe("formatSignedAmount", () => {
  it("renders a negative amount with a leading minus", () => {
    expect(formatSignedAmount(-5_000_000).startsWith("-")).toBe(true);
    expect(formatSignedAmount(5_000_000).startsWith("-")).toBe(false);
  });

  it("defers sign placement to formatVal", () => {
    // Pins the reason the abs()-then-prepend-"-" version was dropped: formatVal
    // already renders the sign, so the two agree for every input.
    for (const value of [-5_000_000, -1_250, 0, 1_250, 5_000_000]) {
      expect(formatSignedAmount(value)).toBe(formatVal(value));
    }
  });
});

describe("getFacilityUpgradeCost", () => {
  it("scales linearly with the current level", () => {
    expect(getFacilityUpgradeCost(1)).toBe(250_000);
    expect(getFacilityUpgradeCost(4)).toBe(1_000_000);
  });
});
