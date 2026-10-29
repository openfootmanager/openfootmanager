import { useEffect, useState } from "react";

import { getPlayerKnowledge, type PlayerKnowledge } from "../services/scoutingService";

const UNKNOWN: PlayerKnowledge = { kind: "unknown" };

/** An answer, with the question it answers: these players, at this point of the game. */
interface Answer {
  key: string;
  revision: string;
  knowledge: Record<string, PlayerKnowledge>;
}

/**
 * What the manager's club knows about these players, from the backend's one rule. It asks again
 * when `revision` changes (pass something that moves with the game: the career, the date, the
 * clubs involved), and reads every player as unknown until the answer to the current question
 * arrives, so an answer about another career or another day never shows while the next is on its
 * way or after it fails: hiding and then revealing never flashes what the club should not see.
 */
export function usePlayerKnowledge(
  playerIds: string[],
  revision: string,
): (playerId: string) => PlayerKnowledge {
  const [answer, setAnswer] = useState<Answer | null>(null);
  // JSON, not a joined string: a package's ids may hold any character, a comma included.
  const key = JSON.stringify(playerIds);

  useEffect(() => {
    const ids: string[] = JSON.parse(key);
    if (ids.length === 0) return;
    let cancelled = false;
    getPlayerKnowledge(ids)
      .then((knowledge) => {
        if (!cancelled) setAnswer({ key, revision, knowledge });
      })
      .catch((error: unknown) => {
        console.error("Failed to read what the club knows about these players:", error);
      });
    return () => {
      cancelled = true;
    };
  }, [key, revision]);

  const current = answer?.key === key && answer.revision === revision ? answer.knowledge : {};
  return (playerId) => current[playerId] ?? UNKNOWN;
}
