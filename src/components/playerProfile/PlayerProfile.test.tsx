import { fireEvent, render, screen, waitFor } from "@testing-library/react";
import { beforeAll, beforeEach, describe, expect, it, vi } from "vitest";
import { invoke } from "@tauri-apps/api/core";
import type { StaffData } from "../../store/gameStore";
import PlayerProfile from "./PlayerProfile";
import { i18nReady } from "../../i18n";
import {
  renderOnTab,
  hasWeeklyWage,
  createTeam,
  createPlayer,
  createStaff,
  createGameState,
  createAdvancedStatsSummary,
  defaultInvokeResponse,
} from "./PlayerProfile.harness";
import { riskLevels, knowledge } from "./PlayerProfile.mocks";

/**
 * The player profile's shell: the header, the tabs, the advanced stats, the recent matches, the career, and switching from one player to another.
 * Split from the single `PlayerProfile.test.tsx` (#1406): every test keeps its name and its
 * Given/When/Then; the mocks and fixtures are in `PlayerProfile.mocks.ts` and `PlayerProfile.harness.tsx`.
 */

vi.mock("../../hooks/useContractWarningStages", async () => {
  const { contractWarningStagesMock } = await import("./PlayerProfile.mocks");
  return contractWarningStagesMock;
});
vi.mock("../../hooks/usePlayerKnowledge", async () => {
  const { playerKnowledgeMock } = await import("./PlayerProfile.mocks");
  return playerKnowledgeMock;
});
vi.mock("../../hooks/useVisibleWage", async () => {
  const { visibleWageMock } = await import("./PlayerProfile.mocks");
  return visibleWageMock;
});
vi.mock("../../hooks/usePotentialOpinion", async () => {
  const { potentialOpinionMock } = await import("./PlayerProfile.mocks");
  return potentialOpinionMock;
});
vi.mock("../../hooks/usePositionRatings", async () => {
  const { positionRatingsMock } = await import("./PlayerProfile.mocks");
  return positionRatingsMock;
});
vi.mock("@tauri-apps/api/core", () => ({ invoke: vi.fn() }));
vi.mock("react-i18next", async () => {
  const { reactI18nextMock } = await import("./PlayerProfile.mocks");
  return reactI18nextMock;
});
vi.mock("../../utils/backendI18n", async (importOriginal) => {
  const { backendI18nOverrides } = await import("./PlayerProfile.mocks");
  return {
    ...(await importOriginal<typeof import("../../utils/backendI18n")>()),
    ...backendI18nOverrides,
  };
});
vi.mock("../../lib/countries", async () => {
  const { countriesMock } = await import("./PlayerProfile.mocks");
  return countriesMock;
});

beforeAll(async () => {
  await i18nReady;
});

