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
  const [report, setReport] = useState<ScoutedPlayerView | null>(null);

  useEffect(() => {
    setReport(null);
    if (!enabled) return;

    let cancelled = false;
    getScoutedReport(playerId)
      .then((fetched) => {
        if (!cancelled) setReport(fetched);
      })
      .catch(() => {
        if (!cancelled) setReport(null);
      });

    return () => {
      cancelled = true;
    };
  }, [playerId, enabled, currentDate]);

  return report;
}
