import { useEffect, useState } from "react";

import { getSquadFitness, type SquadFitness } from "../services/squadService";

/**
 * How fit the user's squad is, from the backend (the one summary the staff's warnings use). It asks
 * again when `revision` (the game) changes and keeps the last answer meanwhile; `null` until the
 * first answer, and after a failed one (the last answer may belong to another career).
 */
export function useSquadFitness(revision?: unknown): SquadFitness | null {
  const [fitness, setFitness] = useState<SquadFitness | null>(null);

  useEffect(() => {
    let cancelled = false;
    getSquadFitness()
      .then((answer) => {
        if (!cancelled) setFitness(answer);
      })
      .catch((error: unknown) => {
        console.error("Failed to read the squad fitness:", error);
        if (!cancelled) setFitness(null);
      });
    return () => {
      cancelled = true;
    };
  }, [revision]);

  return fitness;
}
