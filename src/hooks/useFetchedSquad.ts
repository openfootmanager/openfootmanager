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
  // The team the cached squad belongs to. Used to scope the cache so switching
  // clubs (or going unemployed) never exposes the previous team's roster while
  // the new fetch is still pending.
  const [fetchedTeamId, setFetchedTeamId] = useState<string | null>(null);

  const setSquadForCurrentTeam: Dispatch<SetStateAction<PlayerData[] | null>> = (
    next,
  ) => {
    // With no active team there is nothing to cache — never let an optimistic
    // update repopulate (and thus re-expose) a roster in the teamless state.
    if (!teamId) {
      setFetchedTeamId(null);
      setFetchedSquad(null);
      return;
    }
    setFetchedTeamId(teamId);
    setFetchedSquad(next);
  };

  useEffect(() => {
    if (!teamId) {
      setFetchedTeamId(null);
      setFetchedSquad(null);
      return;
    }
    let cancelled = false;
    void getSquad(teamId)
      .then((squad) => {
        if (!cancelled) {
          setFetchedTeamId(teamId);
          setFetchedSquad(squad);
        }
      })
      .catch(() => {});
    return () => {
      cancelled = true;
    };
  }, [teamId, clockDate]);

  // Only serve the cache for the team currently requested.
  return [fetchedTeamId === teamId ? fetchedSquad : null, setSquadForCurrentTeam];
}
