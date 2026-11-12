import { useEffect, useRef, useState } from "react";

import { getTacticsBlueprintStatus } from "../../services/squadService";
import { useGameStore } from "../../store/gameStore";
import type { TacticsLibraryEntry } from "./TacticPicker";

/**
 * Choosing a tactic from the picker. A saved tactic applies at once. A preset brings its own
 * instructions, so it asks first, but only when applying it would replace something: when the
 * manager's instructions already equal the preset's blueprint (the backend's rule, asked about the
 * preset's play style) it applies without asking, keeping the instructions (which are the same, so
 * the result is the preset's, and an edit made while the lookup was out survives). A question the backend could not answer asks, so
 * nothing of the manager's is replaced on a guess.
 *
 * Only the latest choice counts: a lookup that answers after the manager chose something else is
 * dropped, and a question left open when the career changes is withdrawn, so an answer is never
 * applied to a choice or a career it was not asked in.
 *
 * `apply(tactic, keepInstructions)` does the writing: `keepInstructions` takes the preset's formation
 * and play style and leaves the manager's instructions alone.
 */
export function usePresetChoice(
  apply: (tactic: TacticsLibraryEntry, keepInstructions: boolean) => Promise<void>,
): {
  /** The preset waiting for the manager's answer, if any. */
  pending: TacticsLibraryEntry | null;
  choose: (tactic: TacticsLibraryEntry) => Promise<void>;
  replaceInstructions: () => Promise<void>;
  keepInstructions: () => Promise<void>;
  cancel: () => void;
} {
  const [pending, setPending] = useState<TacticsLibraryEntry | null>(null);
  const latestChoice = useRef(0);
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
    replaceInstructions: () => answer(false),
    keepInstructions: () => answer(true),
    cancel: () => setPending(null),
  };
}
