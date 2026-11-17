import { render, screen } from "@testing-library/react";
import { describe, expect, it, vi } from "vitest";

import type { FixtureData, LeagueData } from "../../store/gameStore";
import CompetitionsOverview from "./CompetitionsOverview";
import { getCompetitionStatus } from "./CompetitionsOverview";

vi.mock("react-i18next", () => ({
  initReactI18next: { type: "3rdParty", init: () => undefined },
  useTranslation: () => ({
    t: (key: string, options?: Record<string, string | number>) => {
      if (key === "nations.ar") return "Argentina";
      if (key === "tournaments.competitions.tierDivision") {
        return `${options?.country} Division ${options?.tier}`;
      }
      if (key === "tournaments.competitions.phaseOpening") return "Opening tournament";
      if (key === "tournaments.competitions.phaseClosing") return "Closing tournament";
      if (key === "schedule.season") return `Season ${options?.number}`;
      if (key === "tournaments.competitions.statusNotStarted") return "Not started";
      return key;
    },
  }),
}));

function fixture(
  status: "Scheduled" | "Completed",
  competition: FixtureData["competition"] = "League",
  competitive = true,
): FixtureData {
  return {
    competitive,
    advancing_team_id: null,
    id: `f-${Math.random()}`,
    matchday: 1,
    date: "2026-08-01",
    home_team_id: "t1",
    away_team_id: "t2",
    competition,
    status,
    result:
      status === "Completed"
        ? { home_goals: 1, away_goals: 0, home_scorers: [], away_scorers: [] }
        : null,
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
        fixtures: [fixture("Completed", "Friendly", false)],
        id: "c1",
        name: "X",
        season: 1,
        standings: [],
      } as never),
    ).toBe("notStarted");
  });

  it("ignores FriendlyCup fixtures when determining status", () => {
    // A friendly cup is not competitive either (the backend says so on the fixture)
    expect(
      getCompetitionStatus({
        fixtures: [fixture("Completed", "FriendlyCup", false), fixture("Scheduled")],
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
        fixtures: [fixture("Completed", "PreseasonTournament", false), fixture("Scheduled")],
        id: "c1",
        name: "X",
        season: 1,
        standings: [],
      } as never),
    ).toBe("notStarted");
  });
});

describe("CompetitionsOverview", () => {
  /**
   * Given opening and closing competitions in the same tier
   * When their rows render
   * Then each localized label keeps its season phase visible
   */
  it("shows the phase in split-season competition names", () => {
    const competition = (id: string, phase: "opening" | "closing"): LeagueData => ({
      id,
      name: `Argentina Division 3 ${phase}`,
      name_key: "tournaments.competitions.tierDivision",
      country_id: "AR",
      kind: "League",
      scope: "Domestic",
      season: 2032,
      participant_ids: ["team-1"],
      fixtures: [],
      standings: [],
      calendar: {
        definition_id: `ar-d3-${phase}`,
        edition_basis: { kind: "calendarYear" },
        league_legs: 2,
        matchday_gap_days: null,
        season: { division: { familyId: "ar-league", tier: 3, phase } },
      },
    });

    render(
      <CompetitionsOverview
        competitions={[
          competition("ar-apertura", "opening"),
          competition("ar-clausura", "closing"),
        ]}
        userTeamId="team-1"
        onSelect={vi.fn()}
      />,
    );

    expect(
      screen.getByRole("button", { name: /Argentina Division 3 Opening tournament/ }),
    ).toBeInTheDocument();
    expect(
      screen.getByRole("button", { name: /Argentina Division 3 Closing tournament/ }),
    ).toBeInTheDocument();
  });
});