describe("PlayerProfile contract surfaces", () => {
  beforeEach(() => {
    riskLevels.current = {};
    knowledge.current = {};
    vi.mocked(invoke).mockReset();
    vi.mocked(invoke).mockImplementation(async (command: string) => defaultInvokeResponse(command));
  });

  /**
   * Given a player born on 10 March 2000 and a game date of 1 August 2030
   * When the profile opens
   * Then it shows age 30
   */
  it("shows the age on the game's current date", () => {
    const player = createPlayer({ date_of_birth: "2000-03-10" });
    const state = createGameState(player);
    state.clock = { ...state.clock, current_date: "2030-08-01T00:00:00Z" };

    render(<PlayerProfile player={player} gameState={state} isOwnClub />);

    expect(screen.getByText("Age 30")).toBeInTheDocument();
  });

  /**
   * Given a player's profile, shown under the dashboard header that already has a Back
   * When it renders
   * Then it adds no second Back of its own
   */
  it("leaves Back to the dashboard header", () => {
    const player = createPlayer();

    render(<PlayerProfile player={player} gameState={createGameState(player)} isOwnClub />);

    expect(screen.queryByRole("button", { name: "Back" })).not.toBeInTheDocument();
  });

  /**
   * Given the same player the day before and the day after his birthday in 2031
   * When the profile opens on each date
   * Then it shows 30, then 31
   */
  it.each([
    { date: "2031-03-09T00:00:00Z", age: 30 },
    { date: "2031-03-10T00:00:00Z", age: 31 },
  ])("changes the age when the birthday passes ($date)", ({ date, age }) => {
    const player = createPlayer({ date_of_birth: "2000-03-10" });
    const state = createGameState(player);
    state.clock = { ...state.clock, current_date: date };

    render(<PlayerProfile player={player} gameState={state} isOwnClub />);

    expect(screen.getByText(`Age ${age}`)).toBeInTheDocument();
  });

  it("puts the date, risk and wage in Details and the years left on the Contract tab", () => {
    const player = createPlayer();
    riskLevels.current = { [player.id]: "critical" };
    const gameState = createGameState(player);

    render(<PlayerProfile player={player} gameState={gameState} isOwnClub />);

    // The Contract tab holds the years left and the actions; the date and its risk are in Details.
    expect(screen.getByText("playerProfile.contractSheet.his")).toBeInTheDocument();
    expect(screen.getByText(/until .*15 October 2026/)).toBeInTheDocument();
    expect(screen.getByText("playerProfile.contractSheet.timeLeft")).toBeInTheDocument();
    expect(screen.queryByText("Contract Risk")).toBeNull();
    expect(screen.getAllByText("Critical").length).toBeGreaterThan(0);
    expect(screen.queryByText("Weekly Wage")).toBeNull();
    expect(screen.getByText("Wage")).toBeInTheDocument();
    expect(
      screen.getAllByText((_, element) => hasWeeklyWage(element?.textContent ?? "", "12,000"))
        .length,
    ).toBeGreaterThan(0);
  });

  it("hides contract actions for loaned-in players owned by another club", () => {
    const player = createPlayer({
      team_id: "team-1",
      active_loan: {
        parent_team_id: "team-2",
        loan_team_id: "team-1",
        start_date: "2026-08-01",
        end_date: "2027-01-01",
        wage_contribution_pct: 75,
        buy_option_fee: null,
        loan_start_minutes: 0,
        loan_start_appearances: 0,
        development_reported_minutes: 0,
        development_reported_appearances: 0,
      },
    });

    const gameState = createGameState(player);
    gameState.teams.push(
      createTeam({
        id: "team-2",
        name: "Beta FC",
        short_name: "BET",
        manager_id: null,
      }),
    );

    render(
      <PlayerProfile player={player} gameState={gameState} isOwnClub onGameUpdate={vi.fn()} />,
    );

    expect(screen.getByText("Loan Status")).toBeInTheDocument();
    expect(screen.getByText("On loan from Beta FC")).toBeInTheDocument();
    expect(screen.queryByRole("button", { name: "Renew Contract" })).not.toBeInTheDocument();
    expect(screen.queryByRole("button", { name: "Let Expire" })).not.toBeInTheDocument();
    expect(screen.queryByRole("button", { name: "Terminate Now" })).not.toBeInTheDocument();
  });

  it("shows contract actions for players loaned out by the manager club", () => {
    const player = createPlayer({
      team_id: "team-2",
      active_loan: {
        parent_team_id: "team-1",
        loan_team_id: "team-2",
        start_date: "2026-08-01",
        end_date: "2027-01-01",
        wage_contribution_pct: 75,
        buy_option_fee: null,
        loan_start_minutes: 0,
        loan_start_appearances: 0,
        development_reported_minutes: 0,
        development_reported_appearances: 0,
      },
    });

    const gameState = createGameState(player);
    gameState.teams.push(
      createTeam({
        id: "team-2",
        name: "Beta FC",
        short_name: "BET",
        manager_id: null,
      }),
    );

    renderOnTab(
      "Contract tab",
      <PlayerProfile
        player={player}
        gameState={gameState}
        isOwnClub={false}
        onGameUpdate={vi.fn()}
      />,
    );

    expect(screen.getByText("Loan Status")).toBeInTheDocument();
    expect(screen.getByText("On loan at Beta FC")).toBeInTheDocument();
    expect(screen.getByRole("button", { name: "Renew Contract" })).toBeInTheDocument();
    expect(screen.getByRole("button", { name: "Let Expire" })).toBeInTheDocument();
    expect(screen.getByRole("button", { name: "Terminate Now" })).toBeInTheDocument();
    expect(screen.queryByText("Hire a scout first")).not.toBeInTheDocument();
    expect(screen.queryAllByText("playerProfile.attributesHidden")).toHaveLength(0);
    expect(screen.queryAllByText("??")).toHaveLength(0);
  });

  it("allows selecting the player's team from the hero header", () => {
    const player = createPlayer();
    const gameState = createGameState(player);
    const onSelectTeam = vi.fn();

    render(
      <PlayerProfile player={player} gameState={gameState} isOwnClub onSelectTeam={onSelectTeam} />,
    );

    fireEvent.click(screen.getByRole("button", { name: "Alpha FC" }));

    expect(onSelectTeam).toHaveBeenCalledWith("team-1");
  });

  it("offers a context menu action to open the player's team", () => {
    const player = createPlayer();
    const gameState = createGameState(player);
    const onSelectTeam = vi.fn();

    render(
      <PlayerProfile player={player} gameState={gameState} isOwnClub onSelectTeam={onSelectTeam} />,
    );

    fireEvent.contextMenu(screen.getByTestId("player-profile-team-link"));
    fireEvent.click(screen.getByRole("menuitem", { name: "View team" }));

    expect(onSelectTeam).toHaveBeenCalledWith("team-1");
  });

  it("loads advanced stats from the backend overview query", async () => {
    const player = createPlayer({
      stats: {
        appearances: 10,
        goals: 4,
        assists: 3,
        clean_sheets: 0,
        yellow_cards: 1,
        red_cards: 0,
        avg_rating: 7.2,
        minutes_played: 450,
        shots: 20,
        shots_on_target: 10,
        passes_completed: 80,
        passes_attempted: 100,
        tackles_won: 9,
        interceptions: 6,
        fouls_committed: 5,
      },
    });
    const peerA = createPlayer({
      id: "player-2",
      match_name: "A. Peer",
      full_name: "Alpha Peer",
      stats: {
        ...player.stats,
        shots: 10,
        shots_on_target: 5,
        passes_completed: 70,
        passes_attempted: 100,
        tackles_won: 6,
        interceptions: 4,
        fouls_committed: 3,
      },
    });
    const peerB = createPlayer({
      id: "player-3",
      match_name: "B. Peer",
      full_name: "Bravo Peer",
      stats: {
        ...player.stats,
        shots: 15,
        shots_on_target: 8,
        passes_completed: 75,
        passes_attempted: 100,
        tackles_won: 7,
        interceptions: 5,
        fouls_committed: 4,
      },
    });
    const gameState = {
      ...createGameState(player),
      players: [player, peerA, peerB],
    };

    vi.mocked(invoke).mockImplementation(async (command: string) => {
      if (command === "get_player_stats_overview") {
        return {
          percentileEligible: true,
          metrics: {
            shots: { total: 33, per90: 6.6, percentile: 88 },
            shotsOnTarget: { total: 14, per90: 2.8, percentile: 81 },
            passes: {
              completed: 144,
              attempted: 180,
              accuracy: 80,
              percentile: 77,
            },
            tacklesWon: { total: 15, per90: 3, percentile: 72 },
            interceptions: { total: 11, per90: 2.2, percentile: 70 },
            foulsCommitted: { total: 8, per90: 1.6, percentile: 41 },
          },
        };
      }

      if (command === "get_player_match_history") {
        return [];
      }

      return defaultInvokeResponse(command);
    });

    render(<PlayerProfile player={player} gameState={gameState} isOwnClub />);

    await waitFor(() => {
      expect(invoke).toHaveBeenCalledWith("get_player_stats_overview", {
        playerId: "player-1",
      });
      expect(screen.getByText("Advanced Stats")).toBeInTheDocument();
      expect(screen.getByText("Shots")).toBeInTheDocument();
      expect(screen.getAllByText("33").length).toBeGreaterThan(0);
      expect(screen.getAllByText("80% accurate").length).toBeGreaterThan(0);
      expect(screen.getByText("88th")).toBeInTheDocument();
    });
  });

  /**
   * Given the backend overview flags the season rating as available or not
   * When the profile opens for a player with a stored 7.2 average
   * Then the season card shows 7.2 only when the flag says so
   */
  it.each([
    { seasonRatingRated: true, shown: "7.2" },
    { seasonRatingRated: false, shown: "–" },
  ])(
    "shows the season average only when the backend flags it ($seasonRatingRated)",
    async ({ seasonRatingRated, shown }) => {
      const player = createPlayer({
        stats: { ...createPlayer().stats, avg_rating: 7.2, minutes_played: 450 },
      });
      vi.mocked(invoke).mockImplementation(async (command: string) => {
        if (command === "get_player_stats_overview") {
          return {
            ...createAdvancedStatsSummary(),
            percentileEligible: true,
            seasonRatingRated,
          };
        }
        if (command === "get_player_match_history") {
          return [];
        }
        return defaultInvokeResponse(command);
      });

      render(<PlayerProfile player={player} gameState={createGameState(player)} isOwnClub />);

      await waitFor(() => {
        expect(screen.getByText("playerProfile.avgRating").nextElementSibling).toHaveTextContent(
          shown,
        );
      });
    },
  );

  it("loads and renders recent player match history", async () => {
    const player = createPlayer({
      stats: {
        appearances: 10,
        goals: 4,
        assists: 3,
        clean_sheets: 0,
        yellow_cards: 1,
        red_cards: 0,
        avg_rating: 7.2,
        minutes_played: 450,
        shots: 20,
        shots_on_target: 10,
        passes_completed: 80,
        passes_attempted: 100,
        tackles_won: 9,
        interceptions: 6,
        fouls_committed: 5,
      },
    });
    vi.mocked(invoke).mockImplementation(async (command: string) => {
      if (command === "get_player_match_history") {
        return [
          {
            fixture_id: "fixture-1",
            date: "2025-06-17",
            competition: "League",
            matchday: 2,
            opponent_team_id: "team-3",
            opponent_name: "Bravo FC",
            team_goals: 3,
            opponent_goals: 0,
            minutes_played: 88,
            goals: 2,
            assists: 1,
            shots: 5,
            shots_on_target: 3,
            rating: 8.4,
            rated: true,
          },
        ];
      }

      return defaultInvokeResponse(command);
    });

    render(<PlayerProfile player={player} gameState={createGameState(player)} isOwnClub />);

    await waitFor(() => {
      expect(invoke).toHaveBeenCalledWith("get_player_match_history", {
        playerId: "player-1",
        limit: 5,
      });
      expect(screen.getByText("Previous matches")).toBeInTheDocument();
      expect(screen.getByText(/Bravo FC/)).toBeInTheDocument();
      expect(screen.getByText("3–0")).toBeInTheDocument();
      expect(screen.getByText("8.4")).toBeInTheDocument();
    });
  });

  it("draws the career the backend projects on the Career tab, its moves as lines", async () => {
    const player = createPlayer();
    vi.mocked(invoke).mockImplementation(async (command: string) => {
      if (command === "get_player_career") {
        return {
          by_season: [
            {
              kind: "season",
              key: "2026-team-1",
              season: 2026,
              club_id: "team-1",
              club_name: "Alpha FC",
              appearances: 12,
              goals: 3,
              assists: 2,
              in_progress: true,
              on_loan: false,
            },
            {
              kind: "move",
              key: "move-1",
              date: "2026-08-01",
              movement: "permanent_transfer",
              from_club_name: "Beta FC",
              to_club_name: "Alpha FC",
              fee: 1_250_000,
              loan_end: null,
            },
            {
              kind: "season",
              key: "2025-team-2",
              season: 2025,
              club_id: "team-2",
              club_name: "Beta FC",
              appearances: 30,
              goals: 9,
              assists: 4,
              in_progress: false,
              on_loan: false,
            },
          ],
          by_club: [],
          totals: { appearances: 42, goals: 12, assists: 6 },
        };
      }
      return defaultInvokeResponse(command);
    });

    renderOnTab(
      "Career tab",
      <PlayerProfile player={player} gameState={createGameState(player)} isOwnClub />,
    );

    // The career arrives after the tab opens: the move is a line of the table, with its fee (the
    // file's translator answers a key it does not know with the key).
    expect(await screen.findByText(/playerProfile\.careerMove\.transferFrom/)).toHaveTextContent(
      "€1,250,000",
    );
    expect(screen.getByText("2025/26")).toBeInTheDocument();
  });
});

