import { act, renderHook, waitFor } from "@testing-library/react";
import { invoke } from "@tauri-apps/api/core";
import { beforeEach, describe, expect, it, vi } from "vitest";

import { type GameStateData, useGameStore } from "../../store/gameStore";
import { useTacticsLibrary } from "./useTacticsLibrary";

vi.mock("@tauri-apps/api/core", () => ({ invoke: vi.fn() }));
vi.mock("react-i18next", () => ({
  initReactI18next: { type: "3rdParty", init: () => {} },
  useTranslation: () => ({
    t: (key: string, fallback?: unknown) => (typeof fallback === "string" ? fallback : key),
    i18n: { language: "en" },
  }),
}));

const mockedInvoke = vi.mocked(invoke);

function renderLibrary(gameState: GameStateData | null) {
  return renderHook(() =>
    useTacticsLibrary({
      gameState,
      formation: "4-4-2",
      activePlayStyle: "Balanced",
      initialPreset: null,
      onFormationChange: vi.fn(),
      onPlayStyleChange: vi.fn(),
      onApplyPreset: vi.fn(),
    }),
  );
}

describe("useTacticsLibrary blueprint status", () => {
  beforeEach(() => {
    mockedInvoke.mockReset();
    mockedInvoke.mockResolvedValue({ modified: false });
  });

  /**
   * Given no game is loaded
   * When the hook runs
   * Then the backend is not asked about a blueprint.
   */
  it("does not ask the backend without a game", () => {
    renderLibrary(null);

    expect(mockedInvoke).not.toHaveBeenCalledWith("get_tactics_blueprint_status");
  });

  /**
   * Given a game whose manager has no club
   * When the hook runs
   * Then the backend is not asked about a blueprint.
   */
  it("does not ask the backend without a club", () => {
    const clubless = {
      clock: { current_date: "2026-08-01", start_date: "2026-08-01" },
      teams: [],
      players: [],
      competitions: [],
      manager: { team_id: null },
    } as unknown as GameStateData;

    renderLibrary(clubless);

    expect(mockedInvoke).not.toHaveBeenCalledWith("get_tactics_blueprint_status");
  });
});

/** A promise settled by hand, so a test decides which reply arrives first. */
function deferred<T>() {
  let resolve: (value: T) => void = () => {};
  const promise = new Promise<T>((r) => {
    resolve = r;
  });
  return { promise, resolve };
}

const careerGame = {
  clock: { current_date: "2026-08-01", start_date: "2026-08-01" },
  teams: [],
  players: [],
  competitions: [],
  manager: { id: "mgr1", team_id: null },
} as unknown as GameStateData;

const customIds = (result: { current: ReturnType<typeof useTacticsLibrary> }) =>
  result.current.tacticLibrary.filter((entry) => entry.type === "custom").map((entry) => entry.id);

