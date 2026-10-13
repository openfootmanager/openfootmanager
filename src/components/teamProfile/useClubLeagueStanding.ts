import { useEffect, useState } from "react";

import { fetchClubLeagueStanding } from "./TeamProfile.gateway";
import type { ClubLeagueStanding } from "./TeamProfile.types";

/**
 * The club's place in its league table, from the backend (which knows which table counts in a
 * split season). Refetched as the game date moves, since the table changes with every matchday.
 */
export function useClubLeagueStanding(
  teamId: string,
  currentDate: string,
): {
  teamId: string;
  standing: ClubLeagueStanding | null;
} {
  const [loaded, setLoaded] = useState<{ teamId: string; standing: ClubLeagueStanding | null }>({
    teamId,
    standing: null,
  });

  useEffect(() => {
    let cancelled = false;
    fetchClubLeagueStanding(teamId)
      .then((standing) => {
        if (!cancelled) setLoaded({ teamId, standing });
      })
      .catch(() => {
        if (!cancelled) setLoaded({ teamId, standing: null });
      });
    return () => {
      cancelled = true;
    };
  }, [teamId, currentDate]);

  // A standing fetched for another club must not show under this one while the next loads.
  return loaded.teamId === teamId ? loaded : { teamId, standing: null };
}
