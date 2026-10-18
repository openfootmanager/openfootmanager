import { useCallback, useEffect, useState } from "react";

import { getRatingTiers, type RatingTier } from "../services/squadService";

/**
 * The tier a rating (an ovr or a potential) is in, as the backend defines it. The table is the
 * backend's answer for every rating, so no screen holds a cut-off; ratings read as "Rest" until it
 * arrives. It needs no game, so the package editor uses it too.
 */
export function useRatingTiers(): (rating: number) => RatingTier {
  const [tiers, setTiers] = useState<RatingTier[]>([]);

  useEffect(() => {
    let cancelled = false;
    getRatingTiers()
      .then((table) => {
        if (!cancelled) setTiers(table);
      })
      .catch((error: unknown) => {
        console.error("Failed to load the rating tiers:", error);
      });
    return () => {
      cancelled = true;
    };
  }, []);

  return useCallback((rating) => tiers[Math.round(rating)] ?? "Rest", [tiers]);
}
