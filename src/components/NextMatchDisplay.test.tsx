import { render, screen } from "@testing-library/react";
import { describe, expect, it, vi } from "vitest";
import type { GameStateData } from "../store/gameStore";
import NextMatchDisplay from "./NextMatchDisplay";

vi.mock("react-i18next", () => ({
  useTranslation: () => ({
    t: (key: string) => (key === "tournaments.competitions.nationalCup" ? "Copa Nacional" : key),
  }),
}));

function createGameState(): GameStateData {
  return {
    clock: {
      current_date: "2026-07-10T12:00:00Z",
      start_date: "2026-07-01T12:00:00Z",
    },
    manager: {
      id: "manager-1",
      first_name: "Hansi",
      last_name: "Flick",
      date_of_birth: "1965-02-24",
      nationality: "Germany",
      reputation: 80,
      satisfaction: 70,
      fan_approval: 75,
      team_id: "barcelona",
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
      {
        id: "barcelona",
        name: "FC Barcelona",
        short_name: "FCB",
        country: "Spain",
        city: "Barcelona",
        stadium_name: "Camp Nou",
        stadium_capacity: 99354,
        finance: 1000000,
        manager_id: "manager-1",
        reputation: 90,
        wage_budget: 500000,
        transfer_budget: 1000000,
        season_income: 0,
        season_expenses: 0,
        formation: "4-3-3",
        play_style: "Possession",
        training_focus: "General",
        training_intensity: "Balanced",
        training_schedule: "Balanced",
        founded_year: 1899,
        colors: { primary: "#A50044", secondary: "#004D98" },
        starting_xi_ids: [],
        form: [],
        history: [],
      },
      {
        id: "bayern",
        name: "Munich Bayern",
        short_name: "MB",
        country: "Germany",
        city: "Munich",
        stadium_name: "Allianz Arena",
        stadium_capacity: 75000,
        finance: 1000000,
        manager_id: null,
        reputation: 90,
        wage_budget: 500000,
        transfer_budget: 1000000,
        season_income: 0,
        season_expenses: 0,
        formation: "4-2-3-1",
        play_style: "Balanced",
        training_focus: "General",
        training_intensity: "Balanced",
        training_schedule: "Balanced",
        founded_year: 1900,
        colors: { primary: "#DC052D", secondary: "#FFFFFF" },
        starting_xi_ids: [],
        form: [],
        history: [],
      },
    ],
    players: [],
    staff: [],
    messages: [],
    news: [],
    league: {
      id: "league-1",
      name: "Test League",
      season: 2026,
      standings: [],
      fixtures: [
        {
          id: "friendly-1",
          matchday: 0,
          date: "2026-07-17",
          home_team_id: "bayern",
          away_team_id: "barcelona",
          competition: "Friendly",
          status: "Scheduled",
          result: null,
        },
      ],
    },
    scouting_assignments: [],
    board_objectives: [],
  };
}

const seasonContext: NonNullable<GameStateData["season_context"]> = {
  phase: "InSeason",
  season_complete: false,
  season_start: "2026-08-01",
  season_end: "2027-05-31",
  days_until_season_start: null,
  transfer_window: {
    status: "Closed",
    opens_on: null,
    closes_on: null,
    days_until_opens: null,
    days_remaining: null,
  },
};

/** Both clubs have played both league fixtures: the case where counting fixtures says "complete". */
function completedTwoClubLeague(): GameStateData {
  const state = createGameState();
  const base = state.league!;
  state.league = {
    ...base,
    standings: [
      {
        team_id: "barcelona",
        played: 2,
        won: 1,
        drawn: 0,
        lost: 1,
        goals_for: 2,
        goals_against: 2,
        points: 3,
      },
      {
        team_id: "bayern",
        played: 2,
        won: 1,
        drawn: 0,
        lost: 1,
        goals_for: 2,
        goals_against: 2,
        points: 3,
      },
    ],
    fixtures: [1, 2].map((matchday) => ({
      id: `league-${matchday}`,
      matchday,
      date: `2026-08-0${matchday}`,
      home_team_id: matchday === 1 ? "barcelona" : "bayern",
      away_team_id: matchday === 1 ? "bayern" : "barcelona",
      competition: "League" as const,
      status: "Completed" as const,
      result: { home_goals: 1, away_goals: 1, home_scorers: [], away_scorers: [] },
    })),
  };
  return state;
}

describe("NextMatchDisplay", () => {
  it("renders the user team against the opponent when the user is away", () => {
    render(<NextMatchDisplay gameState={createGameState()} />);

    expect(screen.getByText("FC Barcelona")).toBeInTheDocument();
    expect(screen.getByText("Munich Bayern")).toBeInTheDocument();
    expect(screen.getAllByText("Munich Bayern")).toHaveLength(1);
    expect(screen.getByText("home.away")).toBeInTheDocument();
  });

  it("shows the translated name of a cup fixture", () => {
    const state = createGameState();
    const leagueFixture = state.league?.fixtures[0];
    if (!leagueFixture) throw new Error("expected the existing fixture");
    const cupFixture = {
      ...leagueFixture,
      id: "cup-fixture",
      competition_id: "cup-2026",
      competition: "Cup" as const,
    };
    state.competitions = [
      {
        id: "cup-2026",
        name: "National Cup",
        name_key: "tournaments.competitions.nationalCup",
        season: 2026,
        participant_ids: ["barcelona", "bayern"],
        fixtures: [cupFixture],
        standings: [],
      },
    ];
    state.active_competition_ids = ["cup-2026"];

    render(<NextMatchDisplay gameState={state} />);

    expect(screen.getByText("Copa Nacional")).toBeInTheDocument();
    expect(screen.queryByText("National Cup")).not.toBeInTheDocument();
  });

  /**
   * Given a league whose two clubs have played all their league fixtures, so that counting
   * fixtures says "complete", while the backend says the season is not over (a sibling league in
   * the pyramid is still playing)
   * When there is no next fixture
   * Then the card follows the backend and does not announce the season complete.
   */
  it("does not call the season complete when the backend says it is not", () => {
    const state = completedTwoClubLeague();
    state.season_context = { ...seasonContext, season_complete: false };

    render(<NextMatchDisplay gameState={state} />);

    expect(screen.getByText("home.noUpcomingOpponent")).toBeInTheDocument();
    expect(screen.queryByText("home.seasonComplete")).not.toBeInTheDocument();
  });

  /**
   * Given the backend says the season is over while the fixture count cannot tell
   * When there is no next fixture
   * Then the card announces the season complete.
   */
  it("calls the season complete when the backend says so", () => {
    const state = completedTwoClubLeague();
    state.league!.fixtures = [];
    state.season_context = { ...seasonContext, season_complete: true };

    render(<NextMatchDisplay gameState={state} />);

    expect(screen.getByText("home.seasonComplete")).toBeInTheDocument();
  });
});
