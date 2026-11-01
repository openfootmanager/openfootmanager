import { useEffect, useState } from "react";

import { getPotentialOpinion } from "../services/scoutingService";
import type { PotentialOpinion } from "../store/generated/PotentialOpinion";

/**
 * The club's opinion of one player's potential, from the backend's one rule: a range, never the
 * number, or null when the club holds none. It asks again when `revision` changes (see
 * `gameKnowledgeRevision`), and reads null until the answer to the current question arrives, so
 * another player's or another club's opinion never shows meanwhile or after a failed request.
 */
export function usePotentialOpinion(playerId: string, revision: string): PotentialOpinion | null {
  const [answer, setAnswer] = useState<{
    playerId: string;
    revision: string;
    opinion: PotentialOpinion | null;
  } | null>(null);

  useEffect(() => {
    let cancelled = false;
    getPotentialOpinion(playerId)
      .then((opinion) => {
        if (!cancelled) setAnswer({ playerId, revision, opinion });
      })
      .catch((error: unknown) => {
        console.error("Failed to read the club's opinion of the player's potential:", error);
      });
    return () => {
      cancelled = true;
    };
  }, [playerId, revision]);

  return answer?.playerId === playerId && answer.revision === revision ? answer.opinion : null;
}
