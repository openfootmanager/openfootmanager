import { render, screen } from "@testing-library/react";
import { describe, expect, it, vi } from "vitest";

import HomeTab from "./HomeTab";
import type {
  FixtureData,
  GameStateData,
  NewsArticle,
  PlayerData,
  TeamData,
} from "../../store/gameStore";

const backendI18nMocks = vi.hoisted(() => ({
  resolveBoardObjective: vi.fn((value: unknown) => value),
  resolveMessage: vi.fn((value: unknown) => value),
  resolveNewsArticle: vi.fn((value: unknown) => value),
}));

vi.mock("../NextMatchDisplay", () => ({
  default: () => <div data-testid="next-match-display" />,
}));

vi.mock("./HomeSquadOverviewCard", () => ({
  default: ({ avgCondition, avgOvr, exhaustedCount }: { avgCondition: number; avgOvr: number; exhaustedCount: number }) => (
    <div data-testid="home-squad-overview">{`${avgCondition}|${avgOvr}|${exhaustedCount}`}</div>
  ),
}));

vi.mock("./HomeUnavailablePlayersCard", () => ({
  default: ({ players }: { players: Array<{ full_name: string }> }) => (
    <div data-testid="home-unavailable-players">{players.map((player) => player.full_name).join(",")}</div>
  ),
}));

vi.mock("../../utils/backendI18n", () => ({
  resolveBoardObjective: backendI18nMocks.resolveBoardObjective,
  resolveMessage: backendI18nMocks.resolveMessage,
  resolveNewsArticle: backendI18nMocks.resolveNewsArticle,
}));

