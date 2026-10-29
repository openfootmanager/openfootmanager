import { useEffect, useState } from "react";

import { getPlayerKnowledge, type PlayerKnowledge } from "../services/scoutingService";

const UNKNOWN: PlayerKnowledge = { kind: "unknown" };

/**
 * What the manager's club knows about these players, from the backend's one rule. It asks again
 * when `revision` changes (pass something that moves with the game: the date, the clubs involved),
 * keeps the last answer while the next is on its way, and reads every player as unknown until the
 * first: hiding and then revealing never flashes what the club should not see.
 */
export function usePlayerKnowledge(
  playerIds: string[],
  revision: string,
): (playerId: string) => PlayerKnowledge {
  const [knowledge, setKnowledge] = useState<Record<string, PlayerKnowledge>>({});
  // JSON, not a joined string: a package's ids may hold any character, a comma included.
  const key = JSON.stringify(playerIds);

  useEffect(() => {
    const ids: string[] = JSON.parse(key);
    if (ids.length === 0) return;
    let cancelled = false;
    getPlayerKnowledge(ids)
      .then((answer) => {
        if (!cancelled) setKnowledge(answer);
      })
      .catch((error: unknown) => {
        console.error("Failed to read what the club knows about these players:", error);
      });
    return () => {
      cancelled = true;
    };
  }, [key, revision]);

  return (playerId) => knowledge[playerId] ?? UNKNOWN;
}
