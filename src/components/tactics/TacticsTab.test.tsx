import { act, cleanup, fireEvent, render, screen, waitFor, within } from "@testing-library/react";
import { beforeEach, describe, expect, it, vi } from "vitest";
import { invoke } from "@tauri-apps/api/core";
import type { GameStateData, PlayerData } from "../../store/gameStore";
import TacticsTab from "./TacticsTab";
import { resetTacticsLineupCache } from "../../hooks/useTacticsLineupProjection";
import { ratingTierClass } from "../../lib/ratingTier";
import {
  familiarityState,
  focusableIn,
  makeGameState,
  makePlayer,
  makeTeam,
  swapBenchDefenderIn,
} from "../../test-utils/tacticsTabFixtures";

// Whether the backend's position table has arrived; a test says it has not.
const positionTableState = vi.hoisted(() => ({ arrived: true }));

vi.mock("../../hooks/useFamiliarity", async () => {
  const { cannedFamiliarityModule } = await import("../../test-utils/familiarity");
  const { familiarityState } = await import("../../test-utils/tacticsTabFixtures");
  return cannedFamiliarityModule(
    (playerId) => familiarityState.players.get(playerId),
    () => familiarityState.arrived,
  );
});

vi.mock("../../hooks/usePositionGroups", async () => {
  const { cannedPositionsModule } = await import("../../test-utils/positionTable");
  const { createPositionTable } = await import("../../lib/positionTable");
  const canned = cannedPositionsModule();
  const notArrived = createPositionTable([]);
  return {
    ...canned,
    usePositionTable: () => {
      const arrived = canned.usePositionTable();
      return positionTableState.arrived ? arrived : notArrived;
    },
  };
});

vi.mock("react-i18next", () => ({
  // The error banner pulls in utils/backendI18n -> i18n/index, which registers this plugin.
  initReactI18next: { type: "3rdParty", init: () => undefined },
  useTranslation: () => ({
    t: (key: string, fallback?: string | Record<string, unknown>) => {
      if (key === "playerProfile.daysRemaining") {
        return `${String((fallback as Record<string, unknown> | undefined)?.count ?? "")} days remaining`;
      }
      if (key === "playerProfile.injuryDaysShort") {
        return `${String((fallback as Record<string, unknown> | undefined)?.count ?? "")}d`;
      }
      if (key.startsWith("common.injuries.")) {
        return String((fallback as Record<string, unknown> | undefined)?.defaultValue ?? key);
      }
      return typeof fallback === "string" ? fallback : key;
    },
    i18n: { language: "en" },
  }),
}));

vi.mock("../../hooks/useRatingTiers", async () => {
  const { cannedRatingTiersModule } = await import("../../test-utils/ratingTiers");
  return cannedRatingTiersModule();
});

vi.mock("@tauri-apps/api/core", () => ({
  invoke: vi.fn(),
}));

const mockedInvoke = vi.mocked(invoke);

