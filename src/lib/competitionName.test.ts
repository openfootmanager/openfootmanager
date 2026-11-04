import { describe, expect, it } from "vitest";

import { competitionDisplayName, type NamedCompetition } from "./competitionName";

const messages: Record<string, string> = {
  "nations.arg": "Argentina",
  "tournaments.competitions.tierDivision": "{{country}} Division {{tier}}",
};

function translate(key: string, options: Record<string, unknown> = {}): string {
  const template = messages[key] ?? String(options.defaultValue ?? key);
  return template.replace(/\{\{(\w+)\}\}/g, (_match, name: string) => String(options[name] ?? ""));
}

describe("competitionDisplayName", () => {
  it("renders distinct one-based division tiers from the competition calendar", () => {
    const competition: NamedCompetition = {
      name_key: "tournaments.competitions.tierDivision",
      country_id: "ARG",
      calendar: { season: { division: { tier: 3 } } },
    };

    expect(competitionDisplayName(competition, translate)).toBe("Argentina Division 3");
    expect(
      competitionDisplayName(
        {
          ...competition,
          calendar: {
            season: { division: { tier: 4 } },
          },
        },
        translate,
      ),
    ).toBe("Argentina Division 4");
  });

  it("uses the division tier passed through a compact frontend DTO", () => {
    expect(
      competitionDisplayName(
        {
          name_key: "tournaments.competitions.tierDivision",
          country_id: "ARG",
          division_tier: 3,
        },
        translate,
      ),
    ).toBe("Argentina Division 3");
  });
});
