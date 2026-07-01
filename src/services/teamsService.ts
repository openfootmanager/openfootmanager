import { invoke } from "@tauri-apps/api/core";

export const UNGROUPED_LEAGUE_ID = "__ungrouped";

export interface TeamsDirectoryQuery {
  search: string | null;
}

export interface TeamCardColors {
  primary: string;
  secondary: string;
}

export interface TeamCardMedia {
  logo: string | null;
}

export interface TeamCardTeam {
  id: string;
  name: string;
  short_name: string;
  city: string;
  country: string;
  colors: TeamCardColors;
  formation: string;
  play_style: string;
  founded_year: number;
  reputation: number;
  media: TeamCardMedia;
}

export interface TeamStanding {
  played: number;
  won: number;
  drawn: number;
  lost: number;
  goals_for: number;
  goals_against: number;
  points: number;
}

export interface TeamCard {
  team: TeamCardTeam;
  roster_size: number;
  avg_ovr: number;
  total_value: number;
  league_pos: number;
  standing: TeamStanding | null;
}

export interface LeagueGroup {
  id: string;
  name: string;
  name_key?: string | null;
  country_id?: string | null;
  teams: TeamCard[];
}

export interface RegionGroup {
  id: string;
  leagues: LeagueGroup[];
  team_count: number;
}

export interface TeamsDirectory {
  regions: RegionGroup[];
}

export function fetchTeamsDirectory(
  query: TeamsDirectoryQuery,
): Promise<TeamsDirectory> {
  return invoke<TeamsDirectory>("get_teams_directory", { query });
}