describe("TacticsTab", () => {
  beforeEach(() => {
    localStorage.clear();
    resetTacticsLineupCache();
    mockedInvoke.mockReset();
    const defaultGameState = makeGameState();
    const defaultRoster = defaultGameState.players.filter((p) => p.team_id === "team1");
    mockedInvoke.mockImplementation(async (command: string) => {
      if (command === "get_squad") return defaultRoster;
      return defaultGameState;
    });
  });

  it("renders the top tactical controls plus bench player in the left panel", () => {
    render(
      <TacticsTab gameState={makeGameState()} onSelectPlayer={vi.fn()} onGameUpdate={vi.fn()} />,
    );

    expect(screen.getByRole("group", { name: "tactics.tacticBar" })).toBeInTheDocument();
    expect(screen.getByText("tactics.formation")).toBeInTheDocument();
    expect(screen.getByText("tactics.playStyle")).toBeInTheDocument();
    expect(screen.getAllByText(/preMatch\.substitutes/).length).toBeGreaterThan(0);
    expect(screen.getByTestId("bench-player-d5")).toBeInTheDocument();
    expect(screen.getByTestId("pitch-bench-player-d5")).toBeInTheDocument();
  });

  it("shows the compact tactics toolbar across the top of the lineup workspace", () => {
    render(
      <TacticsTab gameState={makeGameState()} onSelectPlayer={vi.fn()} onGameUpdate={vi.fn()} />,
    );

    expect(screen.getByRole("group", { name: "tactics.tacticBar" })).toBeInTheDocument();
    expect(screen.getByRole("button", { name: "tactics.chooseTactic" })).toBeInTheDocument();
    expect(screen.getByRole("combobox", { name: "tactics.formation" })).toBeInTheDocument();
    expect(screen.getByRole("combobox", { name: "tactics.playStyle" })).toBeInTheDocument();
  });

  it("falls back to a custom current setup when no preset matches the active tactic", () => {
    const gameState = makeGameState();
    gameState.teams = [
      makeTeam({
        formation: "4-4-2",
        play_style: "Counter",
        starting_xi_ids: ["gk1", "d1", "d2", "d3", "d4", "m1", "m2", "m3", "m4", "f1", "f2"],
      }),
    ];

    render(<TacticsTab gameState={gameState} onSelectPlayer={vi.fn()} onGameUpdate={vi.fn()} />);

    expect(screen.getByRole("button", { name: "tactics.chooseTactic" })).toHaveTextContent(
      "tactics.customTactic",
    );
    expect(screen.getByRole("button", { name: "tactics.chooseTactic" })).not.toHaveTextContent(
      "balanced-control",
    );
  });

  it("applies a preset by asking the backend for the whole preset", async () => {
    render(
      <TacticsTab gameState={makeGameState()} onSelectPlayer={vi.fn()} onGameUpdate={vi.fn()} />,
    );

    fireEvent.click(screen.getByRole("button", { name: "tactics.chooseTactic" }));
    fireEvent.click(screen.getByRole("option", { name: /high-press/i }));

    await waitFor(() => {
      expect(mockedInvoke).toHaveBeenCalledWith("apply_tactic_preset", {
        careerToken: 0,
        formation: "3-4-3",
        keepInstructions: true,
        playStyle: "HighPress",
      });
    });
  });

  it("shows injured bench players with injury details in the left panel", () => {
    const gameState = makeGameState();
    const injuredBenchPlayer = gameState.players.find((player) => player.id === "d5");
    if (injuredBenchPlayer) {
      injuredBenchPlayer.injury = {
        name: "Ankle sprain",
        days_remaining: 6,
      };
    }

    render(<TacticsTab gameState={gameState} onSelectPlayer={vi.fn()} onGameUpdate={vi.fn()} />);

    expect(screen.getByText("Ankle sprain")).toBeInTheDocument();
    expect(screen.getByText("6d")).toBeInTheDocument();
  });

  // Given a backend-called-up youth, when Tactics opens, then he is selectable.
  it("shows called-up youth in tactics selection", async () => {
    const state = makeGameState();
    state.players = [
      makePlayer("gk1", "Goalkeeper"),
      makePlayer("d5", "Defender", { match_name: "Bench DEF" }),
      makePlayer("called", "Forward", {
        full_name: "Called Youth",
        match_name: "Called Youth",
        squad_role: "Youth",
        match_day_eligible: true,
      }),
    ];
    mockedInvoke.mockImplementation(async (command: string) =>
      command === "get_squad" ? state.players : state,
    );
    render(<TacticsTab gameState={state} onSelectPlayer={vi.fn()} onGameUpdate={vi.fn()} />);
    const [calledUp] = await screen.findAllByRole("button", { name: /Called Youth/i });
    fireEvent.click(calledUp);
    fireEvent.click(screen.getAllByRole("button", { name: /Bench DEF/ })[0]);
    expect(screen.getByText("tactics.selectedPlayer")).toBeInTheDocument();
    expect(screen.getAllByText("Called Youth", { selector: "p" }).length).toBeGreaterThan(0);
  });
  it("keeps youth academy players out of first-team tactics selection", async () => {
    const gameState = makeGameState();
    gameState.players.push(
      makePlayer("y1", "Forward", {
        full_name: "Academy Prospect",
        squad_role: "Youth",
      }),
    );
    // Override so get_squad returns the full roster including the youth player,
    // exercising the client-side isSeniorSquadPlayer filter.
    mockedInvoke.mockImplementation(async (command: string) => {
      if (command === "get_squad") return gameState.players.filter((p) => p.team_id === "team1");
      return gameState;
    });

    render(<TacticsTab gameState={gameState} onSelectPlayer={vi.fn()} onGameUpdate={vi.fn()} />);

    await waitFor(() => {
      expect(screen.queryByText("Academy Prospect")).not.toBeInTheDocument();
    });
  });

  /**
   * Given the position table has not arrived
   * When a bench defender is swapped in for a centre back of the pitch
   * Then nothing is saved: a lineup worked out without the lines must not be written.
   */
  /**
   * Given the position table has not arrived, and then has
   * When the tactics screen is drawn
   * Then it says why lineup changes are not available yet, and says nothing once it has loaded.
   */
  it("says lineup changes wait for the position table, only until it has loaded", async () => {
    positionTableState.arrived = false;
    try {
      const { unmount } = render(
        <TacticsTab gameState={makeGameState()} onSelectPlayer={vi.fn()} onGameUpdate={vi.fn()} />,
      );
      expect(screen.getByRole("status")).toHaveTextContent("common.lineupLoading");
      unmount();
    } finally {
      positionTableState.arrived = true;
    }

    render(
      <TacticsTab gameState={makeGameState()} onSelectPlayer={vi.fn()} onGameUpdate={vi.fn()} />,
    );
    await act(async () => {}); // and the squad has answered for this revision
    expect(screen.queryByText("common.lineupLoading")).toBeNull();
  });

  it("saves no lineup change before the position table has loaded", async () => {
    positionTableState.arrived = false;
    try {
      render(
        <TacticsTab gameState={makeGameState()} onSelectPlayer={vi.fn()} onGameUpdate={vi.fn()} />,
      );

      swapBenchDefenderIn();
      await act(async () => {});

      expect(mockedInvoke).not.toHaveBeenCalledWith("set_starting_xi", expect.anything());
    } finally {
      positionTableState.arrived = true;
    }
  });

  /**
   * Given the position table has arrived but the backend's familiarity has not
   * When the tactics screen is drawn and a bench defender is swapped in for a centre back
   * Then it says lineup changes wait, and nothing is saved: a lineup worked out without the
   *      backend's grade must not be written.
   */
  it("saves no lineup change before the familiarity has loaded", async () => {
    familiarityState.arrived = false;
    try {
      render(
        <TacticsTab gameState={makeGameState()} onSelectPlayer={vi.fn()} onGameUpdate={vi.fn()} />,
      );
      expect(screen.getByRole("status")).toHaveTextContent("common.lineupLoading");

      swapBenchDefenderIn();
      await act(async () => {});

      expect(mockedInvoke).not.toHaveBeenCalledWith("set_starting_xi", expect.anything());
    } finally {
      familiarityState.arrived = true;
    }
  });

  /**
   * Given get_squad has not answered for this revision
   * When the tactics screen is drawn
   * Then it says why lineup changes are not available yet, and says nothing once the squad has
   *      answered.
   */
  it("says lineup changes wait while the squad has not answered", async () => {
    let answer: (players: PlayerData[]) => void = () => {};
    const state = makeGameState();
    mockedInvoke.mockReset();
    mockedInvoke.mockReturnValue(new Promise((resolve) => (answer = resolve)));

    render(<TacticsTab gameState={state} onSelectPlayer={vi.fn()} onGameUpdate={vi.fn()} />);
    expect(screen.getByRole("status")).toHaveTextContent("common.lineupLoading");

    await act(async () => answer(state.players.filter((player) => player.team_id === "team1")));
    expect(screen.queryByText("common.lineupLoading")).toBeNull();
  });

  /**
   * Given the squad answered once, and then the game moved on (a new revision) with its fetch
   *       still in flight
   * When a bench defender is swapped in for a centre back
   * Then nothing is saved: the squad's eligibility is the previous revision's until it answers.
   */
  it("saves no lineup change while a newer get_squad is in flight", async () => {
    const state = makeGameState();
    const roster = state.players.filter((player) => player.team_id === "team1");
    mockedInvoke.mockReset();
    mockedInvoke.mockResolvedValueOnce(roster).mockReturnValue(new Promise(() => {}));

    const { rerender } = render(
      <TacticsTab gameState={state} onSelectPlayer={vi.fn()} onGameUpdate={vi.fn()} />,
    );
    await act(async () => {});
    rerender(
      <TacticsTab gameState={{ ...state }} onSelectPlayer={vi.fn()} onGameUpdate={vi.fn()} />,
    );

    swapBenchDefenderIn();
    await act(async () => {});

    expect(mockedInvoke).not.toHaveBeenCalledWith("set_starting_xi", expect.anything());
  });

  /**
   * Given the position table, the familiarity and the squad have all arrived, but
   *       get_tactics_lineup has not answered for this revision
   * When a bench defender is swapped in for a centre back
   * Then it says lineup changes wait, and nothing is saved: the eleven on screen is the saved one
   *      standing in, not the side the match would field, so a change worked out from it is stale.
   */
  it("saves no lineup change while the backend's lineup has not answered", async () => {
    const state = makeGameState();
    const roster = state.players.filter((player) => player.team_id === "team1");
    mockedInvoke.mockReset();
    mockedInvoke.mockImplementation(async (command: string) => {
      if (command === "get_squad") return roster;
      if (command === "get_tactics_lineup") return new Promise(() => {});
      return state;
    });

    render(<TacticsTab gameState={state} onSelectPlayer={vi.fn()} onGameUpdate={vi.fn()} />);
    await act(async () => {}); // the squad has answered

    swapBenchDefenderIn();
    await act(async () => {});

    expect(mockedInvoke).not.toHaveBeenCalledWith("set_starting_xi", expect.anything());
    expect(screen.getByRole("status")).toHaveTextContent("common.lineupLoading");
  });

  it("sends the correct starting xi order when a bench defender is swapped in for a centre back through the table", async () => {
    render(
      <TacticsTab gameState={makeGameState()} onSelectPlayer={vi.fn()} onGameUpdate={vi.fn()} />,
    );
    await act(async () => {}); // the squad has answered for this revision

    swapBenchDefenderIn();

    await waitFor(() => {
      expect(mockedInvoke).toHaveBeenCalledWith("set_starting_xi", {
        careerToken: 0,
        playerIds: ["gk1", "d5", "d2", "d3", "d4", "m1", "m2", "m3", "m4", "f1", "f2"],
      });
    });
  });

  it("localizes the selected player position in the comparison panel", () => {
    render(
      <TacticsTab gameState={makeGameState()} onSelectPlayer={vi.fn()} onGameUpdate={vi.fn()} />,
    );

    // Modal requires two players — select f1 then m1 to open comparison
    fireEvent.click(screen.getByTestId("xi-player-f1"));
    fireEvent.click(screen.getByTestId("xi-player-m1"));

    // The table names positions too (its badges' full names), so look in the comparison itself.
    const comparison = within(screen.getByRole("dialog"));
    expect(comparison.getAllByText("common.positions.Forward").length).toBeGreaterThan(0);
    expect(screen.queryByText("Forward")).not.toBeInTheDocument();
  });

  /**
   * Given a bench player and a starter picked in the table, and the familiarity not arrived
   * When Confirm swap is pressed (nothing can be saved yet)
   * Then the swap is not saved and the comparison stays open with both picks, as it would have
   *      had Confirm never been pressed.
   */
  it("keeps the picks when a swap is confirmed before the familiarity has loaded", async () => {
    familiarityState.arrived = false;
    try {
      render(
        <TacticsTab gameState={makeGameState()} onSelectPlayer={vi.fn()} onGameUpdate={vi.fn()} />,
      );
      await act(async () => {});

      swapBenchDefenderIn();
      await act(async () => {});

      expect(screen.getByText("tactics.comparePlayer")).toBeInTheDocument();
      expect(mockedInvoke).not.toHaveBeenCalledWith("set_starting_xi", expect.anything());
    } finally {
      familiarityState.arrived = true;
    }
  });

  describe("when a lineup change is not saved because the squad has not answered", () => {
    /**
     * Given a bench player and a starter selected (the comparison is open), and the game has
     *       moved on so the squad's new answer is still in flight (the familiarity is there)
     * When a lineup action is taken from the screen
     * Then nothing is saved and both stay selected: the comparison is still open.
     *
     * The same action is first taken with the squad answered and must save: that proves the
     * fixture builds a lineup, so "nothing saved" below is the save refusing it and not an action
     * that had nothing to build. (Withholding the familiarity would make the builders return no
     * lineup, and the save would never be asked.)
     */
    async function takeActionWithPairSelected(act_: () => void): Promise<void> {
      const state = makeGameState();
      const roster = state.players.filter((player) => player.team_id === "team1");
      const takeAction = async (squadInFlight: boolean) => {
        mockedInvoke.mockReset();
        mockedInvoke.mockResolvedValueOnce(roster).mockResolvedValue(state);
        const { rerender } = render(
          <TacticsTab gameState={state} onSelectPlayer={vi.fn()} onGameUpdate={vi.fn()} />,
        );
        await act(async () => {});
        fireEvent.click(screen.getByTestId("pitch-bench-player-d5"));
        fireEvent.click(screen.getByTestId("xi-player-d2"));
        expect(screen.getByText("tactics.comparePlayer")).toBeInTheDocument();
        if (squadInFlight) {
          mockedInvoke.mockReset();
          mockedInvoke.mockReturnValue(new Promise(() => {}));
          rerender(
            <TacticsTab gameState={{ ...state }} onSelectPlayer={vi.fn()} onGameUpdate={vi.fn()} />,
          );
        }

        act_();
        await act(async () => {});
        const result = {
          saved: mockedInvoke.mock.calls.some(([command]) => command === "set_starting_xi"),
          comparisonOpen: screen.queryByText("tactics.comparePlayer") !== null,
        };
        cleanup();
        return result;
      };

      expect((await takeAction(false)).saved).toBe(true);

      const withheld = await takeAction(true);
      expect(withheld.saved).toBe(false);
      expect(withheld.comparisonOpen).toBe(true);
    }

    it("keeps the selection when Confirm swap is pressed", async () => {
      await takeActionWithPairSelected(() =>
        fireEvent.click(screen.getByRole("button", { name: "tactics.confirmSwap" })),
      );
    });

    it("keeps the selection when a starter is moved to the bench", async () => {
      await takeActionWithPairSelected(() => {
        fireEvent.contextMenu(screen.getByTestId("xi-row-d1"));
        fireEvent.click(screen.getByRole("menuitem", { name: "tactics.moveToBench" }));
      });
    });

    it("keeps the selection when a starter is given his best-fit slot", async () => {
      await takeActionWithPairSelected(() => {
        fireEvent.contextMenu(screen.getByTestId("xi-row-d1"));
        fireEvent.click(screen.getByRole("menuitem", { name: "tactics.assignBestFit" }));
      });
    });

    it("keeps the selection when a bench player is promoted", async () => {
      await takeActionWithPairSelected(() => {
        fireEvent.contextMenu(screen.getByTestId("bench-player-d5"));
        fireEvent.click(screen.getByRole("menuitem", { name: "tactics.promoteToLineup" }));
      });
    });
  });

  it("allows selecting a bench player from the pitch view and swapping them with a starter", async () => {
    render(
      <TacticsTab gameState={makeGameState()} onSelectPlayer={vi.fn()} onGameUpdate={vi.fn()} />,
    );
    await act(async () => {}); // the squad has answered for this revision

    fireEvent.click(screen.getByTestId("pitch-bench-player-d5"));

    // Modal only opens after both players are selected
    expect(screen.queryByText("tactics.selectedPlayer")).not.toBeInTheDocument();

    fireEvent.click(screen.getByTestId("xi-player-d2"));

    expect(mockedInvoke).not.toHaveBeenCalledWith("set_starting_xi", expect.anything());
    expect(screen.getByText("tactics.comparePlayer")).toBeInTheDocument();

    fireEvent.click(screen.getByRole("button", { name: "tactics.confirmSwap" }));

    await waitFor(() => {
      expect(mockedInvoke).toHaveBeenCalledWith("set_starting_xi", {
        careerToken: 0,
        playerIds: ["gk1", "d1", "d5", "d3", "d4", "m1", "m2", "m3", "m4", "f1", "f2"],
      });
    });
  });

  it("uses pitch clicks for selection and swap instead of opening the player profile", async () => {
    const onSelectPlayer = vi.fn();

    render(
      <TacticsTab
        gameState={makeGameState()}
        onSelectPlayer={onSelectPlayer}
        onGameUpdate={vi.fn()}
      />,
    );
    await act(async () => {}); // the squad has answered for this revision

    fireEvent.click(screen.getByTestId("xi-player-d1"));

    expect(onSelectPlayer).not.toHaveBeenCalled();
    // Modal only opens after both players are selected
    expect(screen.queryByText("tactics.selectedPlayer")).not.toBeInTheDocument();

    fireEvent.click(screen.getByTestId("xi-player-d2"));

    expect(onSelectPlayer).not.toHaveBeenCalled();
    expect(mockedInvoke).not.toHaveBeenCalledWith("set_starting_xi", expect.anything());
    expect(screen.getByText("tactics.comparePlayer")).toBeInTheDocument();

    fireEvent.click(screen.getByRole("button", { name: "tactics.confirmSwap" }));

    await waitFor(() => {
      expect(mockedInvoke).toHaveBeenCalledWith("set_starting_xi", {
        careerToken: 0,
        playerIds: ["gk1", "d2", "d1", "d3", "d4", "m1", "m2", "m3", "m4", "f1", "f2"],
      });
    });
  });

  it("shows a comparison panel after selecting a second pitch player", () => {
    render(
      <TacticsTab gameState={makeGameState()} onSelectPlayer={vi.fn()} onGameUpdate={vi.fn()} />,
    );

    fireEvent.click(screen.getByTestId("xi-player-d1"));
    fireEvent.click(screen.getByTestId("xi-player-m1"));

    expect(screen.getByText("tactics.comparePlayer")).toBeInTheDocument();
    expect(screen.getAllByText("Player m1").length).toBeGreaterThan(0);
    expect(screen.getAllByText("common.attributes.vision").length).toBeGreaterThan(0);
    expect(screen.getByRole("button", { name: "tactics.confirmSwap" })).toBeInTheDocument();
  });

  it("clicking a starter row in the left panel selects them for swap, not opening player profile", () => {
    const onSelectPlayer = vi.fn();

    render(
      <TacticsTab
        gameState={makeGameState()}
        onSelectPlayer={onSelectPlayer}
        onGameUpdate={vi.fn()}
      />,
    );

    fireEvent.click(screen.getByTestId("xi-player-d1"));

    expect(onSelectPlayer).not.toHaveBeenCalled();
    // Modal stays closed until a second player is selected
    expect(screen.queryByText("tactics.selectedPlayer")).not.toBeInTheDocument();
  });

  /**
   * Given starters rated 76 and 77, either side of the A-tier cut-off
   * When the lineup list renders
   * Then each rating badge takes the colour of its backend tier (B and A).
   */
  it("colours each starter's rating badge by its backend tier", () => {
    const gameState = makeGameState();
    gameState.players = gameState.players.map((player) =>
      player.id === "d1"
        ? { ...player, ovr: 76 }
        : player.id === "f1"
          ? { ...player, ovr: 77 }
          : player,
    );

    render(<TacticsTab gameState={gameState} onSelectPlayer={vi.fn()} onGameUpdate={vi.fn()} />);

    // The rating is the row's RatingChip, whose box carries the tier's tone.
    expect(within(screen.getByTestId("xi-row-f1")).getByText("77").parentElement).toHaveClass(
      ...ratingTierClass("A", "badge").split(" "),
    );
    expect(within(screen.getByTestId("xi-row-d1")).getByText("76").parentElement).toHaveClass(
      ...ratingTierClass("B", "badge").split(" "),
    );
  });

  it("shows all starting XI players in the left panel list", () => {
    render(
      <TacticsTab gameState={makeGameState()} onSelectPlayer={vi.fn()} onGameUpdate={vi.fn()} />,
    );

    expect(screen.getByTestId("xi-player-d1")).toBeInTheDocument();
    expect(screen.getByTestId("xi-player-gk1")).toBeInTheDocument();
    expect(screen.getByTestId("xi-player-f1")).toBeInTheDocument();
  });

  // A natural striker occupying the right-midfield slot (index 8 in 4-4-2).
  // Issue #272: the left panel and the pitch role picker must follow the
  // deployed slot, which is also what the backend validates roles against.
  const makeOutOfPositionGameState = (): GameStateData => {
    const gameState = makeGameState();
    gameState.players = gameState.players.map((player) =>
      player.id === "m4" ? { ...player, position: "Striker", natural_position: "Striker" } : player,
    );
    return gameState;
  };

  it("shows the deployed slot position for a starter played out of his natural position", () => {
    const gameState = makeOutOfPositionGameState();
    mockedInvoke.mockImplementation(async (command: string) => {
      if (command === "get_squad") return gameState.players.filter((p) => p.team_id === "team1");
      return gameState;
    });

    render(<TacticsTab gameState={gameState} onSelectPlayer={vi.fn()} onGameUpdate={vi.fn()} />);

    // The row's position cell is the slot he plays in, not his natural position (his natural one is
    // among the "can play" badges).
    const positionCell = within(screen.getByTestId("xi-row-m4")).getAllByRole("cell")[1];
    expect(positionCell).toHaveTextContent("common.posAbbr.RightMidfielder");
    expect(positionCell).not.toHaveTextContent("common.posAbbr.Striker");
  });

  it("offers pitch roles for the deployed slot, not the natural position", () => {
    const gameState = makeOutOfPositionGameState();
    mockedInvoke.mockImplementation(async (command: string) => {
      if (command === "get_squad") return gameState.players.filter((p) => p.team_id === "team1");
      return gameState;
    });

    render(<TacticsTab gameState={gameState} onSelectPlayer={vi.fn()} onGameUpdate={vi.fn()} />);

    const row = screen.getByTestId("xi-row-m4");
    fireEvent.click(within(row).getByRole("combobox"));

    // Right-midfield roles are on offer; striker-only roles are not.
    expect(
      screen.getByRole("option", { name: "tactics.playerRoles.InvertedWinger" }),
    ).toBeInTheDocument();
    expect(
      screen.queryByRole("option", { name: "tactics.playerRoles.Poacher" }),
    ).not.toBeInTheDocument();
  });

  /**
   * Given a striker standing at right-midfield whose stored role is Poacher, and a backend lineup
   *       that says he plays Standard there (a poacher is a striker's role, which the slot does not
   *       admit)
   * When the Tactics screen shows him
   * Then the table shows Standard: the role the match will play, not the stale stored one (the
   *      pitch no longer shows a role at all, so the table is the one place it can be read or changed).
   */
  it("shows the role the match will play for a starter in the table, not the stale stored one", async () => {
    const gameState = makeOutOfPositionGameState();
    gameState.teams = gameState.teams.map((team) => ({
      ...team,
      player_roles: { m4: "Poacher" },
    }));
    const roster = gameState.players.filter((p) => p.team_id === "team1");
    const eleven = ["gk1", "d1", "d2", "d3", "d4", "m1", "m2", "m3", "m4", "f1", "f2"];
    mockedInvoke.mockImplementation(async (command: string) => {
      if (command === "get_squad") return roster;
      if (command === "get_tactics_lineup") {
        return {
          formation: "4-4-2",
          eleven: eleven.map((player_id) => ({
            position: player_id === "m4" ? "RightMidfielder" : "CentralMidfielder",
            player_id,
            role: "Standard",
            familiarity: player_id === "m4" ? "out_of_position" : "natural",
          })),
          substitutes: [],
          reserves: [],
        };
      }
      return gameState;
    });

    render(<TacticsTab gameState={gameState} onSelectPlayer={vi.fn()} onGameUpdate={vi.fn()} />);

    const tableRole = await screen.findByRole("combobox", { name: /tactics.playerRoleLabel: M4/ });
    // The select is a button showing only its selected option's label (the options are drawn when it
    // opens), so the whole text is the selected role: exactly Standard, once the lineup has loaded.
    await waitFor(() => expect(tableRole).toHaveTextContent(/^tactics\.playerRoles\.Standard$/));
    expect(tableRole).not.toHaveTextContent("Poacher");
    // And the pitch's token carries no control to disagree with it: its drawing is hidden from the
    // accessibility tree, so look past that (`hidden`) and at anything a keyboard could reach.
    const token = screen.getByTestId("pitch-player-m4");
    expect(within(token).queryByRole("combobox", { hidden: true })).toBeNull();
    expect(focusableIn(token)).toEqual([]);
  });

  it("does not promote an injured bench player into the starting XI", async () => {
    const gameState = makeGameState();
    gameState.players = gameState.players.map((player) =>
      player.id === "d5"
        ? {
            ...player,
            injury: { name: "Hamstring strain", days_remaining: 7 },
            match_day_eligible: false,
          }
        : player,
    );

    render(<TacticsTab gameState={gameState} onSelectPlayer={vi.fn()} onGameUpdate={vi.fn()} />);

    fireEvent.contextMenu(screen.getByTestId("bench-player-d5"));

    expect(
      screen.queryByRole("button", { name: "tactics.promoteToLineup" }),
    ).not.toBeInTheDocument();
    expect(mockedInvoke).not.toHaveBeenCalledWith("set_starting_xi", expect.anything());
  });

  /**
   * Given an injured bench player the backend has ruled out of the match day
   * When the manager picks him and a starter to compare
   * Then Confirm swap is off and the comparison says why, instead of leaving a dead button
   */
  it("says why a swap with an injured bench player cannot be confirmed", async () => {
    const gameState = makeGameState();
    gameState.players = gameState.players.map((player) =>
      player.id === "d5"
        ? {
            ...player,
            injury: { name: "Hamstring strain", days_remaining: 7 },
            match_day_eligible: false,
          }
        : player,
    );
    mockedInvoke.mockImplementation(async (command: string) =>
      command === "get_squad" ? gameState.players.filter((p) => p.team_id === "team1") : {},
    );

    render(<TacticsTab gameState={gameState} onSelectPlayer={vi.fn()} onGameUpdate={vi.fn()} />);

    fireEvent.click(await screen.findByTestId("pitch-bench-player-d5"));
    fireEvent.click(screen.getByTestId("xi-player-d1"));

    const confirm = await screen.findByRole("button", { name: "tactics.confirmSwap" });
    expect(confirm).toBeDisabled();
    expect(screen.getByText("tactics.swapBlocked.unavailable")).toBeInTheDocument();
    expect(confirm).toHaveAccessibleDescription("tactics.swapBlocked.unavailable");
  });

  /**
   * Given an injury crisis in which the backend fields an injured player anyway
   * When the manager opens his bench context menu
   * Then he can be promoted: the lineup follows the backend's match-day verdict, not its own
   *      injury test.
   */
  it("promotes an injured bench player the backend has made eligible", async () => {
    const gameState = makeGameState();
    gameState.players = gameState.players.map((player) =>
      player.id === "d5"
        ? {
            ...player,
            injury: { name: "Hamstring strain", days_remaining: 7 },
            match_day_eligible: true,
          }
        : player,
    );
    mockedInvoke.mockImplementation(async (command: string) =>
      command === "get_squad" ? gameState.players.filter((p) => p.team_id === "team1") : {},
    );

    render(<TacticsTab gameState={gameState} onSelectPlayer={vi.fn()} onGameUpdate={vi.fn()} />);

    fireEvent.contextMenu(await screen.findByTestId("bench-player-d5"));
    fireEvent.click(await screen.findByRole("menuitem", { name: "tactics.promoteToLineup" }));

    await waitFor(() => {
      expect(mockedInvoke).toHaveBeenCalledWith("set_starting_xi", expect.anything());
    });
  });

  it("does not allow swapping an injured bench player into the starting XI", () => {
    const gameState = makeGameState();
    gameState.players = gameState.players.map((player) =>
      player.id === "d5"
        ? {
            ...player,
            injury: { name: "Hamstring strain", days_remaining: 7 },
            match_day_eligible: false,
          }
        : player,
    );

    render(<TacticsTab gameState={gameState} onSelectPlayer={vi.fn()} onGameUpdate={vi.fn()} />);

    fireEvent.click(screen.getByTestId("pitch-bench-player-d5"));
    fireEvent.click(screen.getByTestId("xi-player-d2"));

    expect(screen.getByRole("button", { name: "tactics.confirmSwap" })).toBeDisabled();
  });

  it("offers tactics context-menu actions to promote a bench player", async () => {
    render(
      <TacticsTab gameState={makeGameState()} onSelectPlayer={vi.fn()} onGameUpdate={vi.fn()} />,
    );
    await act(async () => {}); // the squad has answered for this revision

    const benchRow = screen.getByTestId("bench-player-d5");
    fireEvent.contextMenu(benchRow);
    fireEvent.click(screen.getByRole("menuitem", { name: "tactics.promoteToLineup" }));

    // d5 is a coarse "Defender", graded as a centre back as the backend reads him: he is natural at
    // the first centre-back slot (replacing d2), not at the first slot that merely adapts him.
    await waitFor(() => {
      expect(mockedInvoke).toHaveBeenCalledWith("set_starting_xi", {
        careerToken: 0,
        playerIds: ["gk1", "d1", "d5", "d3", "d4", "m1", "m2", "m3", "m4", "f1", "f2"],
      });
    });
  });

  it("offers a starter's context menu the move to the bench", async () => {
    render(
      <TacticsTab gameState={makeGameState()} onSelectPlayer={vi.fn()} onGameUpdate={vi.fn()} />,
    );
    await act(async () => {}); // the squad has answered for this revision

    fireEvent.contextMenu(screen.getByTestId("xi-row-d1"));
    fireEvent.click(screen.getByRole("menuitem", { name: "tactics.moveToBench" }));

    await waitFor(() => {
      expect(mockedInvoke).toHaveBeenCalledWith("set_starting_xi", {
        careerToken: 0,
        playerIds: ["gk1", "d5", "d2", "d3", "d4", "m1", "m2", "m3", "m4", "f1", "f2"],
      });
    });
  });
});
