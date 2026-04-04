import type { TeamRecentMatchEntry, TeamStatsOverview } from "./TeamProfile.types";
import { invokeCommand } from "../../services/tauriClient";

export const TEAM_PROFILE_RECENT_MATCH_LIMIT = 5;

export async function fetchTeamStatsOverview(
  teamId: string,
): Promise<TeamStatsOverview | null> {
  return invokeCommand<TeamStatsOverview | null>("get_team_stats_overview", {
    teamId,
  });
}

export async function fetchTeamRecentMatches(
  teamId: string,
  limit = TEAM_PROFILE_RECENT_MATCH_LIMIT,
): Promise<TeamRecentMatchEntry[]> {
  const result = await invokeCommand<TeamRecentMatchEntry[] | null>(
    "get_team_match_history",
    {
      teamId,
      limit,
    },
  );

  return Array.isArray(result) ? result : [];
}