// `DashboardWorkspaceContent` renders `<PlayerProfile />` without a `key`, so
// switching player reuses the component instance and every piece of hook state
// survives. Anything scoped to one player has to reset on `player.id` itself.
describe("PlayerProfile switching between players", () => {
  beforeEach(() => {
    vi.mocked(invoke).mockReset();
    vi.mocked(invoke).mockImplementation(async (command: string) => defaultInvokeResponse(command));
  });

  function scoutStaff(): StaffData {
    return createStaff({ id: "scout-1", role: "Scout" as StaffData["role"] });
  }

  // Scouting is offered for players the manager does *not* have, so both of
  // these sit at another club.
  it("re-enables scouting for the next player after one has been scouted", async () => {
    const first = createPlayer({ team_id: "team-2" });
    const gameState = createGameState(first, [scoutStaff()]);

    const { rerender } = render(
      <PlayerProfile
        player={first}
        gameState={gameState}
        isOwnClub={false}
        onGameUpdate={vi.fn()}
      />,
    );

    fireEvent.click(screen.getByRole("button", { name: "Scout" }));
    await waitFor(() => {
      expect(invoke).toHaveBeenCalledWith("send_scout", {
        careerToken: 0,
        scoutId: "scout-1",
        playerId: "player-1",
      });
    });

    const second = createPlayer({
      id: "player-2",
      full_name: "Sam Other",
      team_id: "team-2",
    });
    rerender(
      <PlayerProfile
        player={second}
        gameState={createGameState(second, [scoutStaff()])}
        isOwnClub={false}
        onGameUpdate={vi.fn()}
      />,
    );

    // The scout was sent to look at player-1, so player-2 is not being
    // scouted and must still be offered.
    await waitFor(() => {
      expect(screen.getByRole("button", { name: "Scout" })).toBeEnabled();
    });
    expect(screen.queryByText("Scouting in progress")).toBeNull();
  });

  it("drops the previous player's recent matches on arrival", async () => {
    const first = createPlayer({ stats: { ...createPlayer().stats, appearances: 3 } });

    vi.mocked(invoke).mockImplementation(async (command: string, args?: unknown) => {
      if (command === "get_player_match_history") {
        const playerId = (args as { playerId: string }).playerId;
        if (playerId === "player-1") {
          return [
            {
              fixture_id: "fixture-1",
              opponent_name: "Rivals FC",
              date: "2026-07-20",
              is_home: true,
              goals: 1,
              assists: 0,
              rating: 7.5,
              rated: true,
              minutes_played: 90,
            },
          ];
        }
        return [];
      }

      return defaultInvokeResponse(command);
    });

    const { rerender } = render(
      <PlayerProfile
        player={first}
        gameState={createGameState(first)}
        isOwnClub
        onGameUpdate={vi.fn()}
      />,
    );

    await waitFor(() => {
      expect(screen.getByText(/Rivals FC/)).toBeInTheDocument();
    });

    const second = createPlayer({
      id: "player-2",
      full_name: "Sam Other",
      stats: { ...createPlayer().stats, appearances: 3 },
    });
    rerender(
      <PlayerProfile
        player={second}
        gameState={createGameState(second)}
        isOwnClub
        onGameUpdate={vi.fn()}
      />,
    );

    await waitFor(() => {
      expect(screen.queryByText(/Rivals FC/)).toBeNull();
    });
  });
});
