import { useEffect, useState } from "react";
import { invoke } from "@tauri-apps/api/core";

import type { SeasonAwardsData } from "../../store/gameStore";

interface UseSeasonAwardsResult {
  awards: SeasonAwardsData | null;
  awardsLoadState: "idle" | "loading" | "error";
  retryAwards: () => void;
}

/**
 * The season awards for one competition's season, fetched on first view.
 *
 * `enabled` is the awards tab being open: nothing is fetched until the player
 * asks for it. Results are cached per season rather than per fetch, so moving
 * between competitions and back serves the season already loaded instead of
 * hitting the backend again.
 */
export function useSeasonAwards(
  currentSeason: number,
  enabled: boolean,
): UseSeasonAwardsResult {
  const [awardsBySeason, setAwardsBySeason] = useState<
    Record<number, SeasonAwardsData>
  >({});
  const [awardsLoadState, setAwardsLoadState] = useState<
    "idle" | "loading" | "error"
  >("idle");
  const [awardsRetryCount, setAwardsRetryCount] = useState(0);

  const awards = awardsBySeason[currentSeason] ?? null;

  useEffect(() => {
    if (!enabled || awards) {
      return;
    }

    let cancelled = false;
    setAwardsLoadState("loading");

    invoke<SeasonAwardsData>("get_season_awards")
      .then((nextAwards) => {
        if (cancelled) {
          return;
        }

        setAwardsBySeason((current) => ({
          ...current,
          [currentSeason]: nextAwards,
        }));
        setAwardsLoadState("idle");
      })
      .catch(() => {
        if (!cancelled) {
          setAwardsLoadState("error");
        }
      });

    return () => {
      cancelled = true;
    };
  }, [enabled, awards, currentSeason, awardsRetryCount]);

  return {
    awards,
    awardsLoadState,
    retryAwards: () => setAwardsRetryCount((count) => count + 1),
  };
}
