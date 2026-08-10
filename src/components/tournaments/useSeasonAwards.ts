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
 * The season awards, fetched on first view and cached per season.
 *
 * Note what the payload is *not*: `get_season_awards` takes no competition, and
 * computes the race within the user's own division — or across the whole game
 * when they have no division. Selecting a different competition does not scope
 * it. The season is still the right cache key, because it is what changes when
 * the awards themselves do.
 *
 * `enabled` is the awards tab being open: nothing is fetched until the player
 * asks for it.
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
  // `cached &&` is load-bearing: with no clock both sides of the date check are
  // undefined, so an optional chain would compare equal on a missing entry.
  const awards = cached && cached.asOfDate === asOfDate ? cached.awards : null;

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
