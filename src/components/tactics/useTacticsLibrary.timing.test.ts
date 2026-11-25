import { act, renderHook, waitFor } from "@testing-library/react";
import { invoke } from "@tauri-apps/api/core";
import { beforeEach, describe, expect, it, vi } from "vitest";

import { type GameStateData, useGameStore } from "../../store/gameStore";
import { useTacticsLibrary } from "./useTacticsLibrary";

vi.mock("@tauri-apps/api/core", () => ({ invoke: vi.fn() }));
// One `t` for every render, as the real hook gives: a new one each render would re-run the effects
// keyed on it, and repair what a test is after.
vi.mock("react-i18next", () => {
  const translation = {
    t: (key: string, fallback?: unknown) => (typeof fallback === "string" ? fallback : key),
    i18n: { language: "en" },
  };
  return {
    initReactI18next: { type: "3rdParty", init: () => {} },
    useTranslation: () => translation,
  };
});

const mockedInvoke = vi.mocked(invoke);

/** A promise settled by hand, so a test decides which reply arrives when. */
function deferred<T>() {
  let resolve: (value: T) => void = () => {};
  let reject: (reason: unknown) => void = () => {};
  const promise = new Promise<T>((res, rej) => {
    resolve = res;
    reject = rej;
  });
  return { promise, resolve, reject };
}

const career = (managerId: string, careerToken = 0) =>
  ({
    clock: { current_date: "2026-08-01", start_date: "2026-08-01" },
    career_token: careerToken,
    teams: [],
    players: [],
    competitions: [],
    manager: { id: managerId, team_id: null },
  }) as unknown as GameStateData;

const wire = (id: string, name = id) => ({
  id,
  name,
  description: "",
  formation: "4-4-2",
  playStyle: "Balanced",
});

// The picker lists a tactic where its name sorts, and a made tactic's name carries its random id, so
// a test compares the set of ids, not their order.
const customIds = (result: { current: ReturnType<typeof useTacticsLibrary> }) =>
  result.current.tacticLibrary.filter((entry) => entry.type === "custom").map((entry) => entry.id);

const callsTo = (command: string) => mockedInvoke.mock.calls.filter(([c]) => c === command);

function mount(initialGame = career("mgr1")) {
  return renderHook(
    ({ game }: { game: GameStateData }) =>
      useTacticsLibrary({
        gameState: game,
        formation: "4-4-2",
        activePlayStyle: "Balanced",
        initialPreset: null,
        onApplyPreset: vi.fn(),
        onKeepInstructions: vi.fn(),
        onGameUpdate: vi.fn(),
        onWriteError: vi.fn(),
        onWriteSaved: vi.fn(),
      }),
    { initialProps: { game: initialGame } },
  );
}

describe("useCustomTacticStore timing (#1438)", () => {
  beforeEach(() => {
    mockedInvoke.mockReset();
    useGameStore.getState().setGameActive(true);
  });

  /**
   * Given two tactics made one after the other, the first one's save still out
   * When the first save's reply arrives (the whole list as the save has it, so without the second)
   * Then the picker still holds the second tactic and it is still the active one: a reply that
   *      predates it must not drop what the manager just made. Its own reply then lists both.
   */
  it("keeps a tactic made after an earlier save's reply was asked for", async () => {
    const replies: Array<ReturnType<typeof deferred<unknown[]>>> = [];
    mockedInvoke.mockImplementation(async (command: string) => {
      if (command === "list_custom_tactics") return [];
      if (command === "legacy_tactics_import_pending") return false;
      if (command === "save_current_tactic") {
        const reply = deferred<unknown[]>();
        replies.push(reply);
        return reply.promise;
      }
      return { modified: false };
    });
    const { result } = mount();
    await waitFor(() => expect(result.current.libraryLoaded).toBe(true));

    act(() => result.current.handleCreateCustomTactic());
    const first = result.current.activeTactic.id;
    await waitFor(() => expect(callsTo("save_current_tactic")).toHaveLength(1));
    act(() => result.current.handleCreateCustomTactic());
    const second = result.current.activeTactic.id;
    expect(second).not.toBe(first);
    expect([...customIds(result)].sort()).toEqual([first, second].sort());

    await act(async () => replies[0].resolve([wire(first)]));

    expect([...customIds(result)].sort()).toEqual([first, second].sort());
    expect(result.current.activeTactic.id).toBe(second);
    await waitFor(() => expect(callsTo("save_current_tactic")).toHaveLength(2));
    await act(async () => replies[1].resolve([wire(second), wire(first)]));
    expect([...customIds(result)].sort()).toEqual([first, second].sort());
    expect(result.current.activeTactic.id).toBe(second);
  });

  /**
   * Given the save's list answered but the question whether the old browser library is still to be
   *       imported was refused
   * When the load settles
   * Then the library shows the tactics the save listed (and Save is on): a failed step after the list
   *      must not leave the player with an empty library.
   */
  it("shows the list it read when the import bookkeeping fails", async () => {
    mockedInvoke.mockImplementation(async (command: string) => {
      if (command === "list_custom_tactics") return [wire("a", "Away")];
      if (command === "legacy_tactics_import_pending") throw new Error("the save is busy");
      return { modified: false };
    });

    const { result } = mount();

    await waitFor(() => expect(result.current.libraryLoaded).toBe(true));
    expect(customIds(result)).toEqual(["a"]);
  });

  /**
   * Given career A's library on screen, and another career opened whose list is still out
   * When the new career's load has not answered
   * Then A's tactics are gone from the picker (they are not this career's), and the new career's
   *      list replaces them when it answers.
   */
  it("does not show the old career's tactics while the new career's load is out", async () => {
    const newList = deferred<unknown[]>();
    let lists = 0;
    mockedInvoke.mockImplementation(async (command: string) => {
      if (command === "list_custom_tactics") {
        lists += 1;
        return lists === 1 ? [wire("a", "Away")] : newList.promise;
      }
      if (command === "legacy_tactics_import_pending") return false;
      return { modified: false };
    });
    const { result, rerender } = mount(career("mgr1"));
    await waitFor(() => expect(customIds(result)).toEqual(["a"]));

    rerender({ game: career("mgr2") });
    act(() => useGameStore.getState().setGameActive(true));

    await waitFor(() => expect(callsTo("list_custom_tactics")).toHaveLength(2));
    expect(customIds(result)).toEqual([]);
    await act(async () => newList.resolve([wire("b", "Home")]));
    expect(customIds(result)).toEqual(["b"]);
  });

  /**
   * Given a library on screen, and the same career reloaded under a new career token (its list out)
   * When the reload has not answered
   * Then the tactics stay on screen: it is the same career's library, and emptying it would drop the
   *      active tactic to a preset for the length of the call.
   */
  it("keeps the library on screen while the same career reloads under a new token", async () => {
    const reload = deferred<unknown[]>();
    let lists = 0;
    mockedInvoke.mockImplementation(async (command: string) => {
      if (command === "list_custom_tactics") {
        lists += 1;
        return lists === 1 ? [wire("a", "Away")] : reload.promise;
      }
      if (command === "legacy_tactics_import_pending") return false;
      return { modified: false };
    });
    const { result, rerender } = mount(career("mgr1", 0));
    await waitFor(() => expect(customIds(result)).toEqual(["a"]));

    rerender({ game: career("mgr1", 1) });

    await waitFor(() => expect(callsTo("list_custom_tactics")).toHaveLength(2));
    expect(customIds(result)).toEqual(["a"]);
    await act(async () => reload.resolve([wire("a", "Away")]));
    expect(customIds(result)).toEqual(["a"]);
  });
});
