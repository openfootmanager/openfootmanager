import { useEffect, useState } from "react";

import { fetchSeasonAwards } from "../../services/competitionsService";
import type { SeasonAwardsData } from "../../store/gameStore";

interface UseSeasonAwardsResult {
  awards: SeasonAwardsData | null;
  awardsLoadState: "idle" | "loading" | "error";
  retryAwards: () => void;
}

interface CachedAwards {
  /** The game date the payload was fetched on — see the staleness note below. */
  asOfDate: string | undefined;
  awards: SeasonAwardsData;
}

/**
 * The season awards for one competition's season, fetched on first view.
 *
 * `enabled` is the awards tab being open: nothing is fetched until the player
 * asks for it. Results are cached per season rather than per fetch, so moving
 * between competitions and back serves the season already loaded.
 *
 * Mid-season the board is standings rather than honours — it is captioned
 * "current leaders" — so a cached payload is only good for the day it was
 * fetched on. Advancing the clock invalidates it; switching competitions and
 * coming back on the same day does not.
 */
export function useSeasonAwards(
  currentSeason: number,
  enabled: boolean,
  asOfDate: string | undefined,
): UseSeasonAwardsResult {
  const [awardsBySeason, setAwardsBySeason] = useState<
    Record<number, CachedAwards>
  >({});
  const [loadState, setLoadState] = useState<"idle" | "loading" | "error">(
    "idle",
  );
  const [awardsRetryCount, setAwardsRetryCount] = useState(0);

  const cached = awardsBySeason[currentSeason];
  const awards = cached?.asOfDate === asOfDate ? cached.awards : null;

  useEffect(() => {
    if (!enabled || awards) {
      return;
    }

    let cancelled = false;
    setLoadState("loading");

    fetchSeasonAwards()
      .then((nextAwards) => {
        if (cancelled) {
          return;
        }

        setAwardsBySeason((current) => ({
          ...current,
          [currentSeason]: { asOfDate, awards: nextAwards },
        }));
        setLoadState("idle");
      })
      .catch(() => {
        if (!cancelled) {
          setLoadState("error");
        }
      });

    return () => {
      cancelled = true;
    };
  }, [enabled, awards, currentSeason, asOfDate, awardsRetryCount]);

  return {
    awards,
    // A season that failed to load must not leave another season's cached
    // awards reporting an error: having the data is what "loaded" means.
    awardsLoadState: awards ? "idle" : loadState,
    retryAwards: () => setAwardsRetryCount((count) => count + 1),
  };
}
