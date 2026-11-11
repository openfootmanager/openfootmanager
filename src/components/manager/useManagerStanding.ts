import { useEffect, useState } from "react";

import { getManagerStanding } from "../../services/managersService";
import type { GameStateData } from "../../store/gameStore";
import type { ManagerStanding } from "../../store/generated/ManagerStanding";

/**
 * The board's and the fans' view of the user's manager, as the backend words it. Asked again
 * whenever the game changes; null until it answers.
 */
export function useManagerStanding(gameState: GameStateData): ManagerStanding | null {
  const [standing, setStanding] = useState<ManagerStanding | null>(null);

  useEffect(() => {
    let cancelled = false;
    void (async () => {
      try {
        const answer = await getManagerStanding();
        if (!cancelled) setStanding(answer);
      } catch (error: unknown) {
        console.error("Failed to fetch the manager's standing", error);
      }
    })();
    return () => {
      cancelled = true;
    };
  }, [gameState]);

  return standing;
}
