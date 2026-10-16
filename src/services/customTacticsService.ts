import { invoke } from "@tauri-apps/api/core";

/** A tactic the player saved, as the save keeps it (camelCase on the wire). */
export interface CustomTactic {
  id: string;
  name: string;
  description: string;
  formation: string;
  playStyle: string;
  sourcePresetName?: string | null;
}

/** The career's saved tactics, in the order they were made. */
export async function listCustomTactics(): Promise<CustomTactic[]> {
  const tactics = await invoke<unknown>("list_custom_tactics");
  return Array.isArray(tactics) ? (tactics as CustomTactic[]) : [];
}

/** Adds the tactic or replaces the one with the same id; returns the whole list. */
export async function saveCustomTactic(tactic: CustomTactic): Promise<CustomTactic[]> {
  return invoke<CustomTactic[]>("save_custom_tactic", { tactic });
}

/** Removes the tactic with `id`; returns the list that is left. */
export async function deleteCustomTactic(id: string): Promise<CustomTactic[]> {
  return invoke<CustomTactic[]>("delete_custom_tactic", { id });
}
