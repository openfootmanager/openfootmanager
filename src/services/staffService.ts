import { invoke } from "@tauri-apps/api/core";

import type { RatingTier } from "./squadService";

import type { GameStateData } from "../store/gameStore";
import type { ScoutingAssignment, StaffData, YouthScoutingAssignment } from "../store/types";
import { invokeForCareer } from "./careerInvoke";

/**
 * A staff member as the Staff page lists him: his record, with the rating the game computes for his
 * role (ofm_core's staff_rating) and that rating's tier among his role (staff_tier). Both are
 * absent on the game state's own staff, which the page draws before the slice arrives.
 */
export type StaffRow = StaffData & { rating?: number; tier?: RatingTier };

export interface StaffSlice {
  team_staff: StaffRow[];
  available_staff: StaffRow[];
  scouting_assignments: ScoutingAssignment[];
  youth_scouting_assignments: YouthScoutingAssignment[];
}

export async function getStaff(teamId: string): Promise<StaffSlice> {
  return invoke<StaffSlice>("get_staff", { teamId });
}

export async function hireStaff(staffId: string): Promise<GameStateData> {
  return invokeForCareer<GameStateData>("hire_staff", { staffId });
}

export async function releaseStaff(staffId: string): Promise<GameStateData> {
  return invokeForCareer<GameStateData>("release_staff", { staffId });
}
