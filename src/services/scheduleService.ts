import { invoke } from "@tauri-apps/api/core";

export interface ScheduleQuery {
  competition_id: string;
}

export interface FixtureResult {
  home_goals: number;
  away_goals: number;
}

export interface FixtureSummary {
  id: string;
  matchday: number;
  date: string;
  home_team_id: string;
  home_team_name: string;
  away_team_id: string;
  away_team_name: string;
  /** "League" | "Cup" | "PreseasonTournament" | "ContinentalClub" | etc. */
  competition: string;
  competition_id: string;
  /** "Scheduled" | "InProgress" | "Completed" */
  status: string;
  result: FixtureResult | null;
}

export interface MatchdayGroup {
  key: string;
  date: string;
  matchday: number;
  competition: string;
  is_next_user_match: boolean;
  fixtures: FixtureSummary[];
}

export interface ScheduleSlice {
  competition_id: string;
  competition_name: string;
  today: string;
  past_groups: MatchdayGroup[];
  upcoming_groups: MatchdayGroup[];
  next_user_match_date: string | null;
}

export function fetchSchedule(query: ScheduleQuery): Promise<ScheduleSlice> {
  return invoke<ScheduleSlice>("get_schedule", { query });
}
