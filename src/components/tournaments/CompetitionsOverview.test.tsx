import { describe, expect, it } from "vitest";

import type { FixtureData } from "../../store/gameStore";
import { getCompetitionStatus } from "./CompetitionsOverview";

function fixture(
  status: "Scheduled" | "Completed",
  competition: FixtureData["competition"] = "League",
): FixtureData {
  return {
    id: `f-${Math.random()}`,
    matchday: 1,
    date: "2026-08-01",
    home_team_id: "t1",
    away_team_id: "t2",
    competition,
    status,
    result: status === "Completed" ? { home_goals: 1, away_goals: 0, home_scorers: [], away_scorers: [] } : null,
  };
}

describe("getCompetitionStatus", () => {
  it("returns notStarted when there are no competitive fixtures", () => {
    expect(
      getCompetitionStatus({
        fixtures: [],
        id: "c1",
        name: "X",
        season: 1,
        standings: [],
      } as never),
    ).toBe("notStarted");
  });

  it("returns notStarted when all competitive fixtures are scheduled", () => {
    expect(
      getCompetitionStatus({
        fixtures: [fixture("Scheduled"), fixture("Scheduled")],
        id: "c1",
        name: "X",
        season: 1,
        standings: [],
      } as never),
    ).toBe("notStarted");
  });

  it("returns inProgress when some but not all competitive fixtures are completed", () => {
    expect(
      getCompetitionStatus({
        fixtures: [fixture("Completed"), fixture("Scheduled")],
        id: "c1",
        name: "X",
        season: 1,
        standings: [],
      } as never),
    ).toBe("inProgress");
  });

  it("returns completed when all competitive fixtures are completed", () => {
    expect(
      getCompetitionStatus({
        fixtures: [fixture("Completed"), fixture("Completed")],
        id: "c1",
        name: "X",
        season: 1,
        standings: [],
      } as never),
    ).toBe("completed");
  });

  it("ignores Friendly fixtures when determining status", () => {
    // Only a Friendly is present — no competitive fixtures → notStarted
    expect(
      getCompetitionStatus({
        fixtures: [fixture("Completed", "Friendly")],
        id: "c1",
        name: "X",
        season: 1,
        standings: [],
      } as never),
    ).toBe("notStarted");
  });

  it("ignores PreseasonTournament fixtures when determining status", () => {
    // Preseason completed + one real scheduled → notStarted (preseason not counted)
    expect(
      getCompetitionStatus({
        fixtures: [
          fixture("Completed", "PreseasonTournament"),
          fixture("Scheduled"),
        ],
        id: "c1",
        name: "X",
        season: 1,
        standings: [],
      } as never),
    ).toBe("notStarted");
  });
});
