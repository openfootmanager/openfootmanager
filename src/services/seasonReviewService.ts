import { invoke } from "@tauri-apps/api/core";
import type { LeagueData } from "../store/gameStore";

export function getFollowedDivision(careerToken: number): Promise<LeagueData | null> {
  return invoke("get_followed_division", { careerToken });
}
