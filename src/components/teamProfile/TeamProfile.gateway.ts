import { invoke } from "@tauri-apps/api/core";

import type {
  ClubLeagueStanding,
  TeamRecentMatchEntry,
  TeamStatsOverview,
} from "./TeamProfile.types";

export const TEAM_PROFILE_RECENT_MATCH_LIMIT = 5;

export function fetchClubLeagueStanding(teamId: string): Promise<ClubLeagueStanding | null> {
  return invoke<ClubLeagueStanding | null>("get_club_league_standing", { teamId });
}

export async function fetchTeamStatsOverview(teamId: string): Promise<TeamStatsOverview | null> {
  return invoke<TeamStatsOverview | null>("get_team_stats_overview", {
    teamId,
  });
}

export async function fetchTeamRecentMatches(
  teamId: string,
  limit = TEAM_PROFILE_RECENT_MATCH_LIMIT,
): Promise<TeamRecentMatchEntry[]> {
  const result = await invoke<TeamRecentMatchEntry[] | null>("get_team_match_history", {
    teamId,
    limit,
  });

  return Array.isArray(result) ? result : [];
}