describe("useTacticsLibrary custom tactics and the save", () => {
  /**
   * Given the library is still loading from the save (its list has not come back)
   * When the player makes a tactic, and then the stale list arrives
   * Then the new tactic is still in the library: nothing made meanwhile is overwritten.
   */
  it("keeps a tactic made while the library is still loading", async () => {
    const list = deferred<unknown[]>();
    const saved: Array<{ id: string }> = [];
    mockedInvoke.mockReset();
    mockedInvoke.mockImplementation(async (command: string, args?: unknown) => {
      if (command === "list_custom_tactics") return list.promise;
      if (command === "save_custom_tactic") {
        saved.push((args as { tactic: { id: string } }).tactic);
        return structuredClone(saved);
      }
      return { modified: false };
    });
    const { result } = renderLibrary(careerGame);

    act(() => result.current.handleCreateCustomTactic());
    await act(async () => list.resolve([]));

    await waitFor(() => expect(saved).toHaveLength(1));
    await waitFor(() => expect(customIds(result)).toEqual([saved[0].id]));
  });

  /**
   * Given two tactics saved one right after the other, the first save's reply arriving late
   * When both have settled
   * Then both are in the library: an older reply never replaces a newer one.
   */
  it("applies the replies of overlapping saves in the order they were made", async () => {
    const firstReply = deferred<unknown[]>();
    const saved: Array<{ id: string }> = [];
    let saves = 0;
    mockedInvoke.mockReset();
    mockedInvoke.mockImplementation(async (command: string, args?: unknown) => {
      if (command === "list_custom_tactics") return [];
      if (command === "save_custom_tactic") {
        saved.push((args as { tactic: { id: string } }).tactic);
        saves += 1;
        const snapshot = structuredClone(saved);
        return saves === 1 ? firstReply.promise.then(() => snapshot) : snapshot;
      }
      return { modified: false };
    });
    const { result } = renderLibrary(careerGame);
    await waitFor(() => expect(mockedInvoke).toHaveBeenCalledWith("list_custom_tactics"));

    act(() => result.current.handleCreateCustomTactic());
    await waitFor(() => expect(saved).toHaveLength(1));
    act(() => result.current.handleCreateCustomTactic());
    // The second save waits for the first: only one has started while the first reply is out.
    expect(saved).toHaveLength(1);
    await act(async () => firstReply.resolve([]));

    await waitFor(() => expect(saved).toHaveLength(2));
    await waitFor(() => expect(customIds(result).sort()).toEqual(saved.map((t) => t.id).sort()));
  });

  const saveCalls = () =>
    mockedInvoke.mock.calls.filter(([command]) => command === "save_custom_tactic");

  /**
   * Given a tactic waiting behind the first load, and the player leaves the screen while another
   *       game replaces the career
   * When the load finishes
   * Then the waiting tactic is not saved: it was asked in a career that is no longer the one open.
   */
  it("does not save a queued tactic once the screen has been left", async () => {
    const list = deferred<unknown[]>();
    mockedInvoke.mockReset();
    mockedInvoke.mockImplementation(async (command: string) => {
      if (command === "list_custom_tactics") return list.promise;
      return { modified: false };
    });
    const { result, unmount } = renderLibrary(careerGame);

    act(() => result.current.handleCreateCustomTactic());
    unmount();
    act(() => useGameStore.getState().clearGame());
    await act(async () => list.resolve([]));

    expect(saveCalls()).toHaveLength(0);
  });

  /**
   * Given a tactic waiting behind the first load, and the player leaves the screen while the same
   *       career stays open
   * When the load finishes
   * Then the waiting tactic is saved: it is still that career's, and leaving a screen lost nothing.
   */
  it("saves a queued tactic when the screen is left but the career is the same", async () => {
    const list = deferred<unknown[]>();
    mockedInvoke.mockReset();
    mockedInvoke.mockImplementation(async (command: string) => {
      if (command === "list_custom_tactics") return list.promise;
      if (command === "save_custom_tactic") return [];
      return { modified: false };
    });
    const { result, unmount } = renderLibrary(careerGame);

    act(() => result.current.handleCreateCustomTactic());
    unmount();
    await act(async () => list.resolve([]));

    expect(saveCalls()).toHaveLength(1);
  });

  /**
   * Given a tactic waiting behind the first load, and the career then changes while the screen
   *       stays open
   * When the first load finishes
   * Then the waiting tactic is not saved into the new career, which is loaded on its own.
   */
  it("does not save a queued tactic into the career that replaced its own", async () => {
    const firstList = deferred<unknown[]>();
    const secondList = deferred<unknown[]>();
    let lists = 0;
    mockedInvoke.mockReset();
    mockedInvoke.mockImplementation(async (command: string) => {
      if (command === "list_custom_tactics") {
        lists += 1;
        return lists === 1 ? firstList.promise : secondList.promise;
      }
      return { modified: false };
    });
    const otherCareer = {
      ...careerGame,
      manager: { id: "mgr2", team_id: null },
    } as unknown as GameStateData;
    const { result, rerender } = renderHook(
      ({ game }: { game: GameStateData }) =>
        useTacticsLibrary({
          gameState: game,
          formation: "4-4-2",
          activePlayStyle: "Balanced",
          initialPreset: null,
          onFormationChange: vi.fn(),
          onPlayStyleChange: vi.fn(),
          onApplyPreset: vi.fn(),
        }),
      { initialProps: { game: careerGame } },
    );

    await waitFor(() => expect(lists).toBe(1));
    act(() => result.current.handleCreateCustomTactic());
    rerender({ game: otherCareer });
    act(() => useGameStore.getState().setGameActive(true));
    // The first career's list arrives late, with a tactic of its own.
    await act(async () =>
      firstList.resolve([
        {
          id: "from-first",
          name: "First",
          description: "",
          formation: "4-4-2",
          playStyle: "Balanced",
        },
      ]),
    );

    await waitFor(() => expect(lists).toBe(2));
    expect(saveCalls()).toHaveLength(0);
    // And it is not shown in the new career while that career's own list is still out.
    expect(customIds(result)).not.toContain("from-first");
    await act(async () => secondList.resolve([]));
    expect(customIds(result)).toEqual([]);
  });

  /**
   * Given an old browser library for the career, and the screen is left while the save is listed
   * When the list comes back
   * Then none of the old tactics is saved into whatever game is open by then.
   */
  it("does not import an old library once the screen has been left", async () => {
    const list = deferred<unknown[]>();
    localStorage.setItem(
      "ofm:tactics:custom:mgr1:2026-08-01:team1",
      JSON.stringify([
        {
          id: "custom:old",
          type: "custom",
          name: "Old",
          description: "",
          formation: "4-4-2",
          playStyle: "Balanced",
        },
      ]),
    );
    mockedInvoke.mockReset();
    mockedInvoke.mockImplementation(async (command: string) => {
      if (command === "list_custom_tactics") return list.promise;
      if (command === "legacy_tactics_import_pending") return true;
      return { modified: false };
    });
    const { unmount } = renderLibrary(careerGame);
    await waitFor(() => expect(mockedInvoke).toHaveBeenCalledWith("list_custom_tactics"));

    unmount();
    act(() => useGameStore.getState().clearGame());
    await act(async () => list.resolve([]));

    expect(saveCalls()).toHaveLength(0);
    localStorage.clear();
  });
});

