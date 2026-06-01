import { fireEvent, render, screen } from "@testing-library/react";
import { describe, expect, it, vi } from "vitest";

import type { GameStateData, ManagerData, TeamData } from "../../store/gameStore";
import ManagersWorldTab from "./ManagersWorldTab";

vi.mock("react-i18next", () => ({
  useTranslation: () => ({
    t: (key: string, params?: Record<string, string | number>) => {
      if (key === "managersWorld.title") return "Managers";
      if (key === "managersWorld.subtitle") return "League managers and open jobs";
      if (key === "managersWorld.vacancy") return "Vacancy";
      if (key === "managersWorld.employed") return "Employed";
      if (key === "managersWorld.unemployed") return "Unemployed";
      if (key === "managersWorld.reputation") return "Reputation";
      if (key === "managersWorld.winRate") return "Win rate";
      if (key === "managersWorld.matches") return "Matches";
      if (key === "managersWorld.currentClub") return "Current club";
      if (key === "managersWorld.openRole") return `Open role at ${params?.team}`;
      if (key === "managersWorld.vacancyBadge") return "Board searching";
      if (key === "managersWorld.noManagers") return "No managers available";
      return key;
    },
    i18n: { language: "en" },
  }),
}));

function createManager(overrides: Partial<ManagerData> = {}): ManagerData {
  return {
    id: "manager-1",
    first_name: "Jane",
    last_name: "Doe",
    date_of_birth: "1980-01-01",
    nationality: "ENG",
    football_nation: "ENG",
    reputation: 72,
    satisfaction: 63,
    fan_approval: 61,
    team_id: "team-1",
    career_stats: {
      matches_managed: 120,
      wins: 64,
      draws: 22,
      losses: 34,
      trophies: 2,
      best_finish: 1,
    },
    career_history: [],
    ...overrides,
  };
}

function createTeam(overrides: Partial<TeamData> = {}): TeamData {
  return {
    id: "team-1",
    name: "Alpha FC",
    short_name: "ALP",
    country: "ENG",
    city: "London",
    stadium_name: "Alpha Ground",
    stadium_capacity: 30000,
    finance: 1000000,
    manager_id: "manager-1",
    reputation: 650,
    wage_budget: 500000,
    transfer_budget: 750000,
    season_income: 0,
    season_expenses: 0,
    formation: "4-3-3",
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

function createGameState(): GameStateData {
  return {
    clock: {
      current_date: "2026-08-10T00:00:00Z",
      start_date: "2026-07-01T00:00:00Z",
    },
    manager: createManager(),
    managers: [
      createManager(),
      createManager({
        id: "manager-2",
        first_name: "Marco",
        last_name: "Rossi",
        team_id: null,
        reputation: 58,
        career_stats: {
          matches_managed: 84,
          wins: 35,
          draws: 18,
          losses: 31,
          trophies: 0,
          best_finish: 4,
        },
      }),
    ],
    teams: [
      createTeam(),
      createTeam({
        id: "team-2",
        name: "Beta United",
        short_name: "BET",
        manager_id: null,
        history: [
          {
            season: 2024,
            league_position: 7,
            played: 32,
            won: 11,
            drawn: 8,
            lost: 13,
            goals_for: 40,
            goals_against: 44,
          },
          {
            season: 2025,
            league_position: 5,
            played: 46,
            won: 18,
            drawn: 12,
            lost: 16,
            goals_for: 58,
            goals_against: 52,
          },
        ],
      }),
    ],
    players: [],
    staff: [],
    messages: [],
    news: [],
    league: null,
    scouting_assignments: [],
    board_objectives: [],
  };
}

describe("ManagersWorldTab", () => {
  it("renders employed managers, unemployed managers, and explicit vacancies", () => {
    render(<ManagersWorldTab gameState={createGameState()} />);

    expect(screen.getByText("Managers")).toBeInTheDocument();
    expect(screen.getByText("Jane Doe")).toBeInTheDocument();
    expect(screen.getByText("Marco Rossi")).toBeInTheDocument();
    expect(screen.getByText("Alpha FC")).toBeInTheDocument();
    expect(screen.getAllByText("Vacancy")).toHaveLength(2);
    expect(screen.getByText("Open role at Beta United")).toBeInTheDocument();
  });

  it("routes team selection from manager clubs and vacancy cards", () => {
    const onSelectTeam = vi.fn();

    render(
      <ManagersWorldTab
        gameState={createGameState()}
        onSelectTeam={onSelectTeam}
      />,
    );

    fireEvent.click(screen.getByRole("button", { name: "Alpha FC" }));
    fireEvent.click(screen.getByRole("button", { name: "Open role at Beta United" }));

    expect(onSelectTeam).toHaveBeenNthCalledWith(1, "team-1");
    expect(onSelectTeam).toHaveBeenNthCalledWith(2, "team-2");
  });

  it("shows vacancy match totals from the latest recorded season", () => {
    render(<ManagersWorldTab gameState={createGameState()} />);

    expect(screen.getByText("46")).toBeInTheDocument();
  });
});