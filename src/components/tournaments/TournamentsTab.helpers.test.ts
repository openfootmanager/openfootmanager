import { describe, expect, it } from "vitest";
import type { TFunction } from "i18next";

import type { FixtureData, LeagueData } from "../../store/gameStore";
import type { PlayerNameEntry } from "../../services/competitionsService";
import {
  buildTopScorers,
  byTablePosition,
  isKnockoutCompetition,
  localizedRoundName,
  summarizeCompetitionProgress,
} from "./TournamentsTab.helpers";

function fixture(overrides: Partial<FixtureData> = {}): FixtureData {
  return {
    id: "fixture-1",
    matchday: 1,
    date: "2026-08-10",
    home_team_id: "team-1",
    away_team_id: "team-2",
    competition: "League",
    status: "Scheduled",
    result: null,
    ...overrides,
  } as FixtureData;
}

function played(
  matchday: number,
  homeScorers: string[],
  awayScorers: string[],
  overrides: Partial<FixtureData> = {},
): FixtureData {
  return fixture({
    id: `fixture-${matchday}-${homeScorers.join("")}${awayScorers.join("")}`,
    matchday,
    status: "Completed",
    result: {
      home_goals: homeScorers.length,
      away_goals: awayScorers.length,
      home_scorers: homeScorers.map((player_id, minute) => ({
        player_id,
        minute,
      })),
      away_scorers: awayScorers.map((player_id, minute) => ({
        player_id,
        minute,
      })),
    },
    ...overrides,
  });
}

function name(overrides: Partial<PlayerNameEntry> = {}): PlayerNameEntry {
  return {
    match_name: "A. Player",
    full_name: "Ada Player",
    team_id: "team-1",
    team_name: "Alpha FC",
    ...overrides,
  } as PlayerNameEntry;
}

function standing(points: number, goalsFor: number, goalsAgainst: number) {
  return { points, goals_for: goalsFor, goals_against: goalsAgainst };
}

describe("isKnockoutCompetition", () => {
  it("treats a plain league table as not a knockout", () => {
    expect(
      isKnockoutCompetition({ rules: { format: "LeagueTable" } } as LeagueData),
    ).toBe(false);
  });

  it("treats any other declared format as a knockout", () => {
    expect(
      isKnockoutCompetition({ rules: { format: "Knockout" } } as LeagueData),
    ).toBe(true);
  });

  it("falls back to the presence of knockout rounds when there are no rules", () => {
    expect(isKnockoutCompetition({} as LeagueData)).toBe(false);
    expect(
      isKnockoutCompetition({ knockout_rounds: [] } as unknown as LeagueData),
    ).toBe(false);
    expect(
      isKnockoutCompetition({
        knockout_rounds: [{ name: "Final" }],
      } as unknown as LeagueData),
    ).toBe(true);
  });
});

describe("byTablePosition", () => {
  it("ranks on points first", () => {
    expect(byTablePosition(standing(10, 0, 0), standing(9, 99, 0))).toBeLessThan(
      0,
    );
  });

  it("breaks equal points on goal difference", () => {
    // +7 beats +5 even though the second side scored far more goals.
    expect(
      byTablePosition(standing(10, 8, 1), standing(10, 20, 15)),
    ).toBeLessThan(0);
  });

  it("breaks equal points and goal difference on goals scored", () => {
    // Both +5; the side that scored 20 goes above the side that scored 6.
    expect(
      byTablePosition(standing(10, 20, 15), standing(10, 6, 1)),
    ).toBeLessThan(0);
    expect(
      byTablePosition(standing(10, 6, 1), standing(10, 20, 15)),
    ).toBeGreaterThan(0);
  });

  it("sorts a table best-first", () => {
    const table = [
      standing(3, 1, 1),
      standing(9, 10, 2),
      standing(3, 5, 5),
      standing(6, 4, 4),
    ];

    expect([...table].sort(byTablePosition).map((row) => row.points)).toEqual([
      9, 6, 3, 3,
    ]);
  });
});

