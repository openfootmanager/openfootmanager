import { useEffect, useState } from "react";

import {
  getCustomTacticDifferences,
  type TacticDifferences,
} from "../../services/customTacticsService";

function anyDifference(differences: TacticDifferences): boolean {
  return (
    differences.formation ||
    differences.playStyle ||
    differences.instructions ||
    differences.eleven ||
    differences.roles ||
    differences.takers
  );
}

/**
 * Whether the team's tactic differs from the saved tactic `savedId` in anything it holds (its
 * formation, style, instructions, eleven, roles or takers), as the backend says: the rule is
 * `tactic_differences` in ofm_core. Asked again whenever the team's tactic (`teamTacticKey`) or the
 * library (`library`, which changes when a save lands) changes; a late answer to an older question is
 * dropped. No saved tactic or no club: nothing differs. A question the backend could not answer
 * counts as a difference: the screen must not call the tactic synced (and switch Update off) on a
 * guess, while a Save that changes nothing is harmless.
 */
export function useSavedTacticDiffers(
  savedId: string | null,
  hasClub: boolean,
  teamTacticKey: string,
  library: unknown,
): boolean {
  const [differs, setDiffers] = useState(false);

  useEffect(() => {
    if (!savedId || !hasClub) {
      setDiffers(false);
      return;
    }

    let cancelled = false;
    getCustomTacticDifferences(savedId)
      .then((differences) => {
        if (!cancelled) setDiffers(Boolean(differences) && anyDifference(differences));
      })
      .catch(() => {
        if (!cancelled) setDiffers(true);
      });
    return () => {
      cancelled = true;
    };
  }, [savedId, hasClub, teamTacticKey, library]);

  return differs;
}