const OLD_KEY = "ofm:tactics:custom:mgr1:2026-08-01:team1";
const oldTactic = {
  id: "custom:old",
  type: "custom",
  name: "Old",
  description: "",
  formation: "4-4-2",
  playStyle: "Balanced",
};
const savedTactic = {
  id: "custom:mine",
  name: "Mine",
  description: "",
  formation: "4-4-2",
  playStyle: "Balanced",
};

/** A backend holding `list`, with the legacy import pending or not, recording what is called. */
function backendWith(list: unknown[], pending: boolean, failSaves = false) {
  const current = [...list];
  mockedInvoke.mockReset();
  mockedInvoke.mockImplementation(async (command: string, args?: unknown) => {
    switch (command) {
      case "list_custom_tactics":
        return [...current];
      case "legacy_tactics_import_pending":
        return pending;
      case "mark_legacy_tactics_imported":
        return undefined;
      case "save_custom_tactic": {
        if (failSaves) throw new Error("disk full");
        current.push((args as { tactic: unknown }).tactic);
        return [...current];
      }
      case "delete_custom_tactic": {
        const id = (args as { id: string }).id;
        const at = current.findIndex((tactic) => (tactic as { id: string }).id === id);
        current.splice(at, 1);
        return [...current];
      }
      default:
        return { modified: false };
    }
  });
}

const callsTo = (command: string) =>
  mockedInvoke.mock.calls.filter(([called]) => called === command);