describe("localizedRoundName", () => {
  const t = ((key: string, options?: { size?: string }) =>
    options?.size ? `${key}:${options.size}` : key) as unknown as TFunction;

  it("maps the named rounds to translation keys", () => {
    expect(localizedRoundName(t, "Final")).toBe("tournaments.rounds.final");
    expect(localizedRoundName(t, "Semifinal")).toBe(
      "tournaments.rounds.semifinal",
    );
    expect(localizedRoundName(t, "Quarterfinal")).toBe(
      "tournaments.rounds.quarterfinal",
    );
  });

  it("passes the bracket size through for 'Round of N'", () => {
    expect(localizedRoundName(t, "Round of 16")).toBe(
      "tournaments.rounds.roundOf:16",
    );
  });

  it("returns anything it does not recognise unchanged", () => {
    expect(localizedRoundName(t, "Group Stage")).toBe("Group Stage");
    expect(localizedRoundName(t, "Round of sixteen")).toBe("Round of sixteen");
  });
});

describe("summarizeCompetitionProgress", () => {
  it("reports an empty competition as not complete", () => {
    const progress = summarizeCompetitionProgress([]);

    expect(progress.totalMatchdays).toBe(0);
    expect(progress.seasonComplete).toBe(false);
    expect(progress.totalGoals).toBe(0);
    expect(progress.completedMatches).toBe(0);
  });

  it("groups fixtures by matchday in playing order", () => {
    const progress = summarizeCompetitionProgress([
      fixture({ id: "c", matchday: 3 }),
      fixture({ id: "a", matchday: 1 }),
      fixture({ id: "b", matchday: 2 }),
      fixture({ id: "a2", matchday: 1 }),
    ]);

    expect(progress.sortedMatchdays.map(([md]) => md)).toEqual([1, 2, 3]);
    expect(progress.sortedMatchdays[0][1]).toHaveLength(2);
  });

  it("counts a matchday complete only when every fixture in it is", () => {
    const progress = summarizeCompetitionProgress([
      played(1, ["p1"], []),
      played(1, [], []),
      played(2, ["p1"], []),
      fixture({ id: "pending", matchday: 2 }),
    ]);

    expect(progress.completedMatchdays).toBe(1);
    expect(progress.totalMatchdays).toBe(2);
    expect(progress.seasonComplete).toBe(false);
  });

  it("marks the season complete once every matchday has played out", () => {
    const progress = summarizeCompetitionProgress([
      played(1, ["p1"], ["p2"]),
      played(2, [], ["p2"]),
    ]);

    expect(progress.seasonComplete).toBe(true);
    expect(progress.completedMatches).toBe(2);
    expect(progress.totalGoals).toBe(3);
  });

  it("counts goals from results only, ignoring unplayed fixtures", () => {
    const progress = summarizeCompetitionProgress([
      played(1, ["p1", "p1"], ["p2"]),
      fixture({ id: "later", matchday: 2 }),
    ]);

    expect(progress.totalGoals).toBe(3);
    expect(progress.completedMatches).toBe(1);
  });
});

describe("buildTopScorers", () => {
  const names = {
    p1: name({ match_name: "One" }),
    p2: name({ match_name: "Two" }),
    p3: name({ match_name: "Three" }),
  };

  it("tallies goals from both sides and sorts best first", () => {
    const scorers = buildTopScorers(
      [played(1, ["p1", "p2"], ["p1"]), played(2, ["p3"], ["p1"])],
      names,
    );

    expect(scorers.map((entry) => [entry.playerId, entry.goals])).toEqual([
      ["p1", 3],
      ["p2", 1],
      ["p3", 1],
    ]);
  });

  it("drops scorers with no known name rather than showing a blank row", () => {
    const scorers = buildTopScorers(
      [played(1, ["p1", "unknown", "unknown"], [])],
      names,
    );

    expect(scorers).toHaveLength(1);
    expect(scorers[0].playerId).toBe("p1");
  });

  it("ignores fixtures that have not been played", () => {
    expect(buildTopScorers([fixture()], names)).toEqual([]);
  });

  it("caps the table at the requested limit", () => {
    const many = Object.fromEntries(
      Array.from({ length: 12 }, (_, i) => [`q${i}`, name()]),
    );
    const goals = played(
      1,
      Array.from({ length: 12 }, (_, i) => `q${i}`),
      [],
    );

    expect(buildTopScorers([goals], many)).toHaveLength(10);
    expect(buildTopScorers([goals], many, 3)).toHaveLength(3);
  });
});
