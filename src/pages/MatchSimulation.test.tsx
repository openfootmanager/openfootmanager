import { fireEvent, render, screen, waitFor } from "@testing-library/react";
import { beforeEach, describe, expect, it, vi } from "vitest";
import { invoke } from "@tauri-apps/api/core";

import MatchSimulation from "./MatchSimulation";

const navigateMock = vi.fn();
const setGameStateMock = vi.fn();
let locationState: unknown = null;
let gameStoreState: {
  gameState: Record<string, unknown> | null;
  setGameState: typeof setGameStateMock;
};

vi.mock("@tauri-apps/api/core", () => ({
  invoke: vi.fn(),
}));

vi.mock("react-router-dom", () => ({
  useNavigate: () => navigateMock,
  useLocation: () => ({ state: locationState }),
}));

vi.mock("react-i18next", () => ({
  useTranslation: () => ({
    t: (key: string, fallback?: string) => fallback ?? key,
    i18n: { language: "en" },
  }),
}));

vi.mock("../store/gameStore", () => ({
  useGameStore: () => gameStoreState,
}));

vi.mock("../components/match/PreMatchSetup", () => ({
  default: ({ snapshot }: { snapshot: { home_team: { name: string } } }) => (
    <div data-testid="prematch">{snapshot.home_team.name}</div>
  ),
}));

vi.mock("../components/match/MatchLive", () => ({
  default: ({
    snapshot,
    onFullTime,
  }: {
    snapshot: { home_team: { name: string } };
    onFullTime?: () => void;
  }) => (
    <button data-testid="match-live" onClick={onFullTime}>
      {snapshot.home_team.name}
    </button>
  ),
}));

vi.mock("../components/match/HalfTimeBreak", () => ({
  default: () => <div data-testid="halftime" />,
}));

vi.mock("../components/match/PostMatchScreen", () => ({
  default: ({
    onFinish,
    roundSummary,
  }: {
    onFinish?: () => void;
    roundSummary?: unknown;
  }) => (
    <div>
      <div data-testid="postmatch-round-summary">
        {roundSummary ? JSON.stringify(roundSummary) : "null"}
      </div>
      <button data-testid="postmatch-finish" onClick={onFinish}>
        Finish Match
      </button>
    </div>
  ),
}));

vi.mock("../components/match/PressConference", () => ({
  default: () => <div data-testid="press" />,
}));

const mockedInvoke = vi.mocked(invoke);

function makeEnginePlayer(
  overrides: Partial<Record<string, unknown>> = {},
): Record<string, unknown> {
  return {
    id: "p1",
    name: "Player One",
    position: "Goalkeeper",
    condition: 100,
    pace: 50,
    stamina: 50,
    strength: 50,
    agility: 50,
    passing: 50,
    shooting: 50,
    tackling: 50,
    dribbling: 50,
    defending: 50,
    positioning: 50,
    vision: 50,
    decisions: 50,
    composure: 50,
    aggression: 50,
    teamwork: 50,
    leadership: 50,
    handling: 50,
    reflexes: 50,
    aerial: 50,
    traits: [],
    ...overrides,
  };
}

function makeSnapshot(
  overrides: Partial<Record<string, unknown>> = {},
): Record<string, unknown> {
  return {
    phase: "PreKickOff",
    current_minute: 0,
    home_score: 0,
    away_score: 0,
    possession: "Home",
    ball_zone: "Midfield",
    home_team: {
      id: "home1",
      name: "Home FC",
      formation: "4-4-2",
      play_style: "Balanced",
      players: [makeEnginePlayer({ id: "home-p1", name: "Home Keeper" })],
    },
    away_team: {
      id: "away1",
      name: "Away FC",
      formation: "4-4-2",
      play_style: "Balanced",
      players: [makeEnginePlayer({ id: "away-p1", name: "Away Keeper" })],
    },
    home_bench: [],
    away_bench: [],
    home_possession_pct: 50,
    away_possession_pct: 50,
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
    ...overrides,
  };
}

