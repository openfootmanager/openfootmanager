import { render, screen } from "@testing-library/react";
import { describe, expect, it } from "vitest";

import type { StaffData } from "../../store/gameStore";
import ScoutingOverviewCards from "./ScoutingOverviewCards";

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
      judgingAbility: 65,
      judgingPotential: 70,
      physiotherapy: 10,
    },
    team_id: "team-1",
    specialization: null,
    wage: 1000,
    contract_end: "2027-06-30",
    ...overrides,
  };
}

describe("ScoutingOverviewCards", () => {
  it("renders scout, assignment, and free-slot counts", () => {
    render(
      <ScoutingOverviewCards
        scouts={[
          createScout({ id: "staff-1", attributes: { coaching: 20, judgingAbility: 65, judgingPotential: 70, physiotherapy: 10 } }),
          createScout({ id: "staff-2", attributes: { coaching: 20, judgingAbility: 80, judgingPotential: 75, physiotherapy: 10 } }),
        ]}
        assignmentCount={3}
        availableScoutCount={1}
        totalCapacity={9}
        labels={{
          scouts: "Scouts",
          activeAssignments: "Active Assignments",
          freeSlots: "Free Slots",
        }}
      />,
    );

    expect(screen.getByText("Scouts")).toBeInTheDocument();
    expect(screen.getByText("2")).toBeInTheDocument();
    expect(screen.getByText("3 / 9")).toBeInTheDocument();
    expect(screen.getByText("1")).toBeInTheDocument();
  });
});