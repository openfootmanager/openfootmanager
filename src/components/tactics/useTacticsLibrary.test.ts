import { act, renderHook, waitFor } from "@testing-library/react";
import { invoke } from "@tauri-apps/api/core";
import { beforeEach, describe, expect, it, vi } from "vitest";

import type { GameStateData } from "../../store/gameStore";
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
   * Given a tactic waiting behind the first load, and the player leaves the screen (another game
   *       is about to be loaded)
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
    await act(async () => list.resolve([]));

    expect(saveCalls()).toHaveLength(0);
  });

  /**
   * Given a tactic waiting behind the first load, and the career then changes while the screen
   *       stays open
   * When the first load finishes
   * Then the waiting tactic is not saved into the new career, which is loaded on its own.
   */
  it("does not save a queued tactic into the career that replaced its own", async () => {
    const firstList = deferred<unknown[]>();
    let lists = 0;
    mockedInvoke.mockReset();
    mockedInvoke.mockImplementation(async (command: string) => {
      if (command === "list_custom_tactics") {
        lists += 1;
        return lists === 1 ? firstList.promise : [];
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
    await act(async () => firstList.resolve([]));

    await waitFor(() => expect(lists).toBe(2));
    expect(saveCalls()).toHaveLength(0);
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
      return { modified: false };
    });
    const { unmount } = renderLibrary(careerGame);

    unmount();
    await act(async () => list.resolve([]));

    expect(saveCalls()).toHaveLength(0);
    localStorage.clear();
  });
});
