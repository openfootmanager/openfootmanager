import type { ScoutingAssignment, StaffData } from "../../store/gameStore";

type ScoutWorkloadAssignment = Pick<ScoutingAssignment, "scout_id">;

export function scoutMaxSlots(ability: number): number {
  void ability;
  return 1;
}

export function scoutAssignmentCount(
  assignments: ScoutWorkloadAssignment[],
  scoutId: string,
): number {
  return assignments.filter((assignment) => assignment.scout_id === scoutId).length;
}

export function calculateAvailableScouts(
  scouts: StaffData[],
  assignments: ScoutWorkloadAssignment[],
): StaffData[] {
  return scouts.filter(
    (scout) =>
      scoutAssignmentCount(assignments, scout.id) <
      scoutMaxSlots(scout.attributes.judging_ability),
  );
}