import { useEffect, useMemo, useRef, useState } from "react";

import { getCustomTacticDifferences } from "../../services/customTacticsService";
import type { TacticDifferences } from "../../store/generated/TacticDifferences";

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
 * dropped.
 *
 * No saved tactic or no club: nothing differs. While a question is pending, or when the backend
 * could not answer it, the tactic counts as differing: the screen must never call it synced (and
 * switch Update and Reset off) on a guess, while a Save that changes nothing is harmless.
 */
export function useSavedTacticDiffers(
  savedId: string | null,
  hasClub: boolean,
  teamTacticKey: string,
  library: unknown,
): boolean {
  const [answer, setAnswer] = useState<{ key: string; differs: boolean } | null>(null);
  // The library changes by identity when a save lands; a counter per identity puts that in the key.
  const libraryCount = useRef(0);
  const libraryRevision = useMemo(() => {
    libraryCount.current += 1;
    return libraryCount.current;
  }, [library]);
  const asking = savedId !== null && hasClub;
  const question = `${savedId}|${teamTacticKey}|${libraryRevision}`;

  useEffect(() => {
    if (!asking || !savedId) return;

    let cancelled = false;
    getCustomTacticDifferences(savedId)
      .then((differences) => {
        if (!cancelled) {
          setAnswer({ key: question, differs: Boolean(differences) && anyDifference(differences) });
        }
      })
      .catch(() => {
        if (!cancelled) setAnswer({ key: question, differs: true });
      });
    return () => {
      cancelled = true;
    };
  }, [asking, savedId, question]);

  if (!asking) return false;
  return answer?.key === question ? answer.differs : true;
}
