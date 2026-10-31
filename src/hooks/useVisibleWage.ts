import { useEffect, useState } from "react";

import { getVisibleWage, type VisibleWage } from "../services/scoutingService";

const HIDDEN: VisibleWage = { kind: "hidden" };

/**
 * One player's weekly wage as the club may see it (exact, approximate or hidden), from the backend.
 * It asks again when `revision` changes, and reads hidden until the answer to the current question
 * arrives, so an exact figure from another player, career or day never flashes on screen, while
 * the next answer is on its way or after it fails.
 */
export function useVisibleWage(playerId: string, revision: string): VisibleWage {
  const [wage, setWage] = useState<{
    playerId: string;
    revision: string;
    wage: VisibleWage;
  } | null>(null);

  useEffect(() => {
    let cancelled = false;
    getVisibleWage(playerId)
      .then((answer) => {
        if (!cancelled) setWage({ playerId, revision, wage: answer ?? HIDDEN });
      })
      .catch((error: unknown) => {
        console.error("Failed to read the wage the club may see:", error);
      });
    return () => {
      cancelled = true;
    };
  }, [playerId, revision]);

  // The answer belongs to the question it was asked for: another player's never shows meanwhile.
  return wage?.playerId === playerId && wage.revision === revision ? wage.wage : HIDDEN;
}
