import { invoke } from "@tauri-apps/api/core";

import type { MatchSnapshot, SubstitutionOutcome, SubstitutionSwap } from "./types";

/**
 * Sends queued swaps to the engine one at a time, in the order they were queued.
 *
 * Each swap is validated by the engine on its own (substitutions left, a player already off, a
 * player sent off), so the first refusal stops the batch: what was applied stays applied and is
 * reported through `onSnapshot` as it happens, and the rest are not attempted. Returns whether
 * the whole batch went through (when the caller may close the substitution screen) or the swap that
 * was refused with the engine's reason, a translation key, for the screen to show.
 */
export async function applySubstitutions(
  side: "Home" | "Away",
  swaps: SubstitutionSwap[],
  onSnapshot: (snapshot: MatchSnapshot) => void,
): Promise<SubstitutionOutcome> {
  for (const [appliedCount, swap] of swaps.entries()) {
    try {
      const snapshot = await invoke<MatchSnapshot>("apply_match_command", {
        command: {
          Substitute: { side, player_off_id: swap.offId, player_on_id: swap.onId },
        },
      });
      onSnapshot(snapshot);
    } catch (err) {
      console.error("Substitution failed:", err);
      return { applied: false, refused: swap, reason: String(err), appliedCount };
    }
  }
  return { applied: true };
}
