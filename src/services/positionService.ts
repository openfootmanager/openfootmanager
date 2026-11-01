import { invoke } from "@tauri-apps/api/core";

/** A position, the group (Goalkeeper, Defender, Midfielder or Forward) it belongs to and its sort rank. */
export interface PositionGroupEntry {
  position: string;
  group: string;
  /** Where the position sorts in a squad list (lower first). */
  rank: number;
}

/** Every position with its group: the backend's one rule, no game needed. */
export async function getPositionGroups(): Promise<PositionGroupEntry[]> {
  return invoke<PositionGroupEntry[]>("get_position_groups");
}
