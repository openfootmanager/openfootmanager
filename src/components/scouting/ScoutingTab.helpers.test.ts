import { describe, expect, it } from "vitest";

import type { ScoutingAssignment, StaffData } from "../../store/gameStore";
import {
  calculateAvailableScouts,
  scoutAssignmentCount,
  scoutMaxSlots,
} from "./ScoutingTab.helpers";

function createScout(overrides: Partial<StaffData> = {}): StaffData {
  return {
    id: "staff-1",
    first_name: "Sam",
    last_name: "Scout",
    date_of_birth: "1985-01-01",
    nationality: "GB",
    role: "Scout",
    attributes: {
      coaching: 20,
      judging_ability: 65,
      judging_potential: 70,
      physiotherapy: 10,
    },
    team_id: "team-1",
    specialization: null,
    wage: 1000,
    contract_end: "2027-06-30",
    ...overrides,
  };
}

function createAssignment(
  overrides: Partial<ScoutingAssignment> = {},
): ScoutingAssignment {
  return {
    id: "assignment-1",
    scout_id: "staff-1",
    player_id: "player-1",
    days_remaining: 3,
    ...overrides,
  };
}

describe("ScoutingTab.helpers", () => {
  it("maps judging ability tiers to scout slot counts", () => {
    expect(scoutMaxSlots(10)).toBe(1);
    expect(scoutMaxSlots(20)).toBe(2);
    expect(scoutMaxSlots(40)).toBe(3);
    expect(scoutMaxSlots(60)).toBe(4);
    expect(scoutMaxSlots(80)).toBe(5);
  });

  it("counts assignments for a specific scout", () => {
    const assignments = [
      createAssignment({ id: "a1", scout_id: "staff-1" }),
      createAssignment({ id: "a2", scout_id: "staff-1" }),
      createAssignment({ id: "a3", scout_id: "staff-2" }),
    ];

    expect(scoutAssignmentCount(assignments, "staff-1")).toBe(2);
    expect(scoutAssignmentCount(assignments, "staff-2")).toBe(1);
  });

  it("returns only scouts with remaining assignment capacity", () => {
    const scouts = [
      createScout({ id: "staff-1", attributes: { coaching: 20, judging_ability: 20, judging_potential: 70, physiotherapy: 10 } }),
      createScout({ id: "staff-2", attributes: { coaching: 20, judging_ability: 80, judging_potential: 70, physiotherapy: 10 } }),
    ];
    const assignments = [
      createAssignment({ id: "a1", scout_id: "staff-1" }),
      createAssignment({ id: "a2", scout_id: "staff-1" }),
      createAssignment({ id: "a3", scout_id: "staff-2" }),
    ];

    expect(calculateAvailableScouts(scouts, assignments).map((scout) => scout.id)).toEqual([
      "staff-2",
    ]);
  });
});