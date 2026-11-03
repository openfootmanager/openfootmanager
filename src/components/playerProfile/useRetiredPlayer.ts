import { useEffect, useState } from "react";

import { fetchRetiredPlayer } from "../../services/retireesService";
import type { PlayerData } from "../../store/types";

/**
 * A retired player who is no longer in the roster. Pass `null` when the player is
 * already in the roster, or when nobody is selected: nothing is fetched then.
 */
export function useRetiredPlayer(playerId: string | null): PlayerData | null {
  const [loaded, setLoaded] = useState<PlayerData | null>(null);

  useEffect(() => {
    if (playerId === null) return;
    let cancelled = false;
    fetchRetiredPlayer(playerId)
      .then((player) => {
        if (!cancelled) setLoaded(player);
      })
      .catch(() => {
        if (!cancelled) setLoaded(null);
      });
    return () => {
      cancelled = true;
    };
  }, [playerId]);

  return playerId !== null && loaded?.id === playerId ? loaded : null;
}
