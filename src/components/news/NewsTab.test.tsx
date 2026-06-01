import { fireEvent, render, screen } from "@testing-library/react";
import { describe, expect, it, vi } from "vitest";

import type { GameStateData, NewsArticle, TeamData } from "../../store/gameStore";
import NewsTab from "./NewsTab";

vi.mock("../season/AwardsCeremonyScreen", () => ({
  default: () => <div>Awards Ceremony Mock</div>,
}));

vi.mock("react-i18next", () => ({
  useTranslation: () => ({
    t: (key: string, params?: Record<string, string | number>) => {
      if (key === "news.noNews") return "No news";
      if (key === "news.newsWillAppear") return "News will appear soon";
      if (key === "common.all") return "All";
      if (key === "common.viewTeam") return "View team";
      if (key === "news.allTeams") return "All Teams";
      if (key === "news.backToNews") return "Back to news";
      if (key === "news.nArticles") return `${params?.count} articles`;
      if (key.startsWith("news.categories.")) {
        return key.replace("news.categories.", "");
      }
      return key;
    },
    i18n: { language: "en" },
  }),
}));

vi.mock("../../utils/backendI18n", () => ({
  resolveNewsArticle: (article: unknown) => article,
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

function createNewsArticle(overrides: Partial<NewsArticle> = {}): NewsArticle {
  return {
    id: "news-1",
    headline: "League headline",
    body: "Longer body text",
    source: "OpenFoot Times",
    date: "2026-08-01",
    category: "LeagueRoundup",
    team_ids: ["team-1", "team-2"],
    player_ids: [],
    match_score: null,
    read: false,
    ...overrides,
  };
}

function createGameState(news: NewsArticle[]): GameStateData {
  return {
    clock: {
      current_date: "2026-08-01T00:00:00Z",
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
    news,
    league: null,
    scouting_assignments: [],
    board_objectives: [],
  };
}

describe("NewsTab", () => {
  it("renders the empty state when there is no news", () => {
    render(<NewsTab gameState={createGameState([])} onSelectTeam={vi.fn()} />);

    expect(screen.getByText("No news")).toBeInTheDocument();
    expect(screen.getByText("News will appear soon")).toBeInTheDocument();
  });

  it("opens article detail and routes team clicks from the detail footer", () => {
    const onSelectTeam = vi.fn();

    render(
      <NewsTab
        gameState={createGameState([createNewsArticle()])}
        onSelectTeam={onSelectTeam}
      />,
    );

    fireEvent.click(screen.getByRole("button", { name: /League headline/i }));

    expect(screen.getByText("Back to news")).toBeInTheDocument();
    expect(screen.getAllByRole("button", { name: "Alpha FC" }).length).toBeGreaterThan(0);

    fireEvent.click(screen.getAllByRole("button", { name: "Alpha FC" })[0]);

    expect(onSelectTeam).toHaveBeenCalledWith("team-1");
  });

  it("offers linked-team context menu actions on article cards", () => {
    const onSelectTeam = vi.fn();

    render(
      <NewsTab
        gameState={createGameState([createNewsArticle()])}
        onSelectTeam={onSelectTeam}
      />,
    );

    fireEvent.contextMenu(screen.getByTestId("news-article-news-1"));
    fireEvent.click(screen.getByRole("button", { name: "View team: Beta FC" }));

    expect(onSelectTeam).toHaveBeenCalledWith("team-2");
  });

  it("filters dedicated transfer roundup articles by category", () => {
    render(
      <NewsTab
        gameState={createGameState([
          createNewsArticle({
            id: "news-transfer-roundup",
            headline: "Transfer roundup headline",
            category: "TransferRoundup",
          }),
          createNewsArticle({
            id: "news-league-roundup",
            headline: "League roundup headline",
            category: "LeagueRoundup",
            date: "2026-07-31",
          }),
        ])}
        onSelectTeam={vi.fn()}
      />,
    );

    fireEvent.click(screen.getByRole("button", { name: "TransferRoundup" }));

    expect(
      screen.getByRole("button", { name: /Transfer roundup headline/i }),
    ).toBeInTheDocument();
    expect(
      screen.queryByRole("button", { name: /League roundup headline/i }),
    ).not.toBeInTheDocument();
  });

  it("opens the dedicated awards ceremony page from a season awards article", () => {
    render(
      <NewsTab
        gameState={createGameState([
          createNewsArticle({
            id: "season_awards_6",
            headline: "Season awards night",
            category: "Editorial",
            i18n_params: {
              season: "6",
              leagueName: "Premier League",
              goldenBootWinner: "Victor Vale",
              goldenBootGoals: "24",
              goldenBootTeam: "Alpha FC",
              potyWinner: "Victor Vale",
              potyRating: "7.8",
              potyTeam: "Alpha FC",
              managerWinner: "Jane Doe",
              managerTeam: "Alpha FC",
            },
          }),
        ])}
        onSelectTeam={vi.fn()}
      />,
    );

    fireEvent.click(screen.getByRole("button", { name: /Season awards night/i }));

    expect(screen.getByText("Awards Ceremony Mock")).toBeInTheDocument();
  });
});