import { useEffect, useRef, useState } from "react";

import { getTacticsBlueprintStatus } from "../../services/squadService";
import { useGameStore } from "../../store/gameStore";
import type { TacticsLibraryEntry } from "./TacticPicker";

/**
 * Choosing a tactic from the picker. A saved tactic applies at once. A preset brings its own
 * instructions, so it asks first, but only when applying it would replace something: when the
 * manager's instructions already equal the preset's blueprint (the backend's rule, asked about the
 * preset's play style) it applies without asking, keeping the instructions (which are the same, so
 * the result is the preset's, and an edit made while the lookup was out survives). A question the
 * backend could not answer asks, so nothing of the manager's is replaced on a guess.
 *
 * A lookup is out for a moment, and an answer is only ever applied to the choice, the career and the
 * setup it was asked about. Every path that can write the formation, the play style or the
 * instructions while one is out ends it, and each is tested:
 * - choosing another tactic (a preset or a saved one) takes a newer generation in `choose`;
 * - the manager's own edit of the formation or the play style, and Reset, call `invalidate` when
 *   they are made (the screen's one entry point for each);
 * - anything else that changes the formation or play style under the tab (an MCP edit, a game
 *   refresh, a saved tactic applied elsewhere) changes `setupKey`, which is compared when the lookup
 *   answers;
 * - the career changing (compared when the lookup answers, and the epoch effect), and the tab
 *   unmounting.
 * An edit of the instructions alone does not end it: the preset then applies keeping them.
 *
 * `apply(tactic, keepInstructions)` does the writing: `keepInstructions` takes the preset's formation
 * and play style and leaves the manager's instructions alone.
 */
export function usePresetChoice(
  apply: (tactic: TacticsLibraryEntry, keepInstructions: boolean) => Promise<void>,
  /** The formation and play style the team has now: a change under a pending lookup makes it stale. */
  setupKey: string,
): {
  /** The preset waiting for the manager's answer, if any. */
  pending: TacticsLibraryEntry | null;
  choose: (tactic: TacticsLibraryEntry) => Promise<void>;
  /** The manager changed the setup a preset writes (the formation, the play style, a Reset): a lookup still out is stale. */
  invalidate: () => void;
  replaceInstructions: () => Promise<void>;
  keepInstructions: () => Promise<void>;
  cancel: () => void;
} {
  const [pending, setPending] = useState<TacticsLibraryEntry | null>(null);
  const latestChoice = useRef(0);
  // The setup as of the latest render, read when a lookup answers.
  const currentSetup = useRef(setupKey);
  currentSetup.current = setupKey;
  const careerEpoch = useGameStore((state) => state.careerEpoch);

  useEffect(() => {
    latestChoice.current += 1;
    setPending(null);
  }, [careerEpoch]);

  // The choice goes with the tab: one still waiting for its lookup when the tab unmounts is dropped.
  useEffect(
    () => () => {
      latestChoice.current += 1;
    },
    [],
  );

  async function choose(tactic: TacticsLibraryEntry): Promise<void> {
    const thisChoice = ++latestChoice.current;
    // Writes carry no career of their own, so a choice is also dropped if the career changed while
    // its lookup was out, even though the tab stayed mounted.
    const askedIn = useGameStore.getState().careerEpoch;
    const askedSetup = currentSetup.current;
    setPending(null);
    if (tactic.type !== "preset") return apply(tactic, false);
    let wouldReplace = true;
    try {
      // An answer that is not an object is a stand-in with nothing to say: nothing to ask about.
      const status = await getTacticsBlueprintStatus(tactic.playStyle);
      wouldReplace = Boolean(status?.modified);
    } catch {
      wouldReplace = true;
    }
    if (thisChoice !== latestChoice.current) return;
    if (useGameStore.getState().careerEpoch !== askedIn) return;
    if (currentSetup.current !== askedSetup) return;
    // Nothing to ask: the instructions already are the preset's. Kept all the same, so an edit the
    // manager commits between the lookup and this write is not wiped by a whole-preset apply.
    if (wouldReplace) setPending(tactic);
    else await apply(tactic, true);
  }

  async function answer(keepInstructions: boolean): Promise<void> {
    const tactic = pending;
    setPending(null);
    if (tactic) await apply(tactic, keepInstructions);
  }

  return {
    pending,
    choose,
    invalidate: () => {
      latestChoice.current += 1;
    },
    replaceInstructions: () => answer(false),
    keepInstructions: () => answer(true),
    cancel: () => setPending(null),
  };
}
