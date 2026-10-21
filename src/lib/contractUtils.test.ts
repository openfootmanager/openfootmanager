import { describe, expect, it } from "vitest";

import { contractExpiryLabel, contractRiskLevelOf } from "./contractUtils";

describe("contractExpiryLabel", () => {
  const t = (key: string, options?: { date?: string }) => `${key}: ${options?.date ?? ""}`;

  /**
   * Given a contract that ends on 30 June 2028 and a game played in German
   * When its expiry is labelled
   * Then the date reads as German writes it, not as the stored ISO string
   */
  it("writes the expiry date in the game's language", () => {
    expect(contractExpiryLabel("2028-06-30", "de", t)).toBe(
      "finances.contractExpiresOn: 30. Juni 2028",
    );
  });

  /**
   * Given the same contract in English
   * When its expiry is labelled
   * Then the date reads as English writes it
   */
  it("writes the expiry date in English for an English game", () => {
    expect(contractExpiryLabel("2028-06-30", "en", t)).toBe(
      "finances.contractExpiresOn: June 30, 2028",
    );
  });
});

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
