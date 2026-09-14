import { useEffect, useState } from "react";

import { fetchTeamRecentMatches, fetchTeamStatsOverview } from "./TeamProfile.gateway";
import type { TeamRecentMatchEntry, TeamStatsOverview } from "./TeamProfile.types";

interface TeamProfileStatsState {
  teamStatsOverview: TeamStatsOverview | null;
  recentMatches: TeamRecentMatchEntry[];
}

export function useTeamProfileStats(teamId: string): TeamProfileStatsState {
  const [teamStatsOverview, setTeamStatsOverview] = useState<TeamStatsOverview | null>(null);
  const [recentMatches, setRecentMatches] = useState<TeamRecentMatchEntry[]>([]);

  useEffect(() => {
    let cancelled = false;

    const loadTeamProfileStats = async (): Promise<void> => {
      const [overviewResult, historyResult] = await Promise.allSettled([
        fetchTeamStatsOverview(teamId),
        fetchTeamRecentMatches(teamId),
      ]);

      if (cancelled) {
        return;
      }

      setTeamStatsOverview(overviewResult.status === "fulfilled" ? overviewResult.value : null);
      setRecentMatches(historyResult.status === "fulfilled" ? historyResult.value : []);
    };

    void loadTeamProfileStats();

    return () => {
      cancelled = true;
    };
  }, [teamId]);

  return {
    teamStatsOverview,
    recentMatches,
  };
}
