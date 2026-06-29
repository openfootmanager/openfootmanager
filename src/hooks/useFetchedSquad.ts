import { useEffect, useState } from "react";
import type { Dispatch, SetStateAction } from "react";

import { getSquad } from "../services/squadService";
import type { PlayerData } from "../store/gameStore";

/**
 * Fetches a team's squad and refetches whenever the team OR the game clock
 * changes, so per-day fields (condition, fitness, injuries) refresh after a day
 * is advanced — not only when the user switches tabs. A `cancelled` guard drops
 * out-of-order responses.
 *
 * Returns a `[squad, setSquad]` tuple (like `useState`) so callers can also
 * patch the cached squad optimistically after a mutation. `squad` is `null`
 * until the first fetch resolves.
 */
export function useFetchedSquad(
  teamId: string | null,
  clockDate: string,
): [PlayerData[] | null, Dispatch<SetStateAction<PlayerData[] | null>>] {
  const [fetchedSquad, setFetchedSquad] = useState<PlayerData[] | null>(null);

  useEffect(() => {
    if (!teamId) return;
    let cancelled = false;
    void getSquad(teamId)
      .then((squad) => {
        if (!cancelled) setFetchedSquad(squad);
      })
      .catch(() => {});
    return () => {
      cancelled = true;
    };
  }, [teamId, clockDate]);

  return [fetchedSquad, setFetchedSquad];
}
