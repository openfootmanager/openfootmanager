import { useEffect, useRef, useState } from "react";

import type { TacticsLibraryEntry } from "./TacticPicker";

const SAVE_CUE_DURATION_MS = 2000;

/**
 * The "Saved" cue on the tactic bar's Save button: it shows for a moment after a save the backend
 * accepted, and only for the tactic and the save it confirms. An answer that lands after the manager
 * moved on (another tactic, a newer save) must not confirm the wrong one.
 */
export function useSavedCue(
  activeTactic: Pick<TacticsLibraryEntry, "id" | "type">,
  onSave: () => Promise<boolean>,
): { showSavedCue: boolean; save: () => Promise<void> } {
  const [showSavedCue, setShowSavedCue] = useState(false);
  const timeoutRef = useRef<ReturnType<typeof setTimeout> | null>(null);
  const activeTacticIdRef = useRef(activeTactic.id);
  activeTacticIdRef.current = activeTactic.id;
  const latestSaveRef = useRef(0);

  useEffect(() => {
    return () => {
      if (timeoutRef.current) clearTimeout(timeoutRef.current);
    };
  }, []);

  async function save(): Promise<void> {
    // The old cue belongs to the previous save: it goes while this one is pending.
    setShowSavedCue(false);
    if (timeoutRef.current) clearTimeout(timeoutRef.current);
    const thisSave = ++latestSaveRef.current;
    const savedTacticId = activeTactic.id;
    // Saving a preset makes a new custom tactic and the bar moves to it; saving a custom one keeps
    // it active, so a different active tactic after the answer means the manager moved away.
    const createsATactic = activeTactic.type !== "custom";
    const saved = await onSave();
    if (!saved || thisSave !== latestSaveRef.current) return;
    if (!createsATactic && activeTacticIdRef.current !== savedTacticId) return;
    setShowSavedCue(true);
    timeoutRef.current = setTimeout(() => setShowSavedCue(false), SAVE_CUE_DURATION_MS);
  }

  return { showSavedCue, save };
}
