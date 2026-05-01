import { invoke } from "@tauri-apps/api/core";
import type { GameStateData } from "../store/gameStore";

export interface JobOpportunity {
  team_id: string;
  team_name: string;
  city: string;
  reputation: number;
  last_league_position: number | null;
}

export interface JobApplicationResponse {
  result:
    | "hired"
    | "rejected"
    | "invalid_team"
    | "already_employed"
    | "same_team"
    | "not_better_club";
  game: GameStateData;
}

export async function getAvailableJobs(): Promise<JobOpportunity[]> {
  return invoke<JobOpportunity[]>("get_available_jobs");
}

export async function applyForJob(
  teamId: string,
): Promise<JobApplicationResponse> {
  return invoke<JobApplicationResponse>("apply_for_job", { teamId });
}
