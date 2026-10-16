import type { ScoutingAssignment } from "../../store/gameStore";

export function buildAlreadyScoutingIds(assignments: ScoutingAssignment[]) {
  return new Set(assignments.map((assignment) => assignment.player_id));
}
