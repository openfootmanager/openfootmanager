import { useEffect, useState } from "react";

import { type AutoSetPieces, autoSelectSetPieces } from "../services/squadService";

/**
 * The takers the backend would use for this starting eleven when none are assigned. It asks again
 * when the eleven changes or the squad `revision` does (the backend ranks on attributes, which move
 * with the days while the eleven stays put), and drops a late answer for an older one. `null` until
 * the first answer for the current eleven and revision: an older revision's picks are never returned,
 * since a manual assignment would save them.
 */
export function useAutoSetPieces(playerIds: string[], revision?: unknown): AutoSetPieces | null {
  const [answer, setAnswer] = useState<{
    key: string;
    revision: unknown;
    picks: AutoSetPieces;
  } | null>(null);
  const key = playerIds.join(",");

  useEffect(() => {
    if (key === "") return;
    let cancelled = false;
    autoSelectSetPieces(key.split(","))
      .then((picks) => {
        if (!cancelled) setAnswer({ key, revision, picks });
      })
      .catch((error: unknown) => {
        console.error("Failed to read the automatic set-piece takers:", error);
      });
    return () => {
      cancelled = true;
    };
  }, [key, revision]);

  return answer?.key === key && answer.revision === revision ? answer.picks : null;
}