function makeGameState(): Record<string, unknown> {
  return {
    clock: {
      current_date: "2026-08-01",
      start_date: "2026-08-01",
    },
    manager: {
      id: "mgr1",
      first_name: "Test",
      last_name: "Manager",
      date_of_birth: "1980-01-01",
      nationality: "GB",
      reputation: 50,
      satisfaction: 50,
      fan_approval: 50,
      team_id: "home1",
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
        id: "home1",
        name: "Home FC",
        short_name: "HOM",
        country: "England",
        city: "Home City",
        stadium_name: "Home Ground",
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
        id: "away1",
        name: "Away FC",
        short_name: "AWY",
        country: "England",
        city: "Away City",
        stadium_name: "Away Ground",
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

describe("MatchSimulation", function (): void {
  beforeEach(function resetState(): void {
    mockedInvoke.mockReset();
    navigateMock.mockReset();
    setGameStateMock.mockReset();
    locationState = null;
    gameStoreState = {
      gameState: makeGameState(),
      setGameState: setGameStateMock,
    };
  });

  it("renders the current live snapshot when get_match_snapshot succeeds", async function (): Promise<void> {
    mockedInvoke.mockResolvedValueOnce(makeSnapshot());

    render(<MatchSimulation />);

    await waitFor(function (): void {
      expect(mockedInvoke).toHaveBeenCalledWith("get_match_snapshot");
    });

    await waitFor(function (): void {
      expect(screen.getByTestId("prematch")).toHaveTextContent("Home FC");
    });
  });

  it("restores the live match session when no snapshot exists but fixture index is provided", async function (): Promise<void> {
    const consoleWarnSpy = vi
      .spyOn(console, "warn")
      .mockImplementation(() => { });
    try {
      locationState = {
        fixtureIndex: 4,
        mode: "live",
        snapshot: makeSnapshot({
          home_team: {
            id: "home1",
            name: "Boot Snapshot FC",
            formation: "4-4-2",
            play_style: "Balanced",
            players: [makeEnginePlayer({ id: "boot-p1", name: "Boot Keeper" })],
          },
        }),
      };

      mockedInvoke.mockRejectedValueOnce(new Error("No active live match"));
      mockedInvoke.mockResolvedValueOnce(
        makeSnapshot({
          home_team: {
            id: "home1",
            name: "Restored FC",
            formation: "4-4-2",
            play_style: "Balanced",
            players: [
              makeEnginePlayer({ id: "restore-p1", name: "Restore Keeper" }),
            ],
          },
        }),
      );

      render(<MatchSimulation />);

      await waitFor(function (): void {
        expect(mockedInvoke).toHaveBeenCalledWith("start_live_match", {
          allowsExtraTime: false,
          fixtureIndex: 4,
          mode: "live",
        });
      });

      expect(screen.getByTestId("prematch")).toHaveTextContent("Restored FC");
    } finally {
      consoleWarnSpy.mockRestore();
    }
  });

  it("moves spectators straight into the live match stage", async function (): Promise<void> {
    locationState = {
      mode: "spectator",
      snapshot: makeSnapshot(),
    };

    mockedInvoke.mockResolvedValueOnce(makeSnapshot());

    render(<MatchSimulation />);

    await waitFor(function (): void {
      expect(screen.getByTestId("match-live")).toHaveTextContent("Home FC");
    });
  });

  it("navigates away from postmatch after the finalized game has been stored", async function (): Promise<void> {
    locationState = {
      mode: "spectator",
      snapshot: makeSnapshot(),
    };

    const finishedGame = makeGameState();
    mockedInvoke.mockResolvedValueOnce(makeSnapshot()).mockResolvedValueOnce({
      game: finishedGame,
      round_summary: {
        matchday: 1,
        is_complete: true,
        pending_fixture_count: 0,
        completed_results: [],
        standings_delta: [],
        notable_upset: null,
        top_scorer_delta: [],
      },
    });

    render(<MatchSimulation />);

    await waitFor(function (): void {
      expect(screen.getByTestId("match-live")).toHaveTextContent("Home FC");
    });

    fireEvent.click(screen.getByTestId("match-live"));

    await waitFor(function (): void {
      expect(mockedInvoke).toHaveBeenLastCalledWith("finish_live_match");
      expect(screen.getByTestId("postmatch-finish")).toBeInTheDocument();
    });

    expect(setGameStateMock).toHaveBeenCalledWith(finishedGame);

    fireEvent.click(screen.getByTestId("postmatch-finish"));

    await waitFor(function (): void {
      expect(navigateMock).toHaveBeenCalledWith("/dashboard");
    });
  });

  it("finalizes the match on full time and passes the round summary into postmatch", async function (): Promise<void> {
    locationState = {
      mode: "spectator",
      snapshot: makeSnapshot(),
    };

    const finishedGame = makeGameState();
    const roundSummary = {
      matchday: 1,
      is_complete: true,
      pending_fixture_count: 0,
      completed_results: [],
      standings_delta: [],
      notable_upset: null,
      top_scorer_delta: [],
    };
    mockedInvoke.mockResolvedValueOnce(makeSnapshot()).mockResolvedValueOnce({
      game: finishedGame,
      round_summary: roundSummary,
    });

    render(<MatchSimulation />);

    await waitFor(function (): void {
      expect(screen.getByTestId("match-live")).toHaveTextContent("Home FC");
    });

    fireEvent.click(screen.getByTestId("match-live"));

    await waitFor(function (): void {
      expect(mockedInvoke).toHaveBeenLastCalledWith("finish_live_match");
      expect(screen.getByTestId("postmatch-finish")).toBeInTheDocument();
    });

    expect(setGameStateMock).toHaveBeenCalledWith(finishedGame);

    expect(screen.getByTestId("postmatch-round-summary")).toHaveTextContent(
      '"matchday":1',
    );

    fireEvent.click(screen.getByTestId("postmatch-finish"));

    await waitFor(function (): void {
      expect(navigateMock).toHaveBeenCalledWith("/dashboard");
    });
  });
});
