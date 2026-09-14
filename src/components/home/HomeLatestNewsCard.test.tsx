import { render, screen } from "@testing-library/react";
import { describe, expect, it, vi } from "vitest";

import type { NewsArticle, TeamData } from "../../store/gameStore";
import HomeLatestNewsCard from "./HomeLatestNewsCard";

vi.mock("react-i18next", () => ({
  useTranslation: () => ({
    t: (key: string) => {
      if (key === "home.allNews") return "All News";
      if (key === "home.latestNews") return "Latest News";
      if (key === "home.noNews") return "No news available.";
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

function createArticle(overrides: Partial<NewsArticle> = {}): NewsArticle {
  return {
    id: "news-1",
    headline: "Big win for Alpha FC",
    body: "Body",
    source: "OpenFoot Times",
    date: "2025-01-10",
    category: "LeagueRoundup",
    team_ids: [],
    player_ids: [],
    match_score: {
      home_team_id: "team-1",
      away_team_id: "team-2",
      home_goals: 2,
      away_goals: 1,
    },
    read: false,
    ...overrides,
  };
}

describe("HomeLatestNewsCard", () => {
  it("renders latest news headlines and scorelines", () => {
    render(
      <HomeLatestNewsCard
        articles={[createArticle()]}
        teams={[
          createTeam(),
          createTeam({ id: "team-2", name: "Beta FC", manager_id: "manager-2" }),
        ]}
        lang="en"
      />,
    );

    expect(screen.getByText("Latest News")).toBeInTheDocument();
    expect(screen.getByText("Big win for Alpha FC")).toBeInTheDocument();
    expect(screen.getByText(/2-1/)).toBeInTheDocument();
  });

  it("renders the empty state when there is no latest news", () => {
    render(<HomeLatestNewsCard articles={[]} teams={[]} lang="en" />);

    expect(screen.getByText("No news available.")).toBeInTheDocument();
  });
});
