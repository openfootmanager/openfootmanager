import { useEffect, useState } from "react";
import { getScoutedReport } from "../../services/scoutingService";
import type { ScoutedPlayerView } from "../../store/gameStore";

/**
 * The scout's report on a player, fetched from the backend (which also decides when it is out
 * of date). Refetched when the game date moves, since a scouting job can finish on any day.
 */
export function useScoutedReport(
  playerId: string,
  enabled: boolean,
  currentDate: string,
): ScoutedPlayerView | null {
  const [fetched, setFetched] = useState<ScoutedPlayerView | null>(null);

  useEffect(() => {
    setFetched(null);
    if (!enabled) return;

    let cancelled = false;
    getScoutedReport(playerId)
      .then((report) => {
        if (!cancelled) setFetched(report);
      })
      .catch(() => {
        if (!cancelled) setFetched(null);
      });

    return () => {
      cancelled = true;
    };
  }, [playerId, enabled, currentDate]);

  // The effect clears after the render in which the player changed, so a report for
  // another player must read as none here.
  return fetched?.player_id === playerId ? fetched : null;
}
