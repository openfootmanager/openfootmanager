import { render, screen, within } from "@testing-library/react";
import { describe, expect, it, vi } from "vitest";

import type { NextOpponentWidgetData } from "./HomeTab.helpers";
import HomeNextOpponentCard from "./HomeNextOpponentCard";

vi.mock("react-i18next", () => ({
  useTranslation: () => ({
    t: (key: string, params?: Record<string, string | number>) => {
      if (key === "dashboard.schedule") return "Schedule";
      if (key === "home.nextOpponent") return "Next Opponent";
      if (key === "common.matchday") return `Matchday ${params?.n}`;
      if (key === "season.friendly") return "Friendly";
      if (key === "home.home") return "Home";
      if (key === "home.away") return "Away";
      if (key === "common.pts") return "pts";
      if (key === "home.noUpcomingOpponent") return "No upcoming league fixture.";
      // Portuguese form letters, so a dot showing the backend's English "W" would fail.
      if (key === "common.recentForm") return "Recent form";
      if (key === "common.formResult.W") return "V";
      if (key === "common.formResult.D") return "E";
      if (key === "common.formResult.L") return "D";
      return key;
    },
  }),
}));

function createNextOpponent(): NextOpponentWidgetData {
  return {
    fixture: {
      competitive: true,
      advancing_team_id: null,
      id: "fixture-1",
      matchday: 2,
      date: "2025-01-12",
      home_team_id: "team-1",
      away_team_id: "team-2",
      competition: "League",
      status: "Scheduled",
      result: null,
    },
    isHome: true,
    opponent: {
      id: "team-2",
      name: "Beta FC",
      short_name: "BET",
      country: "BR",
      city: "Rio",
      stadium_name: "Beta Arena",
      stadium_capacity: 50000,
      finance: 0,
      manager_id: "manager-2",
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
      form: ["W", "D", "W"],
      history: [],
    },
    recentForm: ["W", "D", "W"],
    standingPoints: 9,
    standingPosition: 2,
    competitionName: "Premier Division",
  };
}

describe("HomeNextOpponentCard", () => {
  it("renders the next opponent widget content", () => {
    render(<HomeNextOpponentCard nextOpponent={createNextOpponent()} lang="en" />);

    expect(screen.getByText("Next Opponent")).toBeInTheDocument();
    expect(screen.getByText("Beta FC")).toBeInTheDocument();
    expect(screen.getByText(/Matchday 2/i)).toBeInTheDocument();
    expect(screen.getByText("Home")).toBeInTheDocument();
    expect(screen.getByText("9 pts")).toBeInTheDocument();
  });

  /**
   * Given the next opponent's recent results from the backend, as English letters (W, D, W)
   * When the card draws them, in Portuguese
   * Then they are the shared form list, each in the locale's letter (V, E, V), not the backend's
   *      English letter on a raw-palette square as before.
   */
  it("draws the opponent's form in the player's own letters", () => {
    render(<HomeNextOpponentCard nextOpponent={createNextOpponent()} lang="en" />);

    const form = screen.getByRole("list", { name: "Recent form" });
    const letters = within(form)
      .getAllByRole("listitem")
      .map((item) => item.querySelector('[aria-hidden="true"]')?.textContent);
    expect(letters).toEqual(["V", "E", "V"]);
  });

  it("labels a cup tie with its competition name, not as a friendly", () => {
    const nextOpponent = createNextOpponent();
    nextOpponent.fixture = { ...nextOpponent.fixture, competition: "Cup" };
    nextOpponent.competitionName = "National Cup";

    render(<HomeNextOpponentCard nextOpponent={nextOpponent} lang="en" />);

    expect(screen.getByText(/National Cup/)).toBeInTheDocument();
    expect(screen.queryByText(/Friendly/)).not.toBeInTheDocument();
  });

  it("renders the empty state when no next opponent exists", () => {
    render(<HomeNextOpponentCard nextOpponent={null} lang="en" />);

    expect(screen.getByText("No upcoming league fixture.")).toBeInTheDocument();
  });
});
