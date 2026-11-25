import { act, renderHook, waitFor } from "@testing-library/react";
import { invoke } from "@tauri-apps/api/core";
import { beforeEach, describe, expect, it, vi } from "vitest";

import { type GameStateData, useGameStore } from "../../store/gameStore";
import { usePresetChoice } from "./usePresetChoice";
import { useTacticsLibrary } from "./useTacticsLibrary";

/**
 * The bar is busy while a write to the tactic is out, and a career change reopens it: an operation of
 * the career that has left is not the open career's, whether it completes or not, and when it does
 * complete it must not clear an operation the new career has started since.
 */

vi.mock("@tauri-apps/api/core", () => ({ invoke: vi.fn() }));
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
  const promise = new Promise<T>((res) => {
    resolve = res;
  });
  return { promise, resolve };
}

const career = {
  clock: { current_date: "2026-08-01", start_date: "2026-08-01" },
  career_token: 0,
  teams: [],
  players: [],
  competitions: [],
  manager: { id: "mgr1", team_id: null },
} as unknown as GameStateData;

const wire = (id: string) => ({
  id,
  name: id,
  description: "",
  formation: "4-4-2",
  playStyle: "Balanced",
});

const callsTo = (command: string) => mockedInvoke.mock.calls.filter(([c]) => c === command);

/** The career changes: another game is loaded. */
const changeCareer = () =>
  act(() => useGameStore.setState((state) => ({ careerEpoch: state.careerEpoch + 1 })));

function mount() {
  return renderHook(() => {
    const library = useTacticsLibrary({
      gameState: career,
      formation: "4-4-2",
      activePlayStyle: "Defensive",
      initialPreset: null,
      onApplyPreset: vi.fn(async () => true),
      onKeepInstructions: vi.fn(async () => true),
      onGameUpdate: vi.fn(),
      onWriteError: vi.fn(),
      onWriteSaved: vi.fn(),
    });
    const presetChoice = usePresetChoice(
      (tactic, keepInstructions) => library.applyTacticSelection(tactic, { keepInstructions }),
      "4-4-2|Defensive",
      library.runAfterApplies,
    );
    return { library, presetChoice };
  });
}

describe("the tactic bar's busy state across careers", () => {
  beforeEach(() => {
    mockedInvoke.mockReset();
    useGameStore.getState().setGameActive(true);
  });

  const entry = (
    result: { current: ReturnType<typeof mount>["result"]["current"] },
    id: string,
  ) => {
    const found = result.current.library.tacticLibrary.find((e) => e.id === id);
    if (!found) throw new Error(`${id} is in the library`);
    return found;
  };

  /**
   * Given a saved tactic A being applied (its reply held) in one career
   * When the career changes, a saved tactic B is applied in the new one (its reply held), and A's
   *      reply finally arrives
   * Then the bar is open as soon as the career changes, busy again while B is out, still busy when
   *      A's reply arrives (A cannot clear B), and open once B's reply arrives.
   */
  it("reopens when the career changes, and an old apply's completion cannot clear a new one", async () => {
    const replies: Array<ReturnType<typeof deferred<unknown>>> = [];
    mockedInvoke.mockImplementation(async (command: string) => {
      if (command === "list_custom_tactics") return [wire("a"), wire("b")];
      if (command === "legacy_tactics_import_pending") return false;
      if (command === "apply_custom_tactic") {
        const reply = deferred<unknown>();
        replies.push(reply);
        return reply.promise;
      }
      return { modified: false };
    });
    const { result } = mount();
    await waitFor(() => expect(result.current.library.libraryLoaded).toBe(true));

    act(() => void result.current.library.applyTacticSelection(entry(result, "a")));
    await waitFor(() => expect(callsTo("apply_custom_tactic")).toHaveLength(1));
    expect(result.current.library.isTacticWriteOut).toBe(true);

    changeCareer();
    expect(result.current.library.isTacticWriteOut).toBe(false);
    act(() => void result.current.library.applyTacticSelection(entry(result, "b")));
    expect(result.current.library.isTacticWriteOut).toBe(true);
    await act(async () => replies[0].resolve({ marker: "a" }));
    // A is done; B's turn comes and its request goes out. A's completion did not clear B.
    await waitFor(() => expect(callsTo("apply_custom_tactic")).toHaveLength(2));
    expect(result.current.library.isTacticWriteOut).toBe(true);
    await act(async () => replies[1].resolve({ marker: "b" }));

    await waitFor(() => expect(result.current.library.isTacticWriteOut).toBe(false));
  });

  /**
   * Given a delete the save has not answered, in one career
   * When the career changes, and then the save answers
   * Then the bar is open as soon as the career changes and stays open.
   */
  it("reopens when the career changes while a delete is out", async () => {
    const deleteReply = deferred<unknown[]>();
    const kept: unknown[] = [];
    mockedInvoke.mockImplementation(async (command: string, args?: unknown) => {
      if (command === "save_current_tactic") {
        kept.push((args as { tactic: unknown }).tactic);
        return [...kept];
      }
      if (command === "list_custom_tactics") return [...kept];
      if (command === "legacy_tactics_import_pending") return false;
      if (command === "delete_custom_tactic") return deleteReply.promise;
      return { modified: false };
    });
    const { result } = mount();
    await waitFor(() => expect(result.current.library.libraryLoaded).toBe(true));
    act(() => result.current.library.handleCreateCustomTactic());
    await waitFor(() => expect(result.current.library.canDeleteTactic).toBe(true));
    act(() => result.current.library.handleDeleteTactic());
    await waitFor(() => expect(callsTo("delete_custom_tactic")).toHaveLength(1));
    expect(result.current.library.isTacticWriteOut).toBe(true);

    changeCareer();
    expect(result.current.library.isTacticWriteOut).toBe(false);
    await act(async () => deleteReply.resolve([]));

    expect(result.current.library.isTacticWriteOut).toBe(false);
  });

  /**
   * Given a preset's lookup held by the backend, in one career
   * When the career changes, and then the lookup answers
   * Then the bar is open as soon as the career changes and stays open.
   */
  it("reopens when the career changes while a preset's lookup is out", async () => {
    const lookup = deferred<unknown>();
    mockedInvoke.mockImplementation(async (command: string) => {
      if (command === "list_custom_tactics") return [];
      if (command === "legacy_tactics_import_pending") return false;
      if (command === "get_tactics_blueprint_status") return lookup.promise;
      return { modified: false };
    });
    const { result } = mount();
    await waitFor(() => expect(result.current.library.libraryLoaded).toBe(true));

    act(() => void result.current.presetChoice.choose(entry(result, "preset:wing-play")));
    await waitFor(() => expect(callsTo("get_tactics_blueprint_status")).toHaveLength(1));
    expect(result.current.presetChoice.lookingUp).toBe(true);

    changeCareer();
    expect(result.current.presetChoice.lookingUp).toBe(false);
    await act(async () => lookup.resolve({ modified: false }));

    expect(result.current.presetChoice.lookingUp).toBe(false);
  });
});
