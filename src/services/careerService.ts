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

/**
 * The translation key of what is wrong with a manager's date of birth on the day a career opens,
 * or `null` when it will do. The backend owns the age rule and the opening date (1 June in a World
 * Cup summer, 1 July otherwise, 120 days later mid-season); the form only shows the answer.
 */
export function validateManagerDob(
  dob: string,
  startYear: number,
  startPhase: string,
): Promise<string | null> {
  return invoke<string | null>("validate_manager_dob", { dob, startYear, startPhase });
}
