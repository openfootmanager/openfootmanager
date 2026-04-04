import { render, screen } from "@testing-library/react";
import { describe, expect, it, vi } from "vitest";

import type { GameStateData, TeamData } from "../../store/gameStore";
import ManagerTab from "./ManagerTab";

vi.mock("react-i18next", () => ({
  useTranslation: () => ({
    t: (key: string, params?: Record<string, string | number>) => {
      if (key === "manager.born") return "Born";
      if (key === "manager.managerOf") return `Manager of ${params?.team}`;
      if (key === "manager.reputation") return "Reputation";
      if (key === "manager.careerStats") return "Career Stats";
      if (key === "manager.matches") return "Matches";
      if (key === "manager.wins") return "Wins";
      if (key === "manager.draws") return "Draws";
      if (key === "manager.losses") return "Losses";
      if (key === "manager.trophies") return "Trophies";
      if (key === "manager.winPercent") return "Win %";
      if (key === "manager.boardStatus") return "Board Status";
      if (key === "manager.board") return "Board";
      if (key === "manager.fans") return "Fans";
      if (key === "manager.boardVeryPleased") return "Board very pleased";
      if (key === "manager.boardSatisfied") return "Board satisfied";
      if (key === "manager.boardConcerns") return "Board concerns";
      if (key === "manager.boardThreat") return "Board threat";
      if (key === "manager.fanAdore") return "Fans adore";
      if (key === "manager.fanBehind") return "Fans behind you";
      if (key === "manager.fanMixed") return "Fans mixed";
      if (key === "manager.fanRestless") return "Fans restless";
      if (key === "manager.fanUnrest") return "Fan unrest";
      if (key === "manager.careerHistory") return "Career History";
      if (key === "manager.club") return "Club";
      if (key === "manager.period") return "Period";
      if (key === "common.played") return "Played";
      if (key === "common.won") return "Won";
      if (key === "common.drawn") return "Drawn";
      if (key === "common.lost") return "Lost";
      if (key === "common.present") return "Present";
      return key;
    },
    i18n: { language: "en" },
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

function createGameState(withHistory: boolean): GameStateData {
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
      reputation: 72,
      satisfaction: 83,
      fan_approval: 64,
      team_id: "team-1",
      career_stats: {
        matches_managed: 100,
        wins: 55,
        draws: 20,
        losses: 25,
        trophies: 2,
        best_finish: 1,
      },
      career_history: withHistory
        ? [
          {
            team_id: "team-0",
            team_name: "Old Town FC",
            start_date: "2021-07-01",
            end_date: null,
            matches: 40,
            wins: 20,
            draws: 10,
            losses: 10,
            best_league_position: 3,
          },
        ]
        : [],
    },
    teams: [createTeam()],
    players: [],
    staff: [],
    messages: [],
    news: [],
    league: null,
    scouting_assignments: [],
    board_objectives: [],
  };
}

describe("ManagerTab", () => {
  it("renders manager profile, win percentage, and career history", () => {
    render(<ManagerTab gameState={createGameState(true)} />);

    expect(screen.getByText("Jane Doe")).toBeInTheDocument();
    expect(screen.getByText("Manager of Alpha FC")).toBeInTheDocument();
    expect(screen.getByText("55%")).toBeInTheDocument();
    expect(screen.getByText("Career History")).toBeInTheDocument();
    expect(screen.getByText("Old Town FC")).toBeInTheDocument();
  });
});