import { invoke } from "@tauri-apps/api/core";

import type { GameStateData } from "../store/gameStore";
import type { TacticContents } from "../store/generated/TacticContents";
import { invokeForCareer } from "./careerInvoke";

/** A tactic the player saved, as the save keeps it (camelCase on the wire). */
export interface CustomTactic {
  id: string;
  name: string;
  description: string;
  formation: string;
  playStyle: string;
  sourcePresetName?: string | null;
  /** Everything else the tactic holds (#1189); absent for one saved before tactics held it. */
  contents?: TacticContents | null;
}

/** The career's saved tactics, in the order they were made. */
export async function listCustomTactics(): Promise<CustomTactic[]> {
  const tactics = await invoke<unknown>("list_custom_tactics");
  return Array.isArray(tactics) ? (tactics as CustomTactic[]) : [];
}

/**
 * The writes below carry the career token the screen was given with the game. The backend refuses
 * them if another career is active by the time they arrive, so a request sent just before a career
 * switch cannot land in the new career.
 */

/** Adds the tactic or replaces the one with the same id; returns the whole list. */
export async function saveCustomTactic(
  tactic: CustomTactic,
  careerToken: number,
): Promise<CustomTactic[]> {
  return invokeForCareer<CustomTactic[]>("save_custom_tactic", { tactic }, careerToken);
}

/**
 * Saves the team's tactic as it stands, whole (formation, style, every instruction, the eleven and
 * their roles, the takers), under the draft's id and name. The backend reads all of it from the
 * team; the draft only names it. Returns the whole list.
 */
export async function saveCurrentTactic(
  draft: CustomTactic,
  careerToken: number,
): Promise<CustomTactic[]> {
  return invokeForCareer<CustomTactic[]>("save_current_tactic", { tactic: draft }, careerToken);
}

/** Puts the saved tactic on the team; returns the game as it now stands. */
export async function applyCustomTactic(id: string, careerToken: number): Promise<GameStateData> {
  return invokeForCareer<GameStateData>("apply_custom_tactic", { id }, careerToken);
}

/** Removes the tactic with `id`; returns the list that is left. */
export async function deleteCustomTactic(id: string, careerToken: number): Promise<CustomTactic[]> {
  return invokeForCareer<CustomTactic[]>("delete_custom_tactic", { id }, careerToken);
}

/**
 * Whether the old browser library still has to be copied into this career. It is copied once, so a
 * tactic the player deletes afterwards is not copied back on the next load.
 */
export async function legacyTacticsImportPending(): Promise<boolean> {
  return invoke<boolean>("legacy_tactics_import_pending");
}

/** Records that the old browser library has been copied in, for good. */
export async function markLegacyTacticsImported(careerToken: number): Promise<void> {
  await invokeForCareer("mark_legacy_tactics_imported", {}, careerToken);
}
