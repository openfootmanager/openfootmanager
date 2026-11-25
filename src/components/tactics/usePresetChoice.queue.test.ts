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

/** The game a saved tactic's answer is: the user's team with the setup it now has. */
const gameWithSetup = (marker: string, setup: { formation: string; playStyle: string }) => ({
  marker,
  teams: [{ id: "t1", formation: setup.formation, play_style: setup.playStyle }],
  manager: { id: "mgr1", team_id: "t1" },
});

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
        // The team the answer reports, as the screen reads it.
        const landed = game as unknown as {
          teams?: Array<{ formation: string; play_style: string }>;
        };
        const team = landed.teams?.[0];
        if (team) setSetup({ formation: team.formation, playStyle: team.play_style });
      },
      onWriteError: vi.fn(),
      onWriteSaved: vi.fn(),
    });
    const presetChoice = usePresetChoice(
      (tactic, keepInstructions) => library.applyTacticSelection(tactic, { keepInstructions }),
      `${setup.formation}|${setup.playStyle}`,
      library.appliesSettled,
    );
    // `externalChange` stands for a change of the setup from outside the tab (an MCP edit, a game
    // refresh): nothing the tab's own applies delivered.
    return { library, presetChoice, externalChange: setSetup };
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
      if (command === "list_custom_tactics") {
        return [
          wire("a", "Away"),
          { ...wire("b", "Home"), formation: "4-3-3", playStyle: "Attacking" },
        ];
      }
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
      replies[1].resolve(gameWithSetup("after b", { formation: "4-3-3", playStyle: "Attacking" })),
    );
    await act(async () => {
      await Promise.all(asked);
    });

    expect(result.current.presetChoice.pending?.id).toBe("preset:wing-play");
    expect(onKeepInstructions).not.toHaveBeenCalled();
    expect(onApplyPreset).not.toHaveBeenCalled();
  });

  /**
   * Given saved tactic A being applied (its answer held), a preset chosen behind it, and then the
   *       setup changed from outside the tab while the choice waits (not by A: nothing the tab's own
   *       applies delivered)
   * When A answers
   * Then the preset choice is dropped, as one whose setup changed under it always is: the newer
   *      formation must not be overwritten by an older choice, and the manager is not asked.
   */
  it("drops a preset choice when the setup changes from outside the tab while it waits", async () => {
    const replies: Array<ReturnType<typeof deferred<unknown>>> = [];
    mockedInvoke.mockImplementation(async (command: string) => {
      if (command === "list_custom_tactics") return [wire("a", "Away")];
      if (command === "legacy_tactics_import_pending") return false;
      if (command === "apply_custom_tactic") {
        const reply = deferred<unknown>();
        replies.push(reply);
        return reply.promise;
      }
      if (command === "get_tactics_blueprint_status") return { modified: true };
      return { modified: false };
    });
    const onKeepInstructions = vi.fn(async () => true);
    const onApplyPreset = vi.fn(async () => true);
    const { result } = mountWithPresetChoice({ onApplyPreset, onKeepInstructions });
    await waitFor(() => expect(customIds({ current: result.current.library })).toEqual(["a"]));
    const entry = (id: string) => {
      const found = result.current.library.tacticLibrary.find((e) => e.id === id);
      if (!found) throw new Error(`${id} is in the library`);
      return found;
    };

    const asked: Array<Promise<void>> = [];
    act(() => {
      asked.push(result.current.library.applyTacticSelection(entry("a")));
      asked.push(result.current.presetChoice.choose(entry("preset:wing-play")));
    });
    await waitFor(() => expect(callsTo("apply_custom_tactic")).toHaveLength(1));
    act(() => result.current.externalChange({ formation: "3-5-2", playStyle: "Possession" }));
    await act(async () => replies[0].resolve({ marker: "after a" }));
    await act(async () => {
      await Promise.all(asked);
    });

    expect(result.current.presetChoice.pending).toBeNull();
    expect(onKeepInstructions).not.toHaveBeenCalled();
    expect(onApplyPreset).not.toHaveBeenCalled();
  });

  /**
   * Given the setup changed from outside the tab BEFORE the manager chose a preset (nothing in the
   *       queue, so no apply is behind it)
   * When the manager chooses the preset
   * Then the choice is asked about as usual: a setup the screen showed before the click is not an
   *      independent change under it, so an older setup must not drop a choice made on the new one.
   */
  it("does not drop a preset chosen after the setup had already changed from outside the tab", async () => {
    mockedInvoke.mockImplementation(async (command: string) => {
      if (command === "list_custom_tactics") return [wire("a", "Away")];
      if (command === "legacy_tactics_import_pending") return false;
      if (command === "get_tactics_blueprint_status") return { modified: true };
      return { modified: false };
    });
    const { result } = mountWithPresetChoice({
      onApplyPreset: vi.fn(async () => true),
      onKeepInstructions: vi.fn(async () => true),
    });
    await waitFor(() => expect(customIds({ current: result.current.library })).toEqual(["a"]));
    const preset = result.current.library.tacticLibrary.find((e) => e.id === "preset:wing-play");
    if (!preset) throw new Error("the preset is in the library");

    act(() => result.current.externalChange({ formation: "3-5-2", playStyle: "Possession" }));
    await act(async () => {
      await result.current.presetChoice.choose(preset);
    });

    expect(result.current.presetChoice.pending?.id).toBe("preset:wing-play");
  });

  /**
   * Given saved tactic A being applied (its answer held), a preset chosen behind it, and A saved
   *       again while it waited, so the backend puts on the team the formation and style A holds NOW,
   *       not the ones the screen had for A when it was asked
   * When A answers
   * Then the preset choice is asked about: the setup the team got is the one A's own apply left, not
   *      an independent change, so the delivered setups are what the answer reports.
   */
  it("keeps a preset choice when the saved tactic's apply delivered a setup it was re-saved to", async () => {
    const reply = deferred<unknown>();
    mockedInvoke.mockImplementation(async (command: string) => {
      if (command === "list_custom_tactics") return [wire("a", "Away")];
      if (command === "legacy_tactics_import_pending") return false;
      if (command === "apply_custom_tactic") return reply.promise;
      if (command === "get_tactics_blueprint_status") return { modified: true };
      return { modified: false };
    });
    const { result } = mountWithPresetChoice({
      onApplyPreset: vi.fn(async () => true),
      onKeepInstructions: vi.fn(async () => true),
    });
    await waitFor(() => expect(customIds({ current: result.current.library })).toEqual(["a"]));
    const entry = (id: string) => {
      const found = result.current.library.tacticLibrary.find((e) => e.id === id);
      if (!found) throw new Error(`${id} is in the library`);
      return found;
    };

    const asked: Array<Promise<void>> = [];
    act(() => {
      asked.push(result.current.library.applyTacticSelection(entry("a")));
      asked.push(result.current.presetChoice.choose(entry("preset:wing-play")));
    });
    await waitFor(() => expect(callsTo("apply_custom_tactic")).toHaveLength(1));
    // The library entry says 4-4-2 / Balanced; the saved tactic now holds 3-5-2 / Possession.
    await act(async () =>
      reply.resolve(gameWithSetup("after a", { formation: "3-5-2", playStyle: "Possession" })),
    );
    await act(async () => {
      await Promise.all(asked);
    });

    expect(result.current.presetChoice.pending?.id).toBe("preset:wing-play");
  });
});
