import { fireEvent, render, screen, waitFor, within } from "@testing-library/react";
import { beforeEach, describe, expect, it, vi } from "vitest";
import { invoke } from "@tauri-apps/api/core";
import TacticsTab from "./TacticsTab";
import { resetTacticsLineupCache } from "../../hooks/useTacticsLineupProjection";
import { makeGameState } from "../../test-utils/tacticsTabFixtures";

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

describe("TacticsTab set pieces and match roles", () => {
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
    // The manager picked D3 as corner taker, and only that is saved: the other takers on screen are the
    // backend's fallback picks, which are not written.
    expect(saved.matchRoles.corner_taker).toBe("d3");
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
