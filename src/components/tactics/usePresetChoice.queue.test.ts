import { act, renderHook, waitFor } from "@testing-library/react";
import { useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import { beforeEach, describe, expect, it, vi } from "vitest";

import { type GameStateData, useGameStore } from "../../store/gameStore";
import { usePresetChoice } from "./usePresetChoice";
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

function mountWithPresetChoice(callbacks: {
  onApplyPreset: (formation: string, playStyle: string) => Promise<boolean>;
  onKeepInstructions: (formation: string, playStyle: string) => Promise<boolean>;
}) {
  return renderHook(() => {
    // The team's setup, which a saved tactic changes when it lands (as the game update it brings does).
    const [setup, setSetup] = useState({ formation: "4-4-2", playStyle: "Defensive" });
    const library = useTacticsLibrary({
      gameState: career("mgr1"),
      formation: setup.formation,
      activePlayStyle: setup.playStyle,
      initialPreset: null,
      onApplyPreset: callbacks.onApplyPreset,
      onKeepInstructions: callbacks.onKeepInstructions,
      onGameUpdate: (game) => {
        const landed = game as unknown as { setup?: { formation: string; playStyle: string } };
        if (landed.setup) setSetup(landed.setup);
      },
      onWriteError: vi.fn(),
      onWriteSaved: vi.fn(),
    });
    const presetChoice = usePresetChoice(
      (tactic, keepInstructions) => library.applyTacticSelection(tactic, { keepInstructions }),
      `${setup.formation}|${setup.playStyle}`,
      library.appliesSettled,
    );
    return { library, presetChoice };
  });
}

describe("a preset chosen while saved tactics are being applied (#1447)", () => {
  beforeEach(() => {
    mockedInvoke.mockReset();
    useGameStore.getState().setGameActive(true);
  });

  /**
   * Given saved tactic A being applied (its answer held), saved tactic B asked behind it (B changes
   *       the team's instructions), and then a preset chosen whose blueprint the instructions match
   *       only as they are NOW, before B lands
   * When A and then B have answered
   * Then the preset is judged against the instructions B left (modified now, so the manager is asked)
   *      and is not applied keeping B's instructions without asking: the decision is made after the
   *      earlier applies have landed, not at the click. B also moves the team to another formation and
   *      style, as a saved tactic does; the choice is not dropped as stale for that.
   */
  it("asks about instructions an earlier queued apply changed, instead of keeping them silently", async () => {
    const replies: Array<ReturnType<typeof deferred<unknown>>> = [];
    let instructionsDifferFromBlueprint = false;
    mockedInvoke.mockImplementation(async (command: string, args?: unknown) => {
      if (command === "list_custom_tactics") return [wire("a", "Away"), wire("b", "Home")];
      if (command === "legacy_tactics_import_pending") return false;
      if (command === "apply_custom_tactic") {
        const reply = deferred<unknown>();
        replies.push(reply);
        // Tactic b brings its own instructions: once it has landed, they differ from the preset's.
        void reply.promise.then(() => {
          if ((args as { id: string }).id === "b") instructionsDifferFromBlueprint = true;
        });
        return reply.promise;
      }
      // The backend decides when it is asked, against the instructions it holds at that moment.
      if (command === "get_tactics_blueprint_status") {
        return { modified: instructionsDifferFromBlueprint };
      }
      return { modified: false };
    });
    const onKeepInstructions = vi.fn(async () => true);
    const onApplyPreset = vi.fn(async () => true);
    const { result } = mountWithPresetChoice({ onApplyPreset, onKeepInstructions });
    await waitFor(() =>
      expect([...customIds({ current: result.current.library })].sort()).toEqual(["a", "b"]),
    );
    const entry = (id: string) => {
      const found = result.current.library.tacticLibrary.find((e) => e.id === id);
      if (!found) throw new Error(`${id} is in the library`);
      return found;
    };

    const asked: Array<Promise<void>> = [];
    act(() => {
      asked.push(result.current.library.applyTacticSelection(entry("a")));
      asked.push(result.current.library.applyTacticSelection(entry("b")));
      asked.push(result.current.presetChoice.choose(entry("preset:wing-play")));
    });
    await waitFor(() => expect(callsTo("apply_custom_tactic")).toHaveLength(1));
    await act(async () => replies[0].resolve({ marker: "after a" }));
    await waitFor(() => expect(callsTo("apply_custom_tactic")).toHaveLength(2));
    await act(async () =>
      replies[1].resolve({
        marker: "after b",
        setup: { formation: "4-3-3", playStyle: "Attacking" },
      }),
    );
    await act(async () => {
      await Promise.all(asked);
    });

    expect(result.current.presetChoice.pending?.id).toBe("preset:wing-play");
    expect(onKeepInstructions).not.toHaveBeenCalled();
    expect(onApplyPreset).not.toHaveBeenCalled();
  });
});
