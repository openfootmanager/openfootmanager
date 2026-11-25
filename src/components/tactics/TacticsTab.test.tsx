import { act, cleanup, fireEvent, render, screen, waitFor, within } from "@testing-library/react";
import { beforeEach, describe, expect, it, vi } from "vitest";
import { invoke, type InvokeArgs } from "@tauri-apps/api/core";
import type { GameStateData, PlayerData } from "../../store/gameStore";
import type { CustomTactic } from "../../services/customTacticsService";
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

  it("can duplicate the current setup into a custom tactic shell", () => {
    render(
      <TacticsTab gameState={makeGameState()} onSelectPlayer={vi.fn()} onGameUpdate={vi.fn()} />,
    );

    fireEvent.click(screen.getByRole("button", { name: "tactics.tacticActions" }));
    fireEvent.click(screen.getByRole("menuitem", { name: "tactics.duplicateTactic" }));

    expect(screen.getByRole("button", { name: "tactics.updateTactic" })).toBeInTheDocument();
    fireEvent.click(screen.getByRole("button", { name: "tactics.chooseTactic" }));
    expect(screen.getByRole("option", { name: /tactics.copyOfTactic/i })).toBeInTheDocument();
  });

  /** Makes the faked backend keep saved tactics, as the save does, on top of its other answers. */
  function keepSavedTactics() {
    const saved: CustomTactic[] = [];
    const others = mockedInvoke.getMockImplementation();
    mockedInvoke.mockImplementation(async (command: string, args?: InvokeArgs) => {
      if (command === "list_custom_tactics") return structuredClone(saved);
      if (command === "save_custom_tactic" || command === "save_current_tactic") {
        const tactic = (args as { tactic: CustomTactic }).tactic;
        const at = saved.findIndex((existing) => existing.id === tactic.id);
        if (at === -1) saved.push(tactic);
        else saved[at] = tactic;
        return structuredClone(saved);
      }
      return others ? others(command, args) : undefined;
    });
    return saved;
  }

  /**
   * Given a custom tactic made and saved
   * When the screen is closed and opened again
   * Then the tactic is still in the library: it comes from the save, not from the screen.
   */
  it("keeps custom tactics across a save and reload", async () => {
    keepSavedTactics();
    const gameState = makeGameState();
    const { unmount } = render(
      <TacticsTab gameState={gameState} onSelectPlayer={vi.fn()} onGameUpdate={vi.fn()} />,
    );

    fireEvent.click(screen.getByRole("button", { name: "tactics.tacticActions" }));
    fireEvent.click(screen.getByRole("menuitem", { name: "tactics.duplicateTactic" }));
    await waitFor(() =>
      expect(mockedInvoke).toHaveBeenCalledWith(
        "save_current_tactic",
        expect.objectContaining({ tactic: expect.objectContaining({ formation: "4-4-2" }) }),
      ),
    );
    unmount();

    render(<TacticsTab gameState={gameState} onSelectPlayer={vi.fn()} onGameUpdate={vi.fn()} />);
    fireEvent.click(screen.getByRole("button", { name: "tactics.chooseTactic" }));

    expect(
      await screen.findByRole("option", { name: /tactics.copyOfTactic/i }),
    ).toBeInTheDocument();
  });

  /**
   * Given a custom tactic made while managing one club
   * When the same manager takes charge of another club
   * Then the tactic is still in the library: the library belongs to the career, not the club.
   */
  it("keeps custom tactics when the manager changes club", async () => {
    keepSavedTactics();
    const originalState = makeGameState();
    const otherClub = makeGameState();
    otherClub.manager.team_id = "team2";
    otherClub.teams = [makeTeam({ id: "team2", manager_id: otherClub.manager.id })];
    otherClub.players = otherClub.players.map((player) => ({ ...player, team_id: "team2" }));

    const { unmount } = render(
      <TacticsTab gameState={originalState} onSelectPlayer={vi.fn()} onGameUpdate={vi.fn()} />,
    );
    fireEvent.click(screen.getByRole("button", { name: "tactics.tacticActions" }));
    fireEvent.click(screen.getByRole("menuitem", { name: "tactics.duplicateTactic" }));
    await waitFor(() =>
      expect(mockedInvoke).toHaveBeenCalledWith("save_current_tactic", expect.anything()),
    );
    unmount();

    render(<TacticsTab gameState={otherClub} onSelectPlayer={vi.fn()} onGameUpdate={vi.fn()} />);
    fireEvent.click(screen.getByRole("button", { name: "tactics.chooseTactic" }));

    expect(
      await screen.findByRole("option", { name: /tactics.copyOfTactic/i }),
    ).toBeInTheDocument();
  }, 15000);

  /**
   * Given a library an older version kept in the browser
   * When the screen opens, and opens again
   * Then its tactics are saved once, the old key is gone, and the second opening adds nothing.
   */
  it("copies an old browser library into the save once", async () => {
    const saved = keepSavedTactics();
    const gameState = makeGameState();
    localStorage.setItem(
      `ofm:tactics:custom:${gameState.manager.id}:${gameState.clock.start_date}:team1`,
      JSON.stringify([
        {
          id: "custom:old",
          type: "custom",
          name: "Old favourite",
          description: "",
          formation: "4-4-2",
          playStyle: "Balanced",
          sourcePresetName: null,
        },
      ]),
    );

    const first = render(
      <TacticsTab gameState={gameState} onSelectPlayer={vi.fn()} onGameUpdate={vi.fn()} />,
    );
    fireEvent.click(screen.getByRole("button", { name: "tactics.chooseTactic" }));
    expect(await screen.findByRole("option", { name: /Old favourite/ })).toBeInTheDocument();
    first.unmount();
    const secondRender = render(
      <TacticsTab gameState={gameState} onSelectPlayer={vi.fn()} onGameUpdate={vi.fn()} />,
    );
    fireEvent.click(screen.getByRole("button", { name: "tactics.chooseTactic" }));
    expect(await screen.findByRole("option", { name: /Old favourite/ })).toBeInTheDocument();
    secondRender.unmount();

    expect(saved.map((tactic) => tactic.id)).toEqual(["custom:old"]);
    // Copied, never moved: the old key is shared by careers begun the same year.
    expect(localStorage.length).toBe(1);
    const saves = mockedInvoke.mock.calls.filter(([command]) => command === "save_custom_tactic");
    expect(saves).toHaveLength(1);
  });

  it("does not mark a preset as active when applying it fails", async () => {
    const gameState = makeGameState();
    mockedInvoke.mockImplementation(async (command) => {
      if (command === "apply_tactic_preset") {
        throw new Error("boom");
      }

      if (command === "get_squad") {
        return gameState.players.filter((p) => p.team_id === "team1");
      }

      return gameState;
    });

    render(
      <TacticsTab gameState={makeGameState()} onSelectPlayer={vi.fn()} onGameUpdate={vi.fn()} />,
    );

    expect(screen.getByRole("button", { name: "tactics.chooseTactic" })).toHaveTextContent(
      "balanced-control",
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

    expect(screen.getByRole("button", { name: "tactics.chooseTactic" })).toHaveTextContent(
      "balanced-control",
    );
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

  it("assigns captaincy from the tactics table context menu", async () => {
    const gameState = makeGameState();
    const roster = gameState.players.filter((p) => p.team_id === "team1");
    mockedInvoke.mockImplementation(async (command: string) => {
      if (command === "get_squad") return roster;
      if (command === "auto_select_set_pieces") {
        return { captain: "m2", penalty_taker: "f2", free_kick_taker: "m3", corner_taker: "d4" };
      }
      return gameState;
    });
    render(<TacticsTab gameState={gameState} onSelectPlayer={vi.fn()} onGameUpdate={vi.fn()} />);
    // The backend's picks are on screen, so a save built from what is displayed would carry them.
    await waitFor(() => {
      expect(mockedInvoke).toHaveBeenCalledWith("auto_select_set_pieces", expect.anything());
    });
    await waitFor(() => expect(screen.getAllByText("match.notAssigned")).toHaveLength(1));

    fireEvent.contextMenu(screen.getByTestId("xi-player-d1"));
    fireEvent.click(screen.getByRole("menuitem", { name: "tactics.makeCaptain" }));

    await waitFor(() => {
      expect(mockedInvoke).toHaveBeenCalledWith("set_team_match_roles", {
        careerToken: 0,
        matchRoles: {
          captain: "d1",
          vice_captain: null,
          penalty_taker: null,
          free_kick_taker: null,
          corner_taker: null,
        },
      });
    });
  });

  it("assigns captaincy from a starter's context menu", async () => {
    const gameState = makeGameState();
    const roster = gameState.players.filter((p) => p.team_id === "team1");
    mockedInvoke.mockImplementation(async (command: string) => {
      if (command === "get_squad") return roster;
      if (command === "auto_select_set_pieces") {
        return { captain: "m2", penalty_taker: "f2", free_kick_taker: "m3", corner_taker: "d4" };
      }
      return gameState;
    });
    render(<TacticsTab gameState={gameState} onSelectPlayer={vi.fn()} onGameUpdate={vi.fn()} />);
    // The backend's picks are on screen, so a save built from what is displayed would carry them.
    await waitFor(() => {
      expect(mockedInvoke).toHaveBeenCalledWith("auto_select_set_pieces", expect.anything());
    });
    await waitFor(() => expect(screen.getAllByText("match.notAssigned")).toHaveLength(1));

    fireEvent.contextMenu(screen.getByTestId("xi-row-d1"));
    fireEvent.click(screen.getByRole("menuitem", { name: "tactics.makeCaptain" }));

    await waitFor(() => {
      expect(mockedInvoke).toHaveBeenCalledWith("set_team_match_roles", {
        careerToken: 0,
        matchRoles: {
          captain: "d1",
          vice_captain: null,
          penalty_taker: null,
          free_kick_taker: null,
          corner_taker: null,
        },
      });
    });
  });

  /** Given the backend's own set-piece picks for the eleven, when the manager auto-selects,
   * then those picks are saved, and no vice-captain is invented for them. */
  it("persists the backend's set-piece picks when the manager auto-selects", async () => {
    const gameState = makeGameState();
    const roster = gameState.players.filter((p) => p.team_id === "team1");
    mockedInvoke.mockImplementation(async (command: string) => {
      if (command === "get_squad") return roster;
      if (command === "auto_select_set_pieces") {
        return { captain: "m2", penalty_taker: "f2", free_kick_taker: "m3", corner_taker: "d4" };
      }
      return gameState;
    });
    render(<TacticsTab gameState={gameState} onSelectPlayer={vi.fn()} onGameUpdate={vi.fn()} />);
    await waitFor(() => {
      expect(mockedInvoke).toHaveBeenCalledWith("auto_select_set_pieces", {
        playerIds: ["gk1", "d1", "d2", "d3", "d4", "m1", "m2", "m3", "m4", "f1", "f2"],
      });
    });

    fireEvent.click(screen.getByRole("button", { name: "tactics.autoSelectAssignments" }));

    await waitFor(() => {
      expect(mockedInvoke).toHaveBeenCalledWith("set_team_match_roles", {
        careerToken: 0,
        matchRoles: {
          captain: "m2",
          vice_captain: null,
          penalty_taker: "f2",
          free_kick_taker: "m3",
          corner_taker: "d4",
        },
      });
    });
  });

  /** Given the backend's picks changed after the panel last asked (the squad moved on),
   * when the manager auto-selects, then what is saved is the answer asked for at the click,
   * and a failed ask saves nothing so that clicking again is the retry. */
  it("saves the picks the backend gives at the click, and nothing when the ask fails", async () => {
    const gameState = makeGameState();
    const roster = gameState.players.filter((p) => p.team_id === "team1");
    let answer: "stale" | "fresh" | "fails" = "stale";
    mockedInvoke.mockImplementation(async (command: string) => {
      if (command === "get_squad") return roster;
      if (command === "auto_select_set_pieces") {
        if (answer === "fails") throw new Error("backend busy");
        return answer === "stale"
          ? { captain: "m1", penalty_taker: "f1", free_kick_taker: "m1", corner_taker: "d1" }
          : { captain: "m2", penalty_taker: "f2", free_kick_taker: "m3", corner_taker: "d4" };
      }
      return gameState;
    });
    render(<TacticsTab gameState={gameState} onSelectPlayer={vi.fn()} onGameUpdate={vi.fn()} />);
    await waitFor(() => {
      expect(mockedInvoke).toHaveBeenCalledWith("auto_select_set_pieces", expect.anything());
    });
    const rolesSaved = () =>
      mockedInvoke.mock.calls.filter(([command]) => command === "set_team_match_roles");

    answer = "fails";
    fireEvent.click(screen.getByRole("button", { name: "tactics.autoSelectAssignments" }));
    await waitFor(() => {
      expect(mockedInvoke.mock.calls.filter(([c]) => c === "auto_select_set_pieces").length).toBe(
        2,
      );
    });
    expect(rolesSaved()).toHaveLength(0);

    answer = "fresh";
    fireEvent.click(screen.getByRole("button", { name: "tactics.autoSelectAssignments" }));
    await waitFor(() => {
      expect(rolesSaved()).toHaveLength(1);
    });
    expect(rolesSaved()[0][1]).toEqual({
      careerToken: 0,
      matchRoles: {
        captain: "m2",
        vice_captain: null,
        penalty_taker: "f2",
        free_kick_taker: "m3",
        corner_taker: "d4",
      },
    });
  });

  /** Given an auto-select still waiting on the backend, when the manager's roles change
   * meanwhile, then the late answer is dropped instead of overwriting them. */
  it("drops a late auto-select answer when the roles changed while it was pending", async () => {
    const gameState = makeGameState();
    const roster = gameState.players.filter((p) => p.team_id === "team1");
    const releases: Array<(picks: object) => void> = [];
    let pending = false;
    mockedInvoke.mockImplementation(async (command: string) => {
      if (command === "get_squad") return roster;
      if (command === "auto_select_set_pieces") {
        if (!pending)
          return { captain: "m1", penalty_taker: "f1", free_kick_taker: "m1", corner_taker: "d1" };
        return new Promise((resolve) => releases.push(resolve));
      }
      return gameState;
    });
    const props = { onSelectPlayer: vi.fn(), onGameUpdate: vi.fn() };
    const { rerender } = render(<TacticsTab gameState={gameState} {...props} />);
    await waitFor(() => {
      expect(mockedInvoke).toHaveBeenCalledWith("auto_select_set_pieces", expect.anything());
    });
    pending = true;
    fireEvent.click(screen.getByRole("button", { name: "tactics.autoSelectAssignments" }));
    await waitFor(() => expect(releases).toHaveLength(1));

    const reassigned = {
      ...gameState,
      teams: gameState.teams.map((team) =>
        team.id === "team1"
          ? {
              ...team,
              match_roles: {
                captain: "d2",
                vice_captain: null,
                penalty_taker: null,
                free_kick_taker: null,
                corner_taker: null,
              },
            }
          : team,
      ),
    };
    rerender(<TacticsTab gameState={reassigned} {...props} />);
    // The panel's own display refreshes with the new game too; every pending ask is answered.
    for (const release of releases) {
      release({ captain: "m2", penalty_taker: "f2", free_kick_taker: "m3", corner_taker: "d4" });
    }
    await new Promise((resolve) => setTimeout(resolve, 50));

    expect(mockedInvoke.mock.calls.filter(([c]) => c === "set_team_match_roles")).toHaveLength(0);
  });

  /** Given the backend would pick takers for the unassigned roles, when the manager assigns
   * one role, then only that role is saved: the others stay null for the backend to choose. */
  it("saves only the manager's own roles on a manual assignment", async () => {
    const gameState = makeGameState();
    const roster = gameState.players.filter((p) => p.team_id === "team1");
    mockedInvoke.mockImplementation(async (command: string) => {
      if (command === "get_squad") return roster;
      if (command === "auto_select_set_pieces") {
        return { captain: "m2", penalty_taker: "f2", free_kick_taker: "m3", corner_taker: "d4" };
      }
      return gameState;
    });
    render(<TacticsTab gameState={gameState} onSelectPlayer={vi.fn()} onGameUpdate={vi.fn()} />);
    await waitFor(() => {
      expect(mockedInvoke).toHaveBeenCalledWith("auto_select_set_pieces", expect.anything());
    });

    // The takers sit in the team instructions panel; the pitch and the list name D3 too.
    const instructions = within(
      screen.getByRole("region", { name: "tactics.instructionsPanel.title" }),
    );
    fireEvent.click(instructions.getByRole("button", { name: /preMatch\.cornerTaker/ }));
    fireEvent.click(await instructions.findByRole("button", { name: /D3/ }));

    await waitFor(() => {
      expect(mockedInvoke).toHaveBeenCalledWith("set_team_match_roles", expect.anything());
    });
    const saved = mockedInvoke.mock.calls.find(([c]) => c === "set_team_match_roles")?.[1] as {
      matchRoles: Record<string, string | null>;
    };
    expect(saved.matchRoles.captain).toBeNull();
    expect(saved.matchRoles.penalty_taker).toBeNull();
    expect(saved.matchRoles.free_kick_taker).toBeNull();
    expect(saved.matchRoles.vice_captain).toBeNull();
  });

  /** Given the fallback takers are shown, when the game updates with the same eleven, then
   * they stay shown while the new picks are on their way (only the vice-captain is blank). */
  it("keeps the fallback takers shown across a game update with the same eleven", async () => {
    const gameState = makeGameState();
    const roster = gameState.players.filter((p) => p.team_id === "team1");
    let slow = false;
    mockedInvoke.mockImplementation(async (command: string) => {
      if (command === "get_squad") return roster;
      if (command === "auto_select_set_pieces") {
        if (slow) return new Promise(() => {});
        return { captain: "m2", penalty_taker: "f2", free_kick_taker: "m3", corner_taker: "d4" };
      }
      return gameState;
    });
    const props = { onSelectPlayer: vi.fn(), onGameUpdate: vi.fn() };
    const { rerender } = render(<TacticsTab gameState={gameState} {...props} />);
    await waitFor(() => expect(screen.getAllByText("match.notAssigned")).toHaveLength(1));
    slow = true;

    rerender(<TacticsTab gameState={{ ...gameState }} {...props} />);
    await waitFor(() => {
      expect(mockedInvoke.mock.calls.filter(([c]) => c === "auto_select_set_pieces")).toHaveLength(
        2,
      );
    });

    expect(screen.getAllByText("match.notAssigned")).toHaveLength(1);
  });
});
