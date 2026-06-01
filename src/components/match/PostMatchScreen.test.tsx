import { fireEvent, render, screen } from "@testing-library/react";
import { describe, expect, it, vi } from "vitest";

import PostMatchScreen from "./PostMatchScreen";
import type { FixtureData, GameStateData } from "../../store/gameStore";
import { ThemeProvider } from "../../context/ThemeContext";

vi.mock("@tauri-apps/api/core", () => ({
  invoke: vi.fn(),
}));

Object.defineProperty(window, "matchMedia", {
  writable: true,
  value: vi.fn().mockImplementation((query: string) => ({
    matches: query === "(prefers-color-scheme: dark)",
    media: query,
    onchange: null,
    addListener: vi.fn(),
    removeListener: vi.fn(),
    addEventListener: vi.fn(),
    removeEventListener: vi.fn(),
    dispatchEvent: vi.fn(),
  })),
});

vi.mock("react-i18next", () => ({
  useTranslation: () => ({
    t: (key: string, params?: Record<string, string | number>) => {
      if (key === "schedule.matchday") {
        return `Matchday ${params?.number}`;
      }
      if (key === "match.otherMatches") {
        return "Other Matches";
      }
      if (key === "match.otherMatchesToday") {
        return "Other Matches Today";
      }
      if (key === "match.otherMatchesUnavailable") {
        return "Other match context unavailable for this fixture yet.";
      }
      if (key === "match.viewDetails") {
        return "View details";
      }
      if (key === "match.matchDetails") {
        return "Match Details";
      }
      if (key === "common.close") {
        return "Close";
      }
      if (key === "match.roundSummaryUnavailable") {
        return "Round summary unavailable.";
      }
      if (key === "match.roundSummary") {
        return "Round Summary";
      }
      if (key === "home.leagueTable") {
        return "League Table";
      }
      if (key === "home.topScorers") {
        return "Top Scorers";
      }
      if (key === "home.noGoals") {
        return "No goals scored yet.";
      }
      if (key === "common.none") {
        return "None";
      }
      if (params?.team) {
        return `${key}:${params.team}`;
      }
      return key;
    },
  }),
}));