vi.mock("react-i18next", () => ({
  useTranslation: () => ({
    i18n: { language: "en" },
    t: (key: string, params?: Record<string, string | number>) => {
      if (key === "home.nextOpponent") return "Next Opponent";
      if (key === "home.noUpcomingOpponent")
        return "No upcoming league fixture.";
      if (key === "home.leagueDigest") return "League Digest";
      if (key === "home.noLeagueDigest") return "No league digest yet.";
      if (key === "dashboard.news") return "News";
      if (key === "dashboard.schedule") return "Schedule";
      if (key === "home.matchdayN") return `Matchday ${params?.n}`;
      if (key === "season.friendly") return "Friendly";
      if (key === "home.home") return "Home";
      if (key === "home.away") return "Away";
      if (key === "home.pointsShort") return `${params?.points} pts`;
      if (key === "news.categories.LeagueRoundup") return "League Roundup";
      if (key === "news.categories.StandingsUpdate") return "Standings";
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
    colors: {
      primary: "#111111",
      secondary: "#ffffff",
    },
    starting_xi_ids: [],
    match_roles: {
      captain: null,
      vice_captain: null,
      penalty_taker: null,
      free_kick_taker: null,
      corner_taker: null,
    },
    form: [],
    history: [],
    ...overrides,
  };
}

function createPlayer(overrides: Partial<PlayerData> = {}): PlayerData {
  return {
    id: "player-1",
    match_name: "J. Smith",
    full_name: "John Smith",
    date_of_birth: "2000-01-01",
    nationality: "BR",
    position: "Forward",
    natural_position: "Forward",
    alternate_positions: [],
    training_focus: null,
    attributes: {
      pace: 10,
      stamina: 10,
      strength: 10,
      agility: 10,
      passing: 10,
      shooting: 10,
      tackling: 10,
      dribbling: 10,
      defending: 10,
      positioning: 10,
      vision: 10,
      decisions: 10,
      composure: 10,
      aggression: 10,
      teamwork: 10,
      leadership: 10,
      handling: 10,
      reflexes: 10,
      aerial: 10,
    },
    condition: 80,
    morale: 80,
    injury: null,
    team_id: "team-1",
    contract_end: null,
    wage: 0,
    market_value: 0,
    stats: {
      appearances: 0,
      goals: 0,
      assists: 0,
      clean_sheets: 0,
      yellow_cards: 0,
      red_cards: 0,
      avg_rating: 0,
      minutes_played: 0,
    },
    career: [],
    transfer_listed: false,
    loan_listed: false,
    transfer_offers: [],
    traits: [],
    ...overrides,
  };
}

function createFixture(overrides: Partial<FixtureData> = {}): FixtureData {
  return {
    id: "fixture-1",
    matchday: 2,
    date: "2025-01-12",
    home_team_id: "team-1",
    away_team_id: "team-2",
    competition: "League",
    status: "Scheduled",
    result: null,
    ...overrides,
  };
}

function createNewsArticle(overrides: Partial<NewsArticle> = {}): NewsArticle {
  return {
    id: "news-1",
    headline: "Headline",
    body: "Body",
    source: "OpenFoot Times",
    date: "2025-01-10",
    category: "LeagueRoundup",
    team_ids: [],
    player_ids: [],
    match_score: null,
    read: false,
    ...overrides,
  };
}

function createGameState(
  overrides: Partial<GameStateData> = {},
): GameStateData {
  return {
    clock: {
      current_date: "2025-01-20T00:00:00Z",
      start_date: "2025-01-01T00:00:00Z",
    },
    manager: {
      id: "manager-1",
      first_name: "Jane",
      last_name: "Doe",
      date_of_birth: "1980-01-01",
      nationality: "BR",
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
      createTeam({
        id: "team-2",
        name: "Beta FC",
        short_name: "BET",
        form: ["W", "D", "W"],
      }),
    ],
    players: [createPlayer()],
    staff: [],
    messages: [],
    news: [],
    league: {
      id: "league-1",
      name: "League",
      season: 1,
      fixtures: [createFixture()],
      standings: [
        {
          team_id: "team-2",
          played: 1,
          won: 1,
          drawn: 0,
          lost: 0,
          goals_for: 2,
          goals_against: 0,
          points: 3,
        },
        {
          team_id: "team-1",
          played: 1,
          won: 0,
          drawn: 0,
          lost: 1,
          goals_for: 0,
          goals_against: 2,
          points: 0,
        },
      ],
    },
    scouting_assignments: [],
    board_objectives: [],
    ...overrides,
  };
}

describe("HomeTab", function (): void {
  it("resolves latest news articles before rendering the home widget", function (): void {
    backendI18nMocks.resolveNewsArticle.mockImplementationOnce(
      (value: unknown) => ({
        ...(value as NewsArticle),
        headline: "Resolved headline",
        source: "Resolved source",
      }),
    );

    render(
      <HomeTab
        gameState={createGameState({
          news: [
            createNewsArticle({
              id: "news-resolve-1",
              headline: "Fallback headline",
              source: "Fallback source",
              category: "SeasonPreview",
              date: "2025-01-16",
            }),
          ],
        })}
        visitedOnboardingTabs={new Set<string>()}
      />,
    );

    expect(backendI18nMocks.resolveNewsArticle).toHaveBeenNthCalledWith(
      1,
      expect.objectContaining({ id: "news-resolve-1" }),
      0,
      expect.any(Array),
    );
    expect(screen.getByText("Resolved headline")).toBeInTheDocument();
    expect(screen.getByText(/Resolved source/)).toBeInTheDocument();
  });

  it("renders the next opponent and league digest widgets when data is available", function (): void {
    render(
      <HomeTab
        gameState={createGameState({
          news: [
            createNewsArticle({
              id: "digest-1",
              headline: "Standings headline",
              category: "StandingsUpdate",
              date: "2025-01-15",
            }),
          ],
        })}
        visitedOnboardingTabs={new Set<string>()}
      />,
    );

    expect(screen.getByText("Next Opponent")).toBeInTheDocument();
    expect(screen.getByText("Beta FC")).toBeInTheDocument();
    expect(screen.getByText("League Digest")).toBeInTheDocument();
    expect(screen.getAllByText("Standings headline").length).toBeGreaterThan(0);
  });

  it("renders widget empty states when opponent and digest data are unavailable", function (): void {
    render(
      <HomeTab
        gameState={createGameState({
          league: {
            id: "league-1",
            name: "League",
            season: 1,
            fixtures: [],
            standings: [],
          },
          news: [],
        })}
        visitedOnboardingTabs={new Set<string>()}
      />,
    );

    expect(screen.getByText("No upcoming league fixture.")).toBeInTheDocument();
    expect(screen.getByText("No league digest yet.")).toBeInTheDocument();
  });

  it("keeps youth academy players out of first-team home summaries", function (): void {
    render(
      <HomeTab
        gameState={createGameState({
          players: [
            createPlayer({
              id: "senior-1",
              full_name: "Senior Starter",
              condition: 80,
              injury: {
                name: "hamstring_strain",
                days_remaining: 5,
              },
            }),
            createPlayer({
              id: "youth-1",
              full_name: "Youth Prospect",
              condition: 10,
              injury: {
                name: "ankle_sprain",
                days_remaining: 14,
              },
              squad_role: "Youth",
            }),
          ],
        })}
        visitedOnboardingTabs={new Set<string>()}
      />,
    );

    expect(screen.getByTestId("home-squad-overview")).toHaveTextContent(
      "80|1|0",
    );
    expect(screen.getByTestId("home-unavailable-players")).toHaveTextContent(
      "Senior Starter",
    );
    expect(screen.getByTestId("home-unavailable-players")).not.toHaveTextContent(
      "Youth Prospect",
    );
  });
});
