import type { MutableRefObject } from "react";

import { type GameStateData, useGameStore } from "../../store/gameStore";
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
  // A saved tactic is put back whole by the backend: its formation and style, and for one that
  // holds them its instructions, eleven, roles and takers (#1189).
  async function applySavedTactic(nextTactic: TacticsLibraryEntry): Promise<boolean> {
    // An answer that lands after the career changed (the game left, another loaded) is the old
    // career's game: it must not replace the one now open, as `changeSave`'s steps don't.
    const askedIn = useGameStore.getState().careerEpoch;
    try {
      const applied = await applyCustomTactic(nextTactic.id.trim(), careerToken);
      if (useGameStore.getState().careerEpoch !== askedIn) return false;
      onGameUpdate(applied);
      return true;
    } catch (error) {
      console.error("Failed to apply the saved tactic:", error);
      onWriteError(error);
      return false;
    }
  }

  async function applyTacticSelection(
    nextTactic: TacticsLibraryEntry,
    options: { keepInstructions?: boolean } = {},
  ): Promise<void> {
    const isPreset = nextTactic.type === "preset";

    // A preset is a whole setup: the backend sets its formation, play style and
    // phase blueprint together (#365), unless the manager chose to keep their own instructions,
    // which takes the formation and style only. A saved tactic is applied whole too.
    const applyPreset = options.keepInstructions ? onKeepInstructions : onApplyPreset;
    const didApply = isPreset
      ? await applyPreset(nextTactic.formation, nextTactic.playStyle)
      : await applySavedTactic(nextTactic);
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

  return applyTacticSelection;
}
