import { useEffect, useState } from "react";

import { getVisibleWage, type VisibleWage } from "../services/scoutingService";

const HIDDEN: VisibleWage = { kind: "hidden" };

/**
 * One player's weekly wage as the club may see it (exact, approximate or hidden), from the backend.
 * It asks again when `revision` changes, keeps the last answer meanwhile, and reads hidden until
 * the first, so an exact figure never flashes on screen.
 */
export function useVisibleWage(playerId: string, revision: string): VisibleWage {
  const [wage, setWage] = useState<{ playerId: string; wage: VisibleWage } | null>(null);

  useEffect(() => {
    let cancelled = false;
    getVisibleWage(playerId)
      .then((answer) => {
        if (!cancelled) setWage({ playerId, wage: answer ?? HIDDEN });
      })
      .catch((error: unknown) => {
        console.error("Failed to read the wage the club may see:", error);
      });
    return () => {
      cancelled = true;
    };
  }, [playerId, revision]);

  // The answer belongs to the player it was asked for: another player's never shows meanwhile.
  return wage?.playerId === playerId ? wage.wage : HIDDEN;
}
