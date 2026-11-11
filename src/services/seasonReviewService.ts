import { invoke } from "@tauri-apps/api/core";
import type { LeagueData } from "../store/gameStore";

export const CAREER_SCOPED_READ_COMMANDS = ["get_followed_division"] as const;

export function getFollowedDivision(careerToken: number): Promise<LeagueData | null> {
  return invoke(CAREER_SCOPED_READ_COMMANDS[0], { careerToken });
}
