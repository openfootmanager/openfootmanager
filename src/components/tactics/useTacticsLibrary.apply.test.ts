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

function mount(
  initialGame = career("mgr1"),
  callbacks: {
    onGameUpdate?: (game: GameStateData) => void;
    onWriteError?: (error: unknown) => void;
  } = {},
) {
  return renderHook(
    ({ game }: { game: GameStateData }) =>
      useTacticsLibrary({
        gameState: game,
        formation: "4-4-2",
        activePlayStyle: "Balanced",
        initialPreset: null,
        onApplyPreset: vi.fn(),
        onKeepInstructions: vi.fn(),
        onGameUpdate: callbacks.onGameUpdate ?? vi.fn(),
        onWriteError: callbacks.onWriteError ?? vi.fn(),
        onWriteSaved: vi.fn(),
      }),
    { initialProps: { game: initialGame } },
  );
}

describe("useTacticsLibrary delete and apply timing (#1438)", () => {
  beforeEach(() => {
    mockedInvoke.mockReset();
    useGameStore.getState().setGameActive(true);
  });

  const emptyLibrary = (extra: (command: string, args?: unknown) => unknown = () => undefined) =>
    mockedInvoke.mockImplementation(async (command: string, args?: unknown) => {
      const handled = extra(command, args);
      if (handled !== undefined) return handled;
      if (command === "list_custom_tactics") return [];
      if (command === "legacy_tactics_import_pending") return false;
      return { modified: false };
    });

  /**
   * Given a saved tactic active (the setup also matches a preset) and a delete the save refuses
   * When the refusal arrives
   * Then the library has the tactic back and it is the active one again: the optimistic removal let the
   *      selection fall back to the preset, and the refusal must not leave it there.
   */
  it("makes a tactic active again when its delete is refused", async () => {
    const deleteReply = deferred<unknown[]>();
    // A save that keeps what it is given, as the real one does: a refused delete leaves the tactic in
    // the list the screen then asks for.
    const kept: unknown[] = [];
    emptyLibrary((command, args) => {
      if (command === "save_current_tactic") {
        kept.push((args as { tactic: unknown }).tactic);
        return [...kept];
      }
      if (command === "list_custom_tactics") return [...kept];
      if (command === "delete_custom_tactic") return deleteReply.promise;
      return undefined;
    });
    const { result } = mount();
    await waitFor(() => expect(result.current.libraryLoaded).toBe(true));
    act(() => result.current.handleCreateCustomTactic());
    const id = result.current.activeTactic.id;
    await waitFor(() => expect(callsTo("save_current_tactic")).toHaveLength(1));
    await waitFor(() => expect(result.current.canDeleteTactic).toBe(true));

    act(() => result.current.handleDeleteTactic());
    await waitFor(() => expect(callsTo("delete_custom_tactic")).toHaveLength(1));
    await act(async () => deleteReply.reject("be.error.customTactic.notFound"));

    await waitFor(() => expect(customIds(result)).toEqual([id]));
    expect(result.current.activeTactic.id).toBe(id);
  });

  /**
   * Given a saved tactic active (the setup also matches a preset) and a delete the save accepts
   * When the save has answered
   * Then the selection falls back to the preset the setup matches: only a refused delete keeps the tactic.
   */
  it("falls back to the matching preset once a delete is accepted", async () => {
    const kept: unknown[] = [];
    emptyLibrary((command, args) => {
      if (command === "save_current_tactic") {
        kept.push((args as { tactic: unknown }).tactic);
        return [...kept];
      }
      if (command === "delete_custom_tactic") {
        kept.length = 0;
        return [];
      }
      return undefined;
    });
    const { result } = mount();
    await waitFor(() => expect(result.current.libraryLoaded).toBe(true));
    act(() => result.current.handleCreateCustomTactic());
    await waitFor(() => expect(result.current.canDeleteTactic).toBe(true));

    act(() => result.current.handleDeleteTactic());

    await waitFor(() => expect(callsTo("delete_custom_tactic")).toHaveLength(1));
    await waitFor(() => expect(result.current.activeTactic.id).toBe("preset:balanced-control"));
    expect(customIds(result)).toEqual([]);
  });

  /**
   * Given a saved tactic being applied, and another career opened before the apply answers
   * When the apply is refused
   * Then no error is reported: it is the old career's, and the screen's alert belongs to the open one.
   */
  it("does not report a failed apply once its career has been replaced", async () => {
    const applyReply = deferred<unknown>();
    const onWriteError = vi.fn();
    emptyLibrary((command) => {
      if (command === "list_custom_tactics") return [wire("a", "Away")];
      if (command === "apply_custom_tactic") return applyReply.promise;
      return undefined;
    });
    const { result } = mount(career("mgr1"), { onWriteError });
    await waitFor(() => expect(customIds(result)).toEqual(["a"]));
    const saved = result.current.tacticLibrary.find((entry) => entry.id === "a");
    if (!saved) throw new Error("the saved tactic is in the library");

    let applying: Promise<void> = Promise.resolve();
    act(() => {
      applying = result.current.applyTacticSelection(saved);
    });
    await waitFor(() => expect(callsTo("apply_custom_tactic")).toHaveLength(1));
    act(() => useGameStore.getState().setGameActive(true));
    await act(async () => {
      applyReply.reject("be.error.customTactic.notFound");
      await applying;
    });

    expect(onWriteError).not.toHaveBeenCalled();
  });

  /**
   * Given a failed apply in the career that is still open
   * When the apply is refused
   * Then the error is reported (the guard above must not silence the open career's own failures).
   */
  it("reports a failed apply in the open career", async () => {
    const onWriteError = vi.fn();
    emptyLibrary((command) => {
      if (command === "list_custom_tactics") return [wire("a", "Away")];
      if (command === "apply_custom_tactic") throw "be.error.customTactic.notFound";
      return undefined;
    });
    const { result } = mount(career("mgr1"), { onWriteError });
    await waitFor(() => expect(customIds(result)).toEqual(["a"]));
    const saved = result.current.tacticLibrary.find((entry) => entry.id === "a");
    if (!saved) throw new Error("the saved tactic is in the library");
    vi.spyOn(console, "error").mockImplementation(() => {});

    await act(async () => result.current.applyTacticSelection(saved));

    expect(onWriteError).toHaveBeenCalledWith("be.error.customTactic.notFound");
  });

  /**
   * Given two saved tactics picked one after the other, the first one's reply still out
   * When the first answers, and then the second
   * Then the second is not asked of the save until the first has answered, and the screen takes the
   *      games in the order they were asked: the later answer is the game as it ends.
   */
  it("applies saved tactics one at a time, in the order they were asked", async () => {
    const replies: Array<ReturnType<typeof deferred<unknown>>> = [];
    const onGameUpdate = vi.fn();
    emptyLibrary((command) => {
      if (command === "list_custom_tactics") return [wire("a", "Away"), wire("b", "Home")];
      if (command === "apply_custom_tactic") {
        const reply = deferred<unknown>();
        replies.push(reply);
        return reply.promise;
      }
      return undefined;
    });
    const { result } = mount(career("mgr1"), { onGameUpdate });
    await waitFor(() => expect([...customIds(result)].sort()).toEqual(["a", "b"]));
    const entry = (id: string) => {
      const found = result.current.tacticLibrary.find((e) => e.id === id);
      if (!found) throw new Error("the saved tactic is in the library");
      return found;
    };

    let first: Promise<void> = Promise.resolve();
    let second: Promise<void> = Promise.resolve();
    act(() => {
      first = result.current.applyTacticSelection(entry("a"));
      second = result.current.applyTacticSelection(entry("b"));
    });
    await waitFor(() => expect(callsTo("apply_custom_tactic")).toHaveLength(1));
    await act(async () => {});
    expect(callsTo("apply_custom_tactic")).toHaveLength(1);

    await act(async () => replies[0].resolve({ marker: "game after a" }));
    await waitFor(() => expect(callsTo("apply_custom_tactic")).toHaveLength(2));
    await act(async () => replies[1].resolve({ marker: "game after b" }));
    await act(async () => {
      await Promise.all([first, second]);
    });

    expect(onGameUpdate.mock.calls.map(([game]) => (game as { marker: string }).marker)).toEqual([
      "game after a",
      "game after b",
    ]);
    expect(result.current.activeTactic.id).toBe("b");
  });
});