describe("useTacticsLibrary deleting and the old browser library", () => {
  /**
   * Given a career whose old browser library was already copied in, with a tactic still in the
   *       browser's key that the player has since deleted
   * When the library is loaded again
   * Then the old tactic is not copied back: the import happens once per career.
   */
  it("does not copy a deleted tactic back from the browser once the import is done", async () => {
    localStorage.setItem(OLD_KEY, JSON.stringify([oldTactic]));
    backendWith([], false);

    const { result } = renderLibrary(careerGame);
    await waitFor(() => expect(callsTo("list_custom_tactics")).toHaveLength(1));
    await act(async () => {});

    expect(callsTo("save_custom_tactic")).toHaveLength(0);
    expect(customIds(result)).toEqual([]);
    localStorage.clear();
  });

  /**
   * Given a career whose old library has not been copied in
   * When the library is loaded
   * Then the old tactic is saved and the career is marked, once.
   */
  it("copies the old library in and then marks the import done", async () => {
    localStorage.setItem(OLD_KEY, JSON.stringify([oldTactic]));
    backendWith([], true);

    const { result } = renderLibrary(careerGame);
    await waitFor(() => expect(customIds(result)).toEqual(["custom:old"]));

    expect(callsTo("save_custom_tactic")).toHaveLength(1);
    expect(callsTo("mark_legacy_tactics_imported")).toHaveLength(1);
    localStorage.clear();
  });

  /**
   * Given an old library and a save that fails
   * When the library is loaded
   * Then the career is not marked: the tactic it could not save is tried again next time.
   */
  it("does not mark the import done when a save failed", async () => {
    localStorage.setItem(OLD_KEY, JSON.stringify([oldTactic]));
    backendWith([], true, true);

    renderLibrary(careerGame);
    await waitFor(() => expect(callsTo("save_custom_tactic")).toHaveLength(1));
    await act(async () => {});

    expect(callsTo("mark_legacy_tactics_imported")).toHaveLength(0);
    localStorage.clear();
  });

  /**
   * Given the player's setup is a saved custom tactic, and that tactic is the one applied
   * When it is deleted
   * Then the save loses it, the library no longer shows it, and the team keeps its formation and
   *      play style: nothing is applied or changed, and the setup shown falls back to what the
   *      team is actually playing.
   */
  it("deleting the applied tactic leaves the team's setup as it is", async () => {
    backendWith([savedTactic], false);
    const onFormationChange = vi.fn();
    const onPlayStyleChange = vi.fn();
    const onApplyPreset = vi.fn();
    const { result } = renderHook(() =>
      useTacticsLibrary({
        gameState: careerGame,
        formation: "4-4-2",
        activePlayStyle: "Balanced",
        initialPreset: null,
        onFormationChange,
        onPlayStyleChange,
        onApplyPreset,
      }),
    );
    await waitFor(() => expect(customIds(result)).toEqual(["custom:mine"]));
    await act(async () => {
      await result.current.applyTacticSelection(result.current.tacticLibrary[0]);
    });
    expect(result.current.activeTactic.id).toBe("custom:mine");
    expect(result.current.canDeleteTactic).toBe(true);

    act(() => result.current.handleDeleteTactic());

    await waitFor(() => expect(callsTo("delete_custom_tactic")).toHaveLength(1));
    expect(mockedInvoke).toHaveBeenCalledWith("delete_custom_tactic", {
      id: "custom:mine",
      careerToken: 0,
    });
    expect(customIds(result)).toEqual([]);
    expect(result.current.activeTactic.id).not.toBe("custom:mine");
    expect(result.current.activeTactic.formation).toBe("4-4-2");
    expect(result.current.activeTactic.playStyle).toBe("Balanced");
    expect(onFormationChange).not.toHaveBeenCalled();
    expect(onPlayStyleChange).not.toHaveBeenCalled();
    expect(onApplyPreset).not.toHaveBeenCalled();
  });

  /**
   * Given a preset is applied
   * When delete is asked for
   * Then it is not offered and nothing is sent: only a saved custom tactic can be deleted.
   */
  it("offers no delete for a preset or a setup that was never saved", async () => {
    backendWith([], false);
    const { result } = renderLibrary(careerGame);
    await act(async () => {});

    expect(result.current.canDeleteTactic).toBe(false);
    act(() => result.current.handleDeleteTactic());

    expect(callsTo("delete_custom_tactic")).toHaveLength(0);
  });

  /**
   * Given a game installed as career 42
   * When a tactic is made, and the old library is copied in and marked
   * Then every write carries 42, so the backend can refuse it if another career is active.
   */
  it("hands the game's career token back with every write", async () => {
    localStorage.setItem(OLD_KEY, JSON.stringify([oldTactic]));
    backendWith([], true);
    const game = { ...careerGame, career_token: 42 } as unknown as GameStateData;

    const { result } = renderLibrary(game);
    await waitFor(() => expect(callsTo("mark_legacy_tactics_imported")).toHaveLength(1));
    act(() => result.current.handleCreateCustomTactic());
    await waitFor(() => expect(callsTo("save_custom_tactic").length).toBeGreaterThan(1));

    for (const [, args] of [
      ...callsTo("save_custom_tactic"),
      ...callsTo("mark_legacy_tactics_imported"),
    ]) {
      expect(args).toMatchObject({ careerToken: 42 });
    }
    localStorage.clear();
  });
});
