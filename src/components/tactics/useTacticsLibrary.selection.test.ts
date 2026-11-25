import { act, renderHook, waitFor } from "@testing-library/react";
import { invoke } from "@tauri-apps/api/core";
import { describe, expect, it, vi } from "vitest";

import { type GameStateData, useGameStore } from "../../store/gameStore";
import { useTacticsLibrary } from "./useTacticsLibrary";

/**
 * Characterization of `useTacticsLibrary` as it stands, written before the hook is split by
 * responsibility (#1412): each case pins a behaviour a mutation of the hook showed nothing else pinned.
 * They must pass on the hook before and after the split.
 */

vi.mock("@tauri-apps/api/core", () => ({ invoke: vi.fn() }));
// One `t` for every render, as the real hook gives (see useTacticsLibrary.rename.test.ts). It shows what
// it was given: the key, then the values it was asked to put in ("tactics.copyOfTactic|Balanced").
vi.mock("react-i18next", () => {
  const translation = {
    t: (key: string, options?: unknown) => {
      if (typeof options === "string") return options;
      if (options && typeof options === "object") {
        return `${key}|${Object.values(options as Record<string, unknown>).join(",")}`;
      }
      return key;
    },
    i18n: { language: "en" },
  };
  return {
    initReactI18next: { type: "3rdParty", init: () => {} },
    useTranslation: () => translation,
  };
});

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

const listCalls = () => mockedInvoke.mock.calls.filter(([c]) => c === "list_custom_tactics");

describe("useTacticsLibrary loading, characterized", () => {
  const otherCareer = {
    ...careerGame,
    manager: { id: "mgr2", team_id: null },
  } as unknown as GameStateData;
  const sameCareerNewToken = { ...careerGame, career_token: 1 } as unknown as GameStateData;

  /** The hook over a game a test can replace, with one save write held until the test releases it. */
  async function mountWithAHeldWrite() {
    const saveReply = deferred<unknown[]>();
    mockedInvoke.mockReset();
    mockedInvoke.mockImplementation(async (command: string) => {
      if (command === "list_custom_tactics") return [];
      if (command === "save_current_tactic") return saveReply.promise;
      return { modified: false };
    });
    const hook = renderHook(
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
      { initialProps: { game: careerGame } },
    );
    await waitFor(() => expect(listCalls()).toHaveLength(1));
    await waitFor(() => expect(hook.result.current.libraryLoaded).toBe(true));
    act(() => hook.result.current.handleCreateCustomTactic());
    await waitFor(() =>
      expect(mockedInvoke.mock.calls.some(([c]) => c === "save_current_tactic")).toBe(true),
    );
    return { ...hook, release: () => act(async () => saveReply.resolve([])) };
  }

  /**
   * Given a write still out, and the game's career token changing behind it (the same career, a new
   *       token)
   * When the write answers
   * Then the library is asked for again, and is loaded once that answer is in: a queued reload of the
   *      career that is still open runs.
   */
  it("runs a queued reload when only the career token changed", async () => {
    const { result, rerender, release } = await mountWithAHeldWrite();

    rerender({ game: sameCareerNewToken });
    expect(result.current.libraryLoaded).toBe(false);
    await release();

    await waitFor(() => expect(listCalls()).toHaveLength(2));
    await waitFor(() => expect(result.current.libraryLoaded).toBe(true));
  });

  /**
   * Given a write still out, a second load of the same career queued behind it (the game's career
   *       token changed), and then another career replacing the open one
   * When the write answers
   * Then the queued load of the replaced career never asks the save for its list: it was asked in a
   *      career that is no longer the open one. Only the new career's own load does, and it is that
   *      load whose settling marks the library loaded (the hook turns "loaded" on only for its
   *      latest asking), so the one call after the release is the open career's.
   */
  it("does not run a queued load once its career has been replaced", async () => {
    const { result, rerender, release } = await mountWithAHeldWrite();

    rerender({ game: sameCareerNewToken });
    rerender({ game: otherCareer });
    act(() => useGameStore.getState().setGameActive(true));
    expect(result.current.libraryLoaded).toBe(false);
    await release();

    await waitFor(() => expect(result.current.libraryLoaded).toBe(true));
    await act(async () => {});
    expect(listCalls()).toHaveLength(2);
  });
});

