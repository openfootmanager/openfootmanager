import { type MutableRefObject, useRef, useState } from "react";

import { type GameStateData, useGameStore } from "../../store/gameStore";
import { createWriteQueue } from "../../lib/writeQueue";
import { applyCustomTactic } from "../../services/customTacticsService";
import type { TacticsLibraryEntry } from "./TacticPicker";
import { setupKey } from "./TacticsTab.helpers";

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
  // The setups the applies in the queue have left (a tactic carries its formation and play style),
  // so a choice waiting behind them can tell their changes of the setup from an independent one. Kept
  // only while the queue is busy: once it is idle, nothing asked before is still to land.
  const delivered = useRef(new Set<string>());
  const inQueue = useRef(0);

  // The setup the team has in `game`, as the screen reads it (`useTacticsLineup`), or null when the
  // game does not say (no club).
  function setupOfUserTeam(game: GameStateData): string | null {
    const team = game.teams?.find((candidate) => candidate.id === game.manager?.team_id);
    return team ? setupKey(team.formation, team.play_style || "Balanced") : null;
  }

  // One entry in the queue, counted while it waits and runs: the setups the applies have delivered
  // are forgotten once the queue is idle, since nothing asked before is still to land.
  function track<T>(task: () => Promise<T>): Promise<T> {
    inQueue.current += 1;
    return enqueue(task).finally(() => {
      inQueue.current -= 1;
      if (inQueue.current === 0) delivered.current.clear();
    });
  }

  // A saved tactic is put back whole by the backend: its formation and style, and for one that
  // holds them its instructions, eleven, roles and takers (#1189).
  // Resolves to the setup the team got, or null when the apply did not land. The backend applies the
  // tactic as it holds it when the apply runs, which is not always what the screen had for it when it
  // was asked (a re-save in between), so the answer is what says what was delivered.
  async function sendSavedTactic(
    nextTactic: TacticsLibraryEntry,
    askedIn: number,
  ): Promise<string | null> {
    try {
      const applied = await applyCustomTactic(nextTactic.id.trim(), careerToken);
      // An answer that lands after the career changed (the game left, another loaded) is the old
      // career's game: it must not replace the one now open, as `changeSave`'s steps don't.
      if (useGameStore.getState().careerEpoch !== askedIn) return null;
      onGameUpdate(applied);
      return setupOfUserTeam(applied) ?? setupKey(nextTactic.formation, nextTactic.playStyle);
    } catch (error) {
      console.error("Failed to apply the saved tactic:", error);
      // A refusal that lands after the career changed is the old career's: the screen's alert is
      // the open career's, as for the success path above.
      if (useGameStore.getState().careerEpoch === askedIn) onWriteError(error);
      return null;
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
    const didApply = await track(async () => {
      if (stale()) return false;
      if (isPreset) {
        const landed = await applyPreset(nextTactic.formation, nextTactic.playStyle);
        if (landed) delivered.current.add(setupKey(nextTactic.formation, nextTactic.playStyle));
        return landed && !stale();
      }
      const setup = await sendSavedTactic(nextTactic, askedIn);
      if (setup !== null) delivered.current.add(setup);
      return setup !== null;
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
   * Resolves once every apply asked so far has landed (or been refused), with the setups they left:
   * what a decision about the team's instructions has to wait for, since a saved tactic changes them
   * when it lands.
   */
  const appliesSettled = (): Promise<ReadonlySet<string>> =>
    track(async () => new Set(delivered.current));

  /**
   * Runs the manager's own write (a formation or play style edit) after every apply asked so far, in
   * the one queue: sent at once it could reach the backend before an apply still waiting, which would
   * then overwrite it, and the last thing the manager did must be what stands. Not sent if the career
   * changed by its turn.
   */
  function runAfterApplies<T>(write: () => Promise<T>): Promise<T | undefined> {
    const askedIn = useGameStore.getState().careerEpoch;
    return track(async () =>
      useGameStore.getState().careerEpoch === askedIn ? write() : undefined,
    );
  }

  return { applyTacticSelection, appliesSettled, runAfterApplies };
}
