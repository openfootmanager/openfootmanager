import { fireEvent, render, screen } from "@testing-library/react";
import { describe, expect, it, vi } from "vitest";

import RoundDigestScreen from "./RoundDigestScreen";
import type { GameStateData } from "../../store/gameStore";
import type { MatchSnapshot, RoundSummary } from "./types";

vi.mock("@tauri-apps/api/core", () => ({ invoke: vi.fn() }));

vi.mock("react-i18next", () => ({
  useTranslation: () => ({
    t: (key: string, params?: Record<string, string | number>) => {
      if (key === "schedule.matchday") return `Matchday ${params?.number}`;
      if (key === "match.assist") return `assist: ${params?.name}`;
      if (key === "match.subFor") return `for ${params?.name}`;
      return key;
    },
  }),
}));

vi.mock("../ui", () => ({
  Badge: ({ children }: { children: React.ReactNode }) => <span>{children}</span>,
  TeamLogo: ({ team }: { team: { name?: string; short_name?: string } }) => (
    <span>{team?.short_name ?? team?.name}</span>
  ),
}));

vi.mock("./PostMatchHelpers", () => ({
  QuickStat: ({
    label,
    home,
    away,
  }: {
    label: string;
    home: string | number;
    away: string | number;
  }) => (
    <div>
      {label}: {home} – {away}
    </div>
  ),
}));

vi.mock("./helpers", () => ({
  getEventDisplay: () => ({ icon: "⚽", color: "text-green-500" }),
  makeTeamFallback: (name: string) => ({ name, short_name: name.slice(0, 3) }),
}));

function makeSnapshot(): MatchSnapshot {
  return {
    phase: "FullTime",
    current_minute: 90,
    home_score: 2,
    away_score: 1,
    possession: "Home",
    ball_zone: "Midfield",
    home_team: {
      id: "team1",
      name: "Alpha FC",
      formation: "4-4-2",
      play_style: "Balanced",
      players: [],
    },
    away_team: {
      id: "team2",
      name: "Beta FC",
      formation: "4-4-2",
      play_style: "Balanced",
      players: [],
    },
    home_bench: [],
    away_bench: [],
    home_possession_pct: 55,
    away_possession_pct: 45,
    events: [],
    home_subs_made: 0,
    away_subs_made: 0,
    max_subs: 5,
    home_set_pieces: {
      free_kick_taker: null,
      corner_taker: null,
      penalty_taker: null,
      captain: null,
    },
    away_set_pieces: {
      free_kick_taker: null,
      corner_taker: null,
      penalty_taker: null,
      captain: null,
    },
    substitutions: [],
    allows_extra_time: false,
    home_yellows: {},
    away_yellows: {},
    sent_off: [],
  };
}

