import { describe, expect, it } from "vitest";

import { contractRiskLevelOf } from "./contractUtils";

describe("contractRiskLevelOf", () => {
  /**
   * Given each of the backend's contract warning stages
   * When it is read as a badge
   * Then the last six months and nearer are critical, the year before that a warning, and no stage
   *      stable.
   */
  it("reads the last six months as critical, the year before as a warning, and none as stable", () => {
    expect(contractRiskLevelOf("final_weeks")).toBe("critical");
    expect(contractRiskLevelOf("three_months")).toBe("critical");
    expect(contractRiskLevelOf("six_months")).toBe("critical");
    expect(contractRiskLevelOf("twelve_months")).toBe("warning");
    expect(contractRiskLevelOf(null)).toBe("stable");
    expect(contractRiskLevelOf(undefined)).toBe("stable");
  });
});
