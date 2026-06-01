import { fireEvent, render, screen } from "@testing-library/react";
import { describe, expect, it, vi } from "vitest";

import type { FixtureData, GameStateData, TeamData } from "../../store/gameStore";
import ScheduleTab from "./ScheduleTab";

vi.mock("../../lib/seasonContext", () => ({
  resolveSeasonContext: () => ({
    phase: "RegularSeason",
    season_start: "2026-08-01",
  }),
}));

vi.mock("react-i18next", () => ({
  useTranslation: () => ({
    t: (key: string, params?: Record<string, string | number>) => {
      if (key === "schedule.noLeague") return "No league";
      if (key === "schedule.fixtures") return "Fixtures";
      if (key === "schedule.standings") return "Standings";
      if (key === "common.team") return "Team";
      if (key === "common.viewTeam") return "View team";
      if (key === "common.played") return "P";
      if (key === "common.won") return "W";
      if (key === "common.drawn") return "D";
      if (key === "common.lost") return "L";
      if (key === "common.gf") return "GF";
      if (key === "common.ga") return "GA";
      if (key === "common.gd") return "GD";
      if (key === "common.pts") return "Pts";
      if (key === "schedule.season") return `Season ${params?.number}`;
      if (key === "schedule.matchday") return `Matchday ${params?.number}`;
      return key;
    },
    i18n: {
      language: "en",
    },
  }),
}));

function createTeam(overrides: Partial<TeamData> = {}): TeamData {
  return {
    id: "team-1",
    name: "Alpha FC",
    short_name: "ALP",
    country: "GB",
    city: "London",
    stadium_name: "Alpha Ground",
    stadium_capacity: 30000,
    finance: 500000,
    manager_id: "manager-1",
    reputation: 50,
    wage_budget: 50000,
    transfer_budget: 250000,
    season_income: 0,
    season_expenses: 0,
    formation: "4-4-2",
    play_style: "Balanced",
    training_focus: "General",
    training_intensity: "Balanced",
    training_schedule: "Balanced",
    founded_year: 1900,
    colors: { primary: "#000000", secondary: "#ffffff" },
    starting_xi_ids: [],
    form: [],
    history: [],
    ...overrides,
  };
}

function createFixture(overrides: Partial<FixtureData> = {}): FixtureData {
  return {
    id: "fixture-1",
    matchday: 1,
    date: "2026-08-01",
    home_team_id: "team-1",
    away_team_id: "team-2",
    competition: "League",
    status: "Completed",
    result: {
      home_goals: 2,
      away_goals: 1,
      home_scorers: [],
      away_scorers: [],
    },
    ...overrides,
  };
}

function createGameState(withLeague: boolean): GameStateData {
  return {
    clock: {
      current_date: "2026-08-10T00:00:00Z",
      start_date: "2026-07-01T00:00:00Z",
    },
    manager: {
      id: "manager-1",
      first_name: "Jane",
      last_name: "Doe",
      date_of_birth: "1980-01-01",
      nationality: "GB",
      reputation: 50,
      satisfaction: 50,
      fan_approval: 50,
      team_id: "team-1",
      career_stats: {
        matches_managed: 0,
        wins: 0,
        draws: 0,
        losses: 0,
        trophies: 0,
        best_finish: null,
      },
      career_history: [],
    },
    teams: [
      createTeam(),
      createTeam({ id: "team-2", name: "Beta FC", short_name: "BET" }),
    ],
    players: [],
    staff: [],
    messages: [],
    news: [],
    league: withLeague
      ? {
        id: "league-1",
        name: "Premier League",
        season: 1,
        fixtures: [createFixture()],
        standings: [
          {
            team_id: "team-1",
            played: 1,
            won: 1,
            drawn: 0,
            lost: 0,
            goals_for: 2,
            goals_against: 1,
            points: 3,
          },
          {
            team_id: "team-2",
            played: 1,
            won: 0,
            drawn: 0,
            lost: 1,
            goals_for: 1,
            goals_against: 2,
            points: 0,
          },
        ],
      }
      : null,
    scouting_assignments: [],
    board_objectives: [],
  };
}

describe("ScheduleTab", () => {
  it("renders the empty state when there is no league", () => {
    render(<ScheduleTab gameState={createGameState(false)} onSelectTeam={vi.fn()} />);

    expect(screen.getByText("No league")).toBeInTheDocument();
  });

  it("switches to standings and lets the user select a team", () => {
    const onSelectTeam = vi.fn();

    render(<ScheduleTab gameState={createGameState(true)} onSelectTeam={onSelectTeam} />);

    fireEvent.click(screen.getByRole("button", { name: /Standings/i }));
    fireEvent.click(screen.getByText("Beta FC"));

    expect(onSelectTeam).toHaveBeenCalledWith("team-2");
  });

  it("offers context menu actions for fixture teams", () => {
    const onSelectTeam = vi.fn();

    render(<ScheduleTab gameState={createGameState(true)} onSelectTeam={onSelectTeam} />);

    fireEvent.contextMenu(screen.getByTestId("schedule-fixture-fixture-1"));
    fireEvent.click(screen.getByRole("button", { name: "View team: Beta FC" }));

    expect(onSelectTeam).toHaveBeenCalledWith("team-2");
  });
});