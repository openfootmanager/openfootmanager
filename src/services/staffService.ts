import type { GameStateData } from "../store/gameStore";
import { invokeCommand } from "./tauriClient";

export async function hireStaff(staffId: string): Promise<GameStateData> {
  return invokeCommand<GameStateData>("hire_staff", { staffId });
}

export async function releaseStaff(staffId: string): Promise<GameStateData> {
  return invokeCommand<GameStateData>("release_staff", { staffId });
}