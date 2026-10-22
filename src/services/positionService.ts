import { invoke } from "@tauri-apps/api/core";

/** A position and the group (Goalkeeper, Defender, Midfielder or Forward) it belongs to. */
export interface PositionGroupEntry {
  position: string;
  group: string;
}

/** Every position with its group: the backend's one rule, no game needed. */
export async function getPositionGroups(): Promise<PositionGroupEntry[]> {
  return invoke<PositionGroupEntry[]>("get_position_groups");
}
