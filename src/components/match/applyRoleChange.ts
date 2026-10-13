import { invoke } from "@tauri-apps/api/core";

import type { MatchSnapshot, RoleChangeOutcome } from "./types";

/**
 * Asks the engine to give one of the side's players on the pitch a new role. The engine decides
 * whether the role suits the player's position; a refusal comes back as its translation key for
 * the screen to show, and nothing changes.
 */
export async function applyRoleChange(
  side: "Home" | "Away",
  playerId: string,
  role: string,
  onSnapshot: (snapshot: MatchSnapshot) => void,
): Promise<RoleChangeOutcome> {
  try {
    const snapshot = await invoke<MatchSnapshot>("apply_match_command", {
      command: { ChangePlayerRole: { side, player_id: playerId, role } },
    });
    onSnapshot(snapshot);
    return { applied: true };
  } catch (err) {
    console.error("Role change failed:", err);
    return { applied: false, reason: String(err) };
  }
}
