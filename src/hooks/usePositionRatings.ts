import { useEffect, useState } from "react";

import { getPositionRatings } from "../services/positionService";
import type { PositionRatings } from "../store/generated/PositionRatings";

/**
 * One player's ratings at the positions he plays, from the backend's one rule, or null when the
 * club knows nothing of his attributes. It asks again when `revision` changes (see
 * `gameKnowledgeRevision`), and reads null until the answer for this player and this revision
 * arrives, so another player's ratings never show meanwhile or after a failed request.
 */
export function usePositionRatings(playerId: string, revision: string): PositionRatings | null {
  const [answer, setAnswer] = useState<{
    playerId: string;
    revision: string;
    ratings: PositionRatings | null;
  } | null>(null);

  useEffect(() => {
    let cancelled = false;
    getPositionRatings(playerId)
      .then((ratings) => {
        if (!cancelled) setAnswer({ playerId, revision, ratings: ratings ?? null });
      })
      .catch((error: unknown) => {
        console.error("Failed to read the player's position ratings:", error);
      });
    return () => {
      cancelled = true;
    };
  }, [playerId, revision]);

  return answer?.playerId === playerId && answer.revision === revision ? answer.ratings : null;
}
