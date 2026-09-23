import { render, screen } from "@testing-library/react";
import { describe, expect, it, vi } from "vitest";

import type { TeamData } from "../../store/gameStore";
import type { HomeRecentResult } from "./HomeTab.helpers";
import HomeRecentResultsCard from "./HomeRecentResultsCard";

vi.mock("react-i18next", () => ({
  useTranslation: () => ({
    t: (key: string) => {
      if (key === "dashboard.schedule") return "Schedule";
      if (key === "home.recentResults") return "Recent Results";
      if (key === "home.noMatches") return "No matches played yet.";
      if (key === "home.home") return "Home";
      if (key === "home.away") return "Away";
      return key;
    },
  }),
}));

function createTeam(overrides: Partial<TeamData> = {}): TeamData {
  return {
    id: "team-1",
    name: "Alpha FC",
    short_name: "ALP",
    country: "BR",
    city: "Rio",
    stadium_name: "Alpha Arena",
    stadium_capacity: 50000,
    finance: 0,
    manager_id: "manager-1",
    reputation: 50,
    wage_budget: 0,
    transfer_budget: 0,
    season_income: 0,
    season_expenses: 0,
    formation: "4-4-2",
    play_style: "Balanced",
    training_focus: "Physical",
    training_intensity: "Medium",
    training_schedule: "Balanced",
    founded_year: 1900,
    colors: { primary: "#111111", secondary: "#ffffff" },
    starting_xi_ids: [],
    form: [],
    history: [],
    ...overrides,
  };
}

function createRecentResult(): HomeRecentResult {
  return {
    fixture: {
      id: "fixture-1",
      matchday: 1,
      date: "2025-01-10",
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
    },
    isHome: true,
    myGoals: 2,
    opponentGoals: 1,
    opponentId: "team-2",
    resultCode: "W",
  };
}

describe("HomeRecentResultsCard", () => {
  it("renders recent results", () => {
    render(
      <HomeRecentResultsCard
        recentResults={[createRecentResult()]}
        teams={[
          createTeam(),
          createTeam({ id: "team-2", name: "Beta FC", manager_id: "manager-2" }),
        ]}
      />,
    );

    expect(screen.getByText("Recent Results")).toBeInTheDocument();
    expect(screen.getByText("Beta FC")).toBeInTheDocument();
    expect(screen.getByText("2 - 1")).toBeInTheDocument();
  });

  it("renders the empty state when there are no recent results", () => {
    render(<HomeRecentResultsCard recentResults={[]} teams={[]} />);

    expect(screen.getByText("No matches played yet.")).toBeInTheDocument();
  });
});
