import { type MutableRefObject, useState } from "react";

import { type GameStateData, useGameStore } from "../../store/gameStore";
import { createWriteQueue } from "../../lib/writeQueue";
import { applyCustomTactic } from "../../services/customTacticsService";
import type { TacticsLibraryEntry } from "./TacticPicker";

interface UseApplyTacticArgs {
  onApplyPreset: (formation: string, playStyle: string) => Promise<boolean>;
  /** A preset's formation and play style only: the manager's own instructions stay as they are. */
  onKeepInstructions: (formation: string, playStyle: string) => Promise<boolean>;
  onGameUpdate: (game: GameStateData) => void;
  onWriteError: (error: unknown) => void;
  /** Handed back with the write so the backend can refuse one meant for another career. */
  careerToken: number;
  latestCustomTactics: MutableRefObject<TacticsLibraryEntry[]>;
  setActiveTacticId: (id: string) => void;
  setDraftTacticName: (name: string) => void;
  setPresetAnchorId: (id: string) => void;
}

/** Puts a tactic from the picker on the team (a preset whole, or a saved tactic), then makes it the active one. */
export function useApplyTactic({
  onApplyPreset,
  onKeepInstructions,
  onGameUpdate,
  onWriteError,
  careerToken,
  latestCustomTactics,
  setActiveTacticId,
  setDraftTacticName,
  setPresetAnchorId,
}: UseApplyTacticArgs) {
  // Presets and saved tactics are applied through one queue, one at a time, in the order they were
  // asked. Each answer is the whole game, so two applies in flight whose replies arrive out of order
  // would leave the screen showing the earlier one's game while the save holds the later one's, and
  // a preset chosen after a saved tactic that is still waiting would be overwritten by it.
  const [enqueue] = useState(createWriteQueue);

  // A saved tactic is put back whole by the backend: its formation and style, and for one that
  // holds them its instructions, eleven, roles and takers (#1189).
  async function sendSavedTactic(
    nextTactic: TacticsLibraryEntry,
    askedIn: number,
  ): Promise<boolean> {
    try {
      const applied = await applyCustomTactic(nextTactic.id.trim(), careerToken);
      // An answer that lands after the career changed (the game left, another loaded) is the old
      // career's game: it must not replace the one now open, as `changeSave`'s steps don't.
      if (useGameStore.getState().careerEpoch !== askedIn) return false;
      onGameUpdate(applied);
      return true;
    } catch (error) {
      console.error("Failed to apply the saved tactic:", error);
      // A refusal that lands after the career changed is the old career's: the screen's alert is
      // the open career's, as for the success path above.
      if (useGameStore.getState().careerEpoch === askedIn) onWriteError(error);
      return false;
    }
  }

  async function applyTacticSelection(
    nextTactic: TacticsLibraryEntry,
    options: { keepInstructions?: boolean } = {},
  ): Promise<void> {
    const isPreset = nextTactic.type === "preset";
    // The career it is asked in, taken now and not when its turn comes: a turn that arrives after
    // the career changed is the old career's, and sends nothing.
    const askedIn = useGameStore.getState().careerEpoch;
    const stale = () => useGameStore.getState().careerEpoch !== askedIn;

    // A preset is a whole setup: the backend sets its formation, play style and
    // phase blueprint together (#365), unless the manager chose to keep their own instructions,
    // which takes the formation and style only. A saved tactic is applied whole too.
    const applyPreset = options.keepInstructions ? onKeepInstructions : onApplyPreset;
    const didApply = await enqueue(async () => {
      if (stale()) return false;
      if (!isPreset) return sendSavedTactic(nextTactic, askedIn);
      const applied = await applyPreset(nextTactic.formation, nextTactic.playStyle);
      return applied && !stale();
    });
    if (!didApply) {
      return;
    }

    // The name as the library has it now: a rename that landed while this was applying stands.
    const latest = latestCustomTactics.current.find((entry) => entry.id === nextTactic.id);
    setActiveTacticId(nextTactic.id);
    setDraftTacticName((latest ?? nextTactic).name);

    if (isPreset) {
      setPresetAnchorId(nextTactic.id.replace("preset:", ""));
    }
  }

  /**
   * Resolves once every apply asked so far has landed (or been refused): what a decision about the
   * team's instructions has to wait for, since a saved tactic changes them when it lands.
   */
  const appliesSettled = (): Promise<void> => enqueue(async () => undefined);

  return { applyTacticSelection, appliesSettled };
}