function makeGameState(): GameStateData {
  return {
    clock: { current_date: "2026-08-10", start_date: "2026-08-01" },
    manager: {
      id: "mgr1",
      first_name: "Test",
      last_name: "Manager",
      date_of_birth: "1980-01-01",
      nationality: "GB",
      reputation: 50,
      satisfaction: 50,
      fan_approval: 50,
      team_id: "team1",
      career_stats: {
        matches_managed: 1,
        wins: 1,
        draws: 0,
        losses: 0,
        trophies: 0,
        best_finish: null,
      },
      career_history: [],
    },
    teams: [
      {
        id: "team1",
        name: "Alpha FC",
        short_name: "ALP",
        country: "England",
        city: "Alpha",
        stadium_name: "Alpha Park",
        stadium_capacity: 20000,
        finance: 1000000,
        manager_id: "mgr1",
        reputation: 50,
        wage_budget: 100000,
        transfer_budget: 500000,
        season_income: 0,
        season_expenses: 0,
        formation: "4-4-2",
        play_style: "Balanced",
        training_focus: "General",
        training_intensity: "Balanced",
        training_schedule: "Balanced",
        founded_year: 1900,
        colors: { primary: "#00ff00", secondary: "#ffffff" },
        starting_xi_ids: [],
        form: [],
        history: [],
      },
      {
        id: "team2",
        name: "Beta FC",
        short_name: "BET",
        country: "England",
        city: "Beta",
        stadium_name: "Beta Park",
        stadium_capacity: 20000,
        finance: 1000000,
        manager_id: null,
        reputation: 50,
        wage_budget: 100000,
        transfer_budget: 500000,
        season_income: 0,
        season_expenses: 0,
        formation: "4-4-2",
        play_style: "Balanced",
        training_focus: "General",
        training_intensity: "Balanced",
        training_schedule: "Balanced",
        founded_year: 1900,
        colors: { primary: "#0000ff", secondary: "#ffffff" },
        starting_xi_ids: [],
        form: [],
        history: [],
      },
      {
        id: "team3",
        name: "Gamma FC",
        short_name: "GAM",
        country: "England",
        city: "Gamma",
        stadium_name: "Gamma Park",
        stadium_capacity: 20000,
        finance: 1000000,
        manager_id: null,
        reputation: 60,
        wage_budget: 100000,
        transfer_budget: 500000,
        season_income: 0,
        season_expenses: 0,
        formation: "4-4-2",
        play_style: "Balanced",
        training_focus: "General",
        training_intensity: "Balanced",
        training_schedule: "Balanced",
        founded_year: 1900,
        colors: { primary: "#ff0000", secondary: "#ffffff" },
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
      id: "league1",
      name: "Test League",
      fixtures: [
        {
          id: "fix2",
          competition: "League" as const,
          home_team_id: "team3",
          away_team_id: "team2",
          date: "2026-08-10",
          status: "Completed" as const,
          result: {
            home_goals: 3,
            away_goals: 0,
            home_scorers: [],
            away_scorers: [],
            report: null,
          },
          round: 1,
          matchday: 1,
        },
      ],
    },
    scouting_assignments: [],
    board_objectives: [],
  } as unknown as GameStateData;
}

function makeRoundSummary(): RoundSummary {
  return {
    matchday: 1,
    is_complete: true,
    pending_fixture_count: 0,
    completed_results: [
      {
        fixture_id: "fix1",
        home_team_id: "team1",
        home_team_name: "Alpha FC",
        away_team_id: "team2",
        away_team_name: "Beta FC",
        home_goals: 2,
        away_goals: 1,
      },
      {
        fixture_id: "fix2",
        home_team_id: "team3",
        home_team_name: "Gamma FC",
        away_team_id: "team2",
        away_team_name: "Beta FC",
        home_goals: 3,
        away_goals: 0,
      },
    ],
    standings_delta: [
      {
        team_id: "team1",
        team_name: "Alpha FC",
        previous_position: 3,
        current_position: 1,
        points: 3,
        points_delta: 3,
      },
      {
        team_id: "team3",
        team_name: "Gamma FC",
        previous_position: 1,
        current_position: 2,
        points: 3,
        points_delta: 3,
      },
    ],
    top_scorer_delta: [
      {
        player_id: "p1",
        player_name: "Alice",
        team_id: "team1",
        previous_rank: 2,
        current_rank: 1,
        previous_goals: 0,
        current_goals: 1,
      },
    ],
    notable_upset: null,
  };
}

const defaultProps = {
  snapshot: makeSnapshot(),
  gameState: makeGameState(),
  currentFixture: {
    id: "fix1",
    competition: "League" as const,
    home_team_id: "team1",
    away_team_id: "team2",
    date: "2026-08-10",
    status: "Completed" as const,
    result: {
      home_goals: 2,
      away_goals: 1,
      home_scorers: [],
      away_scorers: [],
      report: null,
    },
    round: 1,
    matchday: 1,
  },
  userSide: "Home" as const,
  isLeagueFixture: true,
  roundSummary: makeRoundSummary(),
  onPressConference: vi.fn(),
  onFinish: vi.fn(),
};

describe("RoundDigestScreen", function () {
  it("renders the matchday heading and league name for a league fixture", function () {
    render(<RoundDigestScreen {...defaultProps} />);

    expect(screen.getByText(/Matchday 1/)).toBeInTheDocument();
    expect(screen.getByText("match.roundSummary")).toBeInTheDocument();
  });

  it("renders the hero result card with score and win badge", function () {
    render(<RoundDigestScreen {...defaultProps} />);

    expect(screen.getByText("match.yourResult")).toBeInTheDocument();
    expect(screen.getByText("match.victory")).toBeInTheDocument();
    expect(screen.getAllByText("Alpha FC").length).toBeGreaterThan(0);
    expect(screen.getAllByText("Beta FC").length).toBeGreaterThan(0);
  });

  it("renders the standings and top scorers for a league fixture", function () {
    render(<RoundDigestScreen {...defaultProps} />);

    expect(screen.getByText("tournaments.leagueTable")).toBeInTheDocument();
    expect(screen.getByText("tournaments.topScorers")).toBeInTheDocument();
    expect(screen.getByText("Alice")).toBeInTheDocument();
  });

  it("does not render standings or top scorers for a friendly", function () {
    render(
      <RoundDigestScreen
        {...defaultProps}
        isLeagueFixture={false}
        roundSummary={null}
      />,
    );

    expect(
      screen.queryByText("tournaments.leagueTable"),
    ).not.toBeInTheDocument();
    expect(
      screen.queryByText("tournaments.topScorers"),
    ).not.toBeInTheDocument();
    expect(screen.getAllByText("match.otherMatches").length).toBeGreaterThan(0);
  });

  it("renders position context with points when standings are available", function () {
    render(<RoundDigestScreen {...defaultProps} />);

    expect(screen.getByText(/match\.pts/)).toBeInTheDocument();
  });

  it("renders the notable upset card when one is present", function () {
    const summary = {
      ...makeRoundSummary(),
      notable_upset: {
        fixture_id: "fix3",
        favorite_team_id: "team3",
        favorite_team_name: "Gamma FC",
        favorite_strength: 80,
        underdog_team_id: "team2",
        underdog_team_name: "Beta FC",
        underdog_strength: 40,
        strength_gap: 40,
        home_goals: 1,
        away_goals: 0,
      },
    };

    render(<RoundDigestScreen {...defaultProps} roundSummary={summary} />);

    expect(screen.getByText("match.notableUpset")).toBeInTheDocument();
    expect(screen.getAllByText(/Beta FC/).length).toBeGreaterThan(0);
  });

  it("calls onPressConference when the press conference button is clicked", function () {
    const onPressConference = vi.fn();
    render(
      <RoundDigestScreen {...defaultProps} onPressConference={onPressConference} />,
    );

    fireEvent.click(screen.getByText("match.pressConference"));
    expect(onPressConference).toHaveBeenCalledOnce();
  });

  it("calls onFinish when the skip button is clicked", function () {
    const onFinish = vi.fn();
    render(<RoundDigestScreen {...defaultProps} onFinish={onFinish} />);

    fireEvent.click(screen.getByText("match.skip"));
    expect(onFinish).toHaveBeenCalledOnce();
  });

  it("opens and closes the other-match details modal", function () {
    const gameStateWithReport = {
      ...makeGameState(),
      league: {
        ...makeGameState().league!,
        fixtures: [
          {
            id: "fix2",
            competition: "League" as const,
            home_team_id: "team3",
            away_team_id: "team2",
            date: "2026-08-10",
            status: "Completed" as const,
            result: {
              home_goals: 3,
              away_goals: 0,
              home_scorers: [],
              away_scorers: [],
              report: {
                events: [],
                home_stats: {
                  possession_pct: 60,
                  shots: 10,
                  shots_on_target: 5,
                  fouls: 8,
                  corners: 4,
                  yellow_cards: 1,
                },
                away_stats: {
                  possession_pct: 40,
                  shots: 5,
                  shots_on_target: 2,
                  fouls: 12,
                  corners: 2,
                  yellow_cards: 2,
                },
              },
            },
            round: 1,
            matchday: 1,
          },
        ],
      },
    } as unknown as GameStateData;

    render(
      <RoundDigestScreen
        {...defaultProps}
        gameState={gameStateWithReport}
      />,
    );

    fireEvent.click(screen.getByText("match.viewDetails"));

    expect(screen.getByRole("dialog")).toBeInTheDocument();
    expect(screen.getByText("match.matchDetails")).toBeInTheDocument();

    fireEvent.click(screen.getByText("common.close"));

    expect(screen.queryByRole("dialog")).not.toBeInTheDocument();
  });
});
