import { describe, expect, it } from "vitest";

import { competitionDisplayName } from "./competitionName";

function translate(key: string, options: Record<string, unknown> = {}): string {
  if (key === "nations.eng") return "England";
  if (key === "nations.ar") return "Argentina";
  if (key.includes("tierDivision")) {
    const phase = key.endsWith("Opening")
      ? " Apertura"
      : key.endsWith("Closing")
        ? " Clausura"
        : "";
    return `${String(options.country)} Division ${String(options.tier)}${phase}`;
  }
  return String(options.defaultValue ?? key);
}

describe("competitionDisplayName", () => {
  /**
   * Given a generic division name key and tier metadata
   * When the competition name is displayed
   * Then the country and arbitrary tier are interpolated
   */
  it("interpolates the country and arbitrary division tier", () => {
    expect(
      competitionDisplayName(
        {
          name: "England Division 3",
          name_key: "tournaments.competitions.tierDivision",
          country_id: "ENG",
          division_tier: 3,
        },
        translate,
      ),
    ).toBe("England Division 3");
  });

  /**
   * Given a split-season generic name key and calendar tier metadata
   * When the competition name is displayed
   * Then the tier and phase are translated from the calendar
   */
  it("uses the tier from a competition calendar for split-season names", () => {
    expect(
      competitionDisplayName(
        {
          name: "Stored legacy competition title",
          name_key: "tournaments.competitions.tierDivisionClosing",
          country_id: "AR",
          calendar: { season: { division: { tier: 3 } } },
        },
        translate,
      ),
    ).toBe("Argentina Division 3 Clausura");
  });

  /**
   * Given legacy generic-name metadata without a tier
   * When the competition name is displayed
   * Then the stored name remains the fallback
   */
  it("falls back to the stored name when a generic key has no tier", () => {
    expect(
      competitionDisplayName(
        {
          name: "A division with legacy metadata",
          name_key: "tournaments.competitions.tierDivision",
          country_id: "ENG",
        },
        translate,
      ),
    ).toBe("A division with legacy metadata");
  });
});
