import { useEffect, useState } from "react";

import { type AutoSetPieces, autoSelectSetPieces } from "../services/squadService";

/**
 * The takers the backend would use for this starting eleven when none are assigned. It asks again
 * when the eleven changes or the squad `revision` does (the backend ranks on attributes, which move
 * with the days while the eleven stays put), and drops a late answer for an older one. `null` until
 * the first answer for this eleven. An older revision's picks stay shown until the new answer lands
 * (as `useFetchedSquad` keeps the previous squad) so the roles do not blink; they are display only,
 * never saved.
 */
export function useAutoSetPieces(playerIds: string[], revision?: unknown): AutoSetPieces | null {
  const [answer, setAnswer] = useState<{ key: string; picks: AutoSetPieces } | null>(null);
  const key = playerIds.join(",");

  useEffect(() => {
    if (key === "") return;
    let cancelled = false;
    autoSelectSetPieces(key.split(","))
      .then((picks) => {
        if (!cancelled) setAnswer({ key, picks });
      })
      .catch((error: unknown) => {
        console.error("Failed to read the automatic set-piece takers:", error);
      });
    return () => {
      cancelled = true;
    };
  }, [key, revision]);

  return answer?.key === key ? answer.picks : null;
}