describe("useTacticsLibrary selection, characterized", () => {
  /**
   * Given a setup that started as the Balanced Control preset (4-4-2, Balanced), a tactic made from
   *       it and then deleted, and the play style since changed to one no preset has at 4-4-2
   * When the manager looks at Reset
   * Then it is offered (the setup differs from the preset it started from only in its play style),
   *      and using it applies that preset.
   */
  it("offers Reset against the preset a setup started from when only its play style differs", async () => {
    emptySave();
    const { result, rerender, onApplyPreset } = mountLibrary({
      formation: "4-4-2",
      playStyle: "Balanced",
    });
    await waitFor(() => expect(result.current.libraryLoaded).toBe(true));
    act(() => result.current.handleCreateCustomTactic());
    await waitFor(() => expect(result.current.canDeleteTactic).toBe(true));
    rerender({ formation: "4-4-2", playStyle: "Defensive" });
    await deleteTheActiveTactic(result);
    await act(async () => {});

    // The unsaved setup now stands in, and it is not changed from itself: the badge is clean, so the
    // only reason left to offer Reset is that it differs from the preset it started from.
    expect(result.current.activeTactic.id).toBe("current:setup");
    expect(result.current.isCommandBarDirty).toBe(false);
    expect(result.current.canResetTactic).toBe(true);
    await act(async () => {
      await result.current.handleResetTactic();
    });

    expect(onApplyPreset).toHaveBeenCalledWith("4-4-2", "Balanced");
  });

  type Props = { formation: string; playStyle: string; game?: GameStateData };

  /** The hook as the Tactics tab mounts it, over a game and a setup a test can change. */
  function mountLibrary(initialProps: Props, onApplyPreset = vi.fn().mockResolvedValue(true)) {
    const hook = renderHook(
      ({ formation, playStyle, game }: Props) =>
        useTacticsLibrary({
          gameState: game ?? careerGame,
          formation,
          activePlayStyle: playStyle,
          initialPreset: null,
          onApplyPreset,
          onKeepInstructions: vi.fn(),
          onGameUpdate: vi.fn(),
          onWriteError: vi.fn(),
          onWriteSaved: vi.fn(),
        }),
      { initialProps },
    );
    return { ...hook, onApplyPreset };
  }

  /**
   * A save that starts empty and keeps what it is given, as the real one does: a write answers with the
   * whole list, so a tactic just made is in the library the hook shows next.
   */
  function emptySave() {
    const held: Array<{ id: string }> = [];
    mockedInvoke.mockReset();
    mockedInvoke.mockImplementation(async (command: string, args?: unknown) => {
      if (command === "list_custom_tactics") return structuredClone(held);
      if (command === "save_current_tactic") {
        const tactic = (args as { tactic: { id: string } }).tactic;
        const at = held.findIndex((entry) => entry.id === tactic.id);
        if (at === -1) held.push(tactic);
        else held[at] = tactic;
        return structuredClone(held);
      }
      if (command === "delete_custom_tactic") {
        const { id } = args as { id: string };
        const at = held.findIndex((entry) => entry.id === id);
        // As the real save does: a tactic it does not hold is refused, not "the last one" removed.
        if (at === -1) throw "be.error.customTactic.notFound";
        held.splice(at, 1);
        return structuredClone(held);
      }
      return { modified: false };
    });
  }

  /**
   * Deletes the saved tactic that is active, and waits for the save to have been asked to delete
   * that very tactic: removing it from the picker alone (the optimistic step) does not count.
   */
  async function deleteTheActiveTactic(result: { current: ReturnType<typeof useTacticsLibrary> }) {
    const id = result.current.activeTactic.id;
    act(() => result.current.handleDeleteTactic());
    await waitFor(() =>
      expect(
        mockedInvoke.mock.calls.some(
          ([command, args]) =>
            command === "delete_custom_tactic" && (args as { id?: string } | undefined)?.id === id,
        ),
      ).toBe(true),
    );
    await waitFor(() => expect(result.current.canDeleteTactic).toBe(false));
  }

  const savedNames = (name: (typeof mockedInvoke.mock.calls)[number]) =>
    (name[1] as { tactic?: { name?: string } } | undefined)?.tactic?.name;

  /**
   * Given a setup that started as the Balanced Control preset, saved as a tactic without the manager
   *       having typed a name
   * When the tactic is saved
   * Then it is named after the preset (the draft name follows the active tactic; the mocked `t` shows
   *      the key it was given), not the generic name.
   */
  it("names a tactic saved from a preset after that preset", async () => {
    emptySave();
    const { result } = mountLibrary({ formation: "4-4-2", playStyle: "Balanced" });
    await waitFor(() => expect(result.current.libraryLoaded).toBe(true));

    await act(async () => {
      await result.current.handleSaveTactic();
    });

    const saves = mockedInvoke.mock.calls.filter(([c]) => c === "save_current_tactic");
    expect(saves.map(savedNames)).toEqual(["tactics.presetNames.balanced-control"]);
  });

  /**
   * Given a library with no tactics
   * When two tactics are made, one after the other
   * Then they are numbered 1 and 2: each count is the library's size plus one.
   */
  it("numbers the tactics it makes by how many the library holds", async () => {
    emptySave();
    const { result } = mountLibrary({ formation: "4-4-2", playStyle: "Defensive" });
    await waitFor(() => expect(result.current.libraryLoaded).toBe(true));

    act(() => result.current.handleCreateCustomTactic());
    act(() => result.current.handleCreateCustomTactic());

    const names = result.current.tacticLibrary
      .filter((entry) => entry.type === "custom")
      .map((entry) => entry.name);
    expect(names.sort()).toEqual(["tactics.customTacticNumber|1", "tactics.customTacticNumber|2"]);
  });

  /**
   * Given the Balanced Control preset active
   * When it is duplicated
   * Then the copy is named after the tactic it copies ("copy of Balanced Control"), not the generic name.
   */
  it("names a duplicate after the tactic it copies", async () => {
    emptySave();
    const { result } = mountLibrary({ formation: "4-4-2", playStyle: "Balanced" });
    await waitFor(() => expect(result.current.libraryLoaded).toBe(true));

    act(() => result.current.handleDuplicateTactic());

    const copies = result.current.tacticLibrary.filter((entry) => entry.type === "custom");
    expect(copies.map((entry) => entry.name)).toEqual([
      "tactics.copyOfTactic|tactics.presetNames.balanced-control",
    ]);
  });

  /**
   * Given a setup that matches no preset (4-4-2 Defensive), a preset applied (the screen's setup
   *       becomes the preset's), the setup then edited away from it, a tactic made and then deleted
   * When the manager looks at Reset
   * Then it is offered against the preset that was applied, and using it applies that preset: an
   *      applied preset is remembered as the setup's anchor.
   */
  it("remembers an applied preset as the one Reset goes back to", async () => {
    emptySave();
    // The screen's callback publishes the game, so the setup becomes the preset's: the host does the same.
    let host: (props: Props) => void = () => {};
    const onApplyPreset = vi.fn(async (formation: string, playStyle: string) => {
      host({ formation, playStyle });
      return true;
    });
    const { result, rerender } = mountLibrary(
      { formation: "4-4-2", playStyle: "Defensive" },
      onApplyPreset,
    );
    host = rerender;
    await waitFor(() => expect(result.current.libraryLoaded).toBe(true));
    const wingPlay = result.current.tacticLibrary.find((entry) => entry.id === "preset:wing-play");
    if (!wingPlay) throw new Error("the wing play preset is in the library");
    await act(async () => {
      await result.current.applyTacticSelection(wingPlay);
    });
    expect(onApplyPreset).toHaveBeenCalledWith("4-3-3", "Attacking");
    // Then the manager edits the setup away from it (a play style no preset has at 4-3-3).
    rerender({ formation: "4-3-3", playStyle: "Defensive" });
    act(() => result.current.handleCreateCustomTactic());
    await waitFor(() => expect(result.current.canDeleteTactic).toBe(true));
    await deleteTheActiveTactic(result);

    expect(result.current.activeTactic.id).toBe("current:setup");
    expect(result.current.canResetTactic).toBe(true);
    onApplyPreset.mockClear();
    await act(async () => {
      await result.current.handleResetTactic();
    });

    expect(onApplyPreset).toHaveBeenCalledWith("4-3-3", "Attacking");
  });

  /**
   * Given a saved tactic applied, and the backend saying the dials differ from the style's blueprint
   *       but the team's tactic equals the saved one
   * When the badge is read
   * Then it does not read as changed: a saved tactic is compared with what was saved, and the
   *      blueprint is a preset's yardstick, not its.
   */
  it("does not read a saved tactic's badge against the style's blueprint", async () => {
    const club = {
      ...careerGame,
      manager: { id: "mgr1", team_id: "t1" },
      teams: [{ id: "t1", formation: "4-4-2", play_style: "Balanced", tactics_phase: {} }],
    } as unknown as GameStateData;
    mockedInvoke.mockReset();
    mockedInvoke.mockImplementation(async (command: string) => {
      if (command === "list_custom_tactics") {
        return [{ id: "a", name: "A", description: "", formation: "4-4-2", playStyle: "Balanced" }];
      }
      if (command === "get_tactics_blueprint_status") return { modified: true };
      if (command === "get_custom_tactic_differences") {
        return {
          formation: false,
          playStyle: false,
          instructions: false,
          eleven: false,
          roles: false,
          takers: false,
        };
      }
      if (command === "apply_custom_tactic") return {};
      return [];
    });
    const { result } = mountLibrary({ formation: "4-4-2", playStyle: "Balanced", game: club });
    await waitFor(() => expect(result.current.tacticLibrary.some((e) => e.id === "a")).toBe(true));
    const saved = result.current.tacticLibrary.find((entry) => entry.id === "a");
    if (!saved) throw new Error("the saved tactic is in the library");
    await act(async () => {
      await result.current.applyTacticSelection(saved);
    });
    await waitFor(() => expect(result.current.activeTactic.id).toBe("a"));
    await waitFor(() =>
      expect(mockedInvoke.mock.calls.some(([c]) => c === "get_custom_tactic_differences")).toBe(
        true,
      ),
    );
    await act(async () => {});

    expect(result.current.isCommandBarDirty).toBe(false);
  });

  const deleteCalls = () => mockedInvoke.mock.calls.filter(([c]) => c === "delete_custom_tactic");
  const saveCalls = () => mockedInvoke.mock.calls.filter(([c]) => c === "save_current_tactic");

  /**
   * Given a preset active, with nothing saved
   * When a delete is asked of the hook anyway
   * Then nothing is sent to the save: only a tactic the save holds can be deleted.
   */
  it("sends no delete for a preset", async () => {
    emptySave();
    const { result } = mountLibrary({ formation: "4-4-2", playStyle: "Balanced" });
    await waitFor(() => expect(result.current.libraryLoaded).toBe(true));
    expect(result.current.canDeleteTactic).toBe(false);

    act(() => result.current.handleDeleteTactic());
    await act(async () => {});

    expect(deleteCalls()).toHaveLength(0);
  });

  /**
   * Given a setup that matches no preset and was never saved (the unsaved setup stands in as a custom
   *       tactic)
   * When the manager looks at Delete and Rename
   * Then both are off: the save does not hold it.
   */
  it("offers neither delete nor rename for an unsaved setup", async () => {
    emptySave();
    const { result } = mountLibrary({ formation: "4-4-2", playStyle: "Defensive" });
    await waitFor(() => expect(result.current.libraryLoaded).toBe(true));

    expect(result.current.activeTactic.type).toBe("custom");
    expect(result.current.canDeleteTactic).toBe(false);
    expect(result.current.canRenameTactic).toBe(false);
  });

  /**
   * Given a tactic made and active
   * When the manager saves
   * Then that tactic is updated (the same id is sent, and the library still holds one), not a second
   *      one made.
   */
  it("saving a saved tactic updates it instead of making another", async () => {
    emptySave();
    const { result } = mountLibrary({ formation: "4-4-2", playStyle: "Defensive" });
    await waitFor(() => expect(result.current.libraryLoaded).toBe(true));
    act(() => result.current.handleCreateCustomTactic());
    await waitFor(() => expect(saveCalls()).toHaveLength(1));
    const madeId = result.current.activeTactic.id;

    await act(async () => {
      await result.current.handleSaveTactic();
    });

    await waitFor(() => expect(saveCalls()).toHaveLength(2));
    const sentIds = saveCalls().map(([, args]) => (args as { tactic: { id: string } }).tactic.id);
    expect(sentIds).toEqual([madeId, madeId]);
    expect(result.current.tacticLibrary.filter((entry) => entry.type === "custom")).toHaveLength(1);
  });

  /**
   * Given a preset active
   * When it is saved as a tactic
   * Then the tactic made is the active one, named after the preset.
   */
  it("makes the tactic saved from a preset the active one", async () => {
    emptySave();
    const { result } = mountLibrary({ formation: "4-4-2", playStyle: "Balanced" });
    await waitFor(() => expect(result.current.libraryLoaded).toBe(true));
    expect(result.current.activeTactic.type).toBe("preset");

    await act(async () => {
      await result.current.handleSaveTactic();
    });

    expect(result.current.activeTactic.type).toBe("custom");
    expect(result.current.activeTactic.name).toBe("tactics.presetNames.balanced-control");
  });

  /**
   * Given a saved tactic whose name arrives blank from the save (an old browser library can hold one)
   * When it is applied and saved
   * Then it is saved under the generic name, not blank.
   */
  it("saves a tactic whose name is blank under the generic name", async () => {
    mockedInvoke.mockReset();
    mockedInvoke.mockImplementation(async (command: string) => {
      if (command === "list_custom_tactics") {
        return [
          { id: "a", name: "  ", description: "", formation: "4-4-2", playStyle: "Defensive" },
        ];
      }
      if (command === "save_current_tactic") return [];
      if (command === "apply_custom_tactic") return {};
      return { modified: false };
    });
    const { result } = mountLibrary({ formation: "4-4-2", playStyle: "Defensive" });
    await waitFor(() => expect(result.current.tacticLibrary.some((e) => e.id === "a")).toBe(true));
    const blank = result.current.tacticLibrary.find((entry) => entry.id === "a");
    if (!blank) throw new Error("the saved tactic is in the library");
    await act(async () => {
      await result.current.applyTacticSelection(blank);
    });
    await waitFor(() => expect(result.current.activeTactic.id).toBe("a"));

    await act(async () => {
      await result.current.handleSaveTactic();
    });

    expect(saveCalls().map((call) => savedNames(call))).toEqual(["tactics.customTactic"]);
  });
});
