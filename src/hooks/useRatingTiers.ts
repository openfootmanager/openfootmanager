import { useCallback, useEffect, useState } from "react";

import { getRatingTiers, type RatingTier } from "../services/squadService";

// The table is the same for the whole session, and a screen mounts this hook once per row, so one
// request serves every mount. A failed request is dropped so the next mount asks again.
let pending: Promise<RatingTier[]> | null = null;
let resolved: RatingTier[] | null = null;

function loadRatingTiers(): Promise<RatingTier[]> {
  pending ??= getRatingTiers()
    .then((table) => {
      resolved = table;
      return table;
    })
    .catch((error: unknown) => {
      pending = null;
      throw error;
    });
  return pending;
}

/** Forget the loaded table, so a test can start from the first request again. */
export function resetRatingTiersCache(): void {
  pending = null;
  resolved = null;
}

/**
 * The tier a rating (an ovr or a potential) is in, as the backend defines it. The table is the
 * backend's answer for every rating, so no screen holds a cut-off; ratings read as "Rest" until it
 * arrives. It needs no game, so the package editor uses it too.
 */
export function useRatingTiers(): (rating: number) => RatingTier {
  const [tiers, setTiers] = useState<RatingTier[]>(resolved ?? []);

  useEffect(() => {
    let cancelled = false;
    loadRatingTiers()
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
