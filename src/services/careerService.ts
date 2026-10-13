import { invoke } from "@tauri-apps/api/core";

/** The backend's verdict on where the active career stands; the frontend only routes on it. */
export type CareerStage = "awaitingClub" | "unemployed" | "employed";

export function getCareerStage(): Promise<CareerStage> {
  return invoke<CareerStage>("get_career_stage");
}

/**
 * Where to send the player once a game is active. A career that has not taken its first club goes
 * to team selection; every other state, and any failure to ask, goes to the dashboard so the
 * player is never left without a screen.
 */
export async function routeForActiveCareer(): Promise<"/select-team" | "/dashboard"> {
  try {
    return (await getCareerStage()) === "awaitingClub" ? "/select-team" : "/dashboard";
  } catch {
    return "/dashboard";
  }
}
