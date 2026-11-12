import { act, renderHook, waitFor } from "@testing-library/react";
import { invoke } from "@tauri-apps/api/core";
import { describe, expect, it, vi } from "vitest";

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

describe("useTacticsLibrary renaming a saved tactic", () => {
  const saved = (id: string, name: string) => ({
    id,
    name,
    description: "",
    formation: "4-4-2",
    playStyle: "Balanced",
  });
  const renameCalls = () =>
    mockedInvoke.mock.calls.filter(([command]) => command === "rename_custom_tactic");
  const saveCalls = () =>
    mockedInvoke.mock.calls.filter(([command]) => command === "save_current_tactic");

  function renderWith(onWriteError = vi.fn(), onWriteSaved = vi.fn()) {
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
          onWriteError,
          onWriteSaved,
        }),
      { initialProps: { game: careerGame } },
    );
  }

  /**
   * Given a save of a new tactic still out
   * When a rename is asked
   * Then it waits behind that save (one queue for every library write) and is sent when it has
   *      answered, and the library then holds the new name.
   */
  it("waits behind an earlier library write", async () => {
    const saveReply = deferred<unknown[]>();
    mockedInvoke.mockReset();
    mockedInvoke.mockImplementation(async (command: string) => {
      if (command === "list_custom_tactics") return [saved("a", "A")];
      if (command === "save_current_tactic") return saveReply.promise;
      if (command === "rename_custom_tactic") return [saved("a", "Away")];
      return { modified: false };
    });
    const { result } = renderWith();
    await waitFor(() => expect(customIds(result)).toEqual(["a"]));

    act(() => result.current.handleCreateCustomTactic());
    await waitFor(() => expect(saveCalls()).toHaveLength(1));
    let answer: string | null | undefined;
    act(() => {
      void result.current.handleRenameTactic("a", "Away").then((reason) => {
        answer = reason;
      });
    });
    await act(async () => {});
    expect(renameCalls()).toHaveLength(0);
    await act(async () => saveReply.resolve([saved("a", "A")]));

    await waitFor(() => expect(renameCalls()).toHaveLength(1));
    await waitFor(() => expect(answer).toBeNull());
    expect(result.current.tacticLibrary.find((entry) => entry.id === "a")?.name).toBe("Away");
  });

  /**
   * Given a rename queued behind the first load
   * When the career changes before the load answers
   * Then no rename is sent: it was asked in a career that is no longer the open one.
   */
  it("sends no rename once the career has changed", async () => {
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
    const { result, rerender } = renderWith();
    await waitFor(() => expect(lists).toBe(1));
    let answer: string | null | undefined;
    act(() => {
      void result.current.handleRenameTactic("a", "Away").then((reason) => {
        answer = reason;
      });
    });

    rerender({ game: otherCareer });
    act(() => useGameStore.getState().setGameActive(true));
    await act(async () => firstList.resolve([saved("a", "A")]));

    await waitFor(() => expect(answer).toBeNull());
    expect(renameCalls()).toHaveLength(0);
  });

  /**
   * Given a rename sent and its reply still out
   * When the career changes and then the reply arrives
   * Then it does not replace the new career's library and the screen is not told a save was accepted.
   */
  it("ignores a rename's reply that arrives after the career changed", async () => {
    const renameReply = deferred<unknown[]>();
    let lists = 0;
    mockedInvoke.mockReset();
    mockedInvoke.mockImplementation(async (command: string) => {
      if (command === "list_custom_tactics") {
        lists += 1;
        return lists === 1 ? [saved("a", "A")] : [saved("mine", "Mine")];
      }
      if (command === "rename_custom_tactic") return renameReply.promise;
      return { modified: false };
    });
    const otherCareer = {
      ...careerGame,
      manager: { id: "mgr2", team_id: null },
    } as unknown as GameStateData;
    const onWriteSaved = vi.fn();
    const { result, rerender } = renderWith(vi.fn(), onWriteSaved);
    await waitFor(() => expect(customIds(result)).toEqual(["a"]));
    act(() => {
      void result.current.handleRenameTactic("a", "Away");
    });
    await waitFor(() => expect(renameCalls()).toHaveLength(1));

    rerender({ game: otherCareer });
    act(() => useGameStore.getState().setGameActive(true));
    await waitFor(() => expect(customIds(result)).toEqual(["mine"]));
    await act(async () => renameReply.resolve([saved("a", "Away")]));

    expect(customIds(result)).toEqual(["mine"]);
    expect(onWriteSaved).not.toHaveBeenCalled();
  });

  /**
   * Given a save that refuses the new name
   * When the rename is asked
   * Then the reason comes back for the dialog to show, and the screen's own alert is not raised.
   */
  it("hands a refusal back to the dialog, not to the screen's alert", async () => {
    mockedInvoke.mockReset();
    mockedInvoke.mockImplementation(async (command: string) => {
      if (command === "list_custom_tactics") return [saved("a", "A")];
      if (command === "rename_custom_tactic") throw "be.error.customTactic.nameRequired";
      return { modified: false };
    });
    const onWriteError = vi.fn();
    const { result } = renderWith(onWriteError);
    await waitFor(() => expect(customIds(result)).toEqual(["a"]));

    let reason: string | null = null;
    await act(async () => {
      reason = await result.current.handleRenameTactic("a", "   ");
    });

    expect(reason).toEqual(expect.any(String));
    expect(onWriteError).not.toHaveBeenCalled();
    expect(result.current.tacticLibrary.find((entry) => entry.id === "a")?.name).toBe("A");
  });

  /**
   * Given a saved tactic being applied (Reset), its answer still out
   * When the tactic is renamed and the rename lands, and then the apply answers
   * Then the draft takes the new name from the library, not the one the apply started with: the
   *      tactic is not marked as changed (Update would undo the rename).
   */
  it("keeps a rename that lands while the tactic is being applied", async () => {
    const applyReply = deferred<unknown>();
    let renamed = false;
    mockedInvoke.mockReset();
    mockedInvoke.mockImplementation(async (command: string) => {
      if (command === "list_custom_tactics") return [saved("a", renamed ? "Away" : "A")];
      if (command === "apply_custom_tactic") return applyReply.promise;
      if (command === "rename_custom_tactic") {
        renamed = true;
        return [saved("a", "Away")];
      }
      return { modified: false };
    });
    const { result } = renderWith();
    await waitFor(() => expect(customIds(result)).toEqual(["a"]));
    const original = result.current.tacticLibrary.find((entry) => entry.id === "a");
    if (!original) throw new Error("the saved tactic is in the library");

    act(() => {
      void result.current.applyTacticSelection(original);
    });
    await waitFor(() =>
      expect(mockedInvoke).toHaveBeenCalledWith("apply_custom_tactic", expect.anything()),
    );
    await act(async () => {
      await result.current.handleRenameTactic("a", "Away");
    });
    await act(async () => applyReply.resolve({}));

    await waitFor(() => expect(result.current.activeTactic.id).toBe("a"));
    expect(result.current.activeTactic.name).toBe("Away");
    expect(result.current.isCommandBarDirty).toBe(false);
  });
});