function makeSnapshot() {
  return {
    phase: "FullTime",
    current_minute: 90,
    home_score: 2,
    away_score: 1,
    possession: "Home" as const,
    ball_zone: "Midfield",
    home_team: {
      id: "team1",
      name: "Alpha FC",
      formation: "4-4-2",
      play_style: "Balanced",
      players: [
        {
          id: "p1",
          name: "Alice",
          position: "Forward",
          ovr: 70,
          condition: 90,
          pace: 70,
          stamina: 70,
          strength: 70,
          agility: 70,
          passing: 70,
          shooting: 70,
          tackling: 40,
          dribbling: 70,
          defending: 40,
          positioning: 70,
          vision: 70,
          decisions: 70,
          composure: 70,
          aggression: 50,
          teamwork: 70,
          leadership: 60,
          handling: 20,
          reflexes: 20,
          aerial: 50,
          traits: [],
        },
      ],
    },
    away_team: {
      id: "team2",
      name: "Beta FC",
      formation: "4-4-2",
      play_style: "Balanced",
      players: [
        {
          id: "p2",
          name: "Bob",
          position: "Forward",
          ovr: 70,
          condition: 90,
          pace: 70,
          stamina: 70,
          strength: 70,
          agility: 70,
          passing: 70,
          shooting: 70,
          tackling: 40,
          dribbling: 70,
          defending: 40,
          positioning: 70,
          vision: 70,
          decisions: 70,
          composure: 70,
          aggression: 50,
          teamwork: 70,
          leadership: 60,
          handling: 20,
          reflexes: 20,
          aerial: 50,
          traits: [],
        },
      ],
    },
    home_bench: [],
    away_bench: [],
    home_possession_pct: 52,
    away_possession_pct: 48,
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

function makeGameState() {
  return {
    clock: {
      current_date: "2026-08-01",
      start_date: "2026-08-01",
    },
    manager: {
      id: "mgr1",
      first_name: "Alex",
      last_name: "Manager",
      date_of_birth: "1980-01-01",
      nationality: "GB",
      reputation: 50,
      satisfaction: 50,
      fan_approval: 50,
      team_id: "team1",
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
        training_focus: "Physical",
        training_intensity: "Medium",
        training_schedule: "Balanced",
        founded_year: 1900,
        colors: { primary: "#00ff00", secondary: "#ffffff" },
        starting_xi_ids: [],
        match_roles: {
          captain: null,
          vice_captain: null,
          penalty_taker: null,
          free_kick_taker: null,
          corner_taker: null,
        },
        form: ["W", "W", "D"],
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
        training_focus: "Physical",
        training_intensity: "Medium",
        training_schedule: "Balanced",
        founded_year: 1900,
        colors: { primary: "#0000ff", secondary: "#ffffff" },
        starting_xi_ids: [],
        match_roles: {
          captain: null,
          vice_captain: null,
          penalty_taker: null,
          free_kick_taker: null,
          corner_taker: null,
        },
        form: ["L", "D", "W"],
        history: [],
      },
    ],
    players: [
      {
        id: "p1",
        match_name: "Alice",
        full_name: "Alice Forward",
        date_of_birth: "2000-01-01",
        nationality: "GB",
        position: "Forward",
        natural_position: "Forward",
        alternate_positions: [],
        training_focus: null,
        attributes: {
          pace: 70,
          stamina: 70,
          strength: 70,
          agility: 70,
          passing: 70,
          shooting: 70,
          tackling: 40,
          dribbling: 70,
          defending: 40,
          positioning: 70,
          vision: 70,
          decisions: 70,
          composure: 70,
          aggression: 50,
          teamwork: 70,
          leadership: 60,
          handling: 20,
          reflexes: 20,
          aerial: 50,
        },
        condition: 90,
        morale: 70,
        injury: null,
        team_id: "team1",
        contract_end: null,
        wage: 10000,
        market_value: 1000000,
        stats: {
          appearances: 0,
          goals: 0,
          assists: 0,
          clean_sheets: 0,
          avg_rating: 0,
          minutes_played: 0,
          yellow_cards: 0,
          red_cards: 0,
        },
        form: [],
        personality: null,
        morale_core: {
          base: 70,
          confidence: 70,
          manager_trust: 70,
          happiness: 70,
          pending_promise: null,
          unresolved_issue: null,
          last_playing_time_concern: null,
        },
      },
      {
        id: "p2",
        match_name: "Bob",
        full_name: "Bob Forward",
        date_of_birth: "2000-01-01",
        nationality: "GB",
        position: "Forward",
        natural_position: "Forward",
        alternate_positions: [],
        training_focus: null,
        attributes: {
          pace: 70,
          stamina: 70,
          strength: 70,
          agility: 70,
          passing: 70,
          shooting: 70,
          tackling: 40,
          dribbling: 70,
          defending: 40,
          positioning: 70,
          vision: 70,
          decisions: 70,
          composure: 70,
          aggression: 50,
          teamwork: 70,
          leadership: 60,
          handling: 20,
          reflexes: 20,
          aerial: 50,
        },
        condition: 90,
        morale: 70,
        injury: null,
        team_id: "team2",
        contract_end: null,
        wage: 10000,
        market_value: 1000000,
        stats: {
          appearances: 0,
          goals: 0,
          assists: 0,
          clean_sheets: 0,
          avg_rating: 0,
          minutes_played: 0,
          yellow_cards: 0,
          red_cards: 0,
        },
        form: [],
        personality: null,
        morale_core: {
          base: 70,
          confidence: 70,
          manager_trust: 70,
          happiness: 70,
          pending_promise: null,
          unresolved_issue: null,
          last_playing_time_concern: null,
        },
      },
    ],
    staff: [],
    messages: [],
    news: [],
    league: null,
    scouting_assignments: [],
    board_objectives: [],
  } as unknown as GameStateData;
}

function makeReportedFixture(id: string) {
  return {
    id,
    matchday: 4,
    date: "2026-08-01",
    home_team_id: "team1",
    away_team_id: "team2",
    competition: "League" as const,
    status: "Completed" as const,
    result: {
      home_goals: 2,
      away_goals: 1,
      home_scorers: [{ player_id: "p1", minute: 12 }],
      away_scorers: [{ player_id: "p2", minute: 68 }],
      report: {
        total_minutes: 90,
        home_stats: {
          possession_pct: 54,
          shots: 13,
          shots_on_target: 6,
          fouls: 8,
          corners: 5,
          yellow_cards: 1,
          red_cards: 0,
        },
        away_stats: {
          possession_pct: 46,
          shots: 9,
          shots_on_target: 4,
          fouls: 11,
          corners: 3,
          yellow_cards: 2,
          red_cards: 0,
        },
        events: [
          {
            minute: 12,
            event_type: "Goal",
            side: "Home" as const,
            player_id: "p1",
            secondary_player_id: null,
          },
          {
            minute: 68,
            event_type: "Goal",
            side: "Away" as const,
            player_id: "p2",
            secondary_player_id: null,
          },
        ],
      },
    },
  } as FixtureData;
}

describe("PostMatchScreen", function (): void {
  it("renders the round summary mini table and scorer list when summary data exists", function (): void {
    const gameState = makeGameState();
    gameState.league = {
      id: "league-1",
      name: "League",
      season: 1,
      fixtures: [makeReportedFixture("fx1")],
      standings: [],
    };

    render(
      <ThemeProvider>
        <PostMatchScreen
          snapshot={makeSnapshot()}
          gameState={gameState}
          currentFixture={{
            id: "fixture-1",
            matchday: 4,
            date: "2026-08-01",
            home_team_id: "team1",
            away_team_id: "team2",
            competition: "League",
            status: "Completed",
            result: null,
          }}
          userSide="Home"
          isSpectator={false}
          importantEvents={[]}
          roundSummary={{
            matchday: 4,
            is_complete: true,
            pending_fixture_count: 0,
            completed_results: [
              {
                fixture_id: "fx1",
                home_team_id: "team1",
                home_team_name: "Alpha FC",
                away_team_id: "team2",
                away_team_name: "Beta FC",
                home_goals: 2,
                away_goals: 1,
              },
            ],
            standings_delta: [
              {
                team_id: "team1",
                team_name: "Alpha FC",
                previous_position: 2,
                current_position: 1,
                points: 12,
                points_delta: 3,
              },
              {
                team_id: "team2",
                team_name: "Beta FC",
                previous_position: 1,
                current_position: 2,
                points: 10,
                points_delta: 0,
              },
            ],
            notable_upset: null,
            top_scorer_delta: [
              {
                player_id: "p1",
                player_name: "Alice",
                team_id: "team1",
                previous_rank: 2,
                current_rank: 1,
                previous_goals: 4,
                current_goals: 6,
              },
            ],
          }}
          onPressConference={() => { }}
          onFinish={() => { }}
        />
      </ThemeProvider>,
    );

    expect(screen.getByText("Matchday 4")).toBeInTheDocument();
    expect(screen.getByText("Alpha FC 2 - 1 Beta FC")).toBeInTheDocument();
    expect(screen.getByText("Alice 12' • Bob 68'")).toBeInTheDocument();
    expect(screen.getByText("View details")).toBeInTheDocument();
    expect(screen.getByText("1. Alpha FC")).toBeInTheDocument();
    expect(screen.getByText("1. Alice")).toBeInTheDocument();
  });

  it("opens a read-only detail modal for another completed fixture", function (): void {
    const gameState = makeGameState();
    gameState.league = {
      id: "league-1",
      name: "League",
      season: 1,
      fixtures: [makeReportedFixture("fx1")],
      standings: [],
    };

    render(
      <ThemeProvider>
        <PostMatchScreen
          snapshot={makeSnapshot()}
          gameState={gameState}
          currentFixture={{
            id: "fixture-1",
            matchday: 4,
            date: "2026-08-01",
            home_team_id: "team1",
            away_team_id: "team2",
            competition: "League",
            status: "Completed",
            result: null,
          }}
          userSide="Home"
          isSpectator={false}
          importantEvents={[]}
          roundSummary={{
            matchday: 4,
            is_complete: true,
            pending_fixture_count: 0,
            completed_results: [
              {
                fixture_id: "fx1",
                home_team_id: "team1",
                home_team_name: "Alpha FC",
                away_team_id: "team2",
                away_team_name: "Beta FC",
                home_goals: 2,
                away_goals: 1,
              },
            ],
            standings_delta: [],
            notable_upset: null,
            top_scorer_delta: [],
          }}
          onPressConference={() => { }}
          onFinish={() => { }}
        />
      </ThemeProvider>,
    );

    fireEvent.click(screen.getByText("View details"));

    expect(screen.getByText("Match Details")).toBeInTheDocument();
    expect(
      screen.getAllByText("Alpha FC 2 - 1 Beta FC").length,
    ).toBeGreaterThan(0);
    expect(screen.getAllByText("Alice").length).toBeGreaterThan(0);
    expect(screen.getAllByText("12'").length).toBeGreaterThan(0);
    expect(screen.getByText("Close")).toBeInTheDocument();
  });

  it("renders a friendly empty state when the round summary is null", function (): void {
    render(
      <ThemeProvider>
        <PostMatchScreen
          snapshot={makeSnapshot()}
          gameState={makeGameState()}
          currentFixture={{
            id: "friendly-1",
            matchday: 0,
            date: "2026-07-20",
            home_team_id: "team1",
            away_team_id: "team2",
            competition: "Friendly",
            status: "Completed",
            result: null,
          }}
          userSide="Home"
          isSpectator={false}
          importantEvents={[]}
          roundSummary={null}
          onPressConference={() => { }}
          onFinish={() => { }}
        />
      </ThemeProvider>,
    );

    expect(screen.getByText("Other Matches")).toBeInTheDocument();
    expect(
      screen.getByText("Other match context unavailable for this fixture yet."),
    ).toBeInTheDocument();
  });
});
