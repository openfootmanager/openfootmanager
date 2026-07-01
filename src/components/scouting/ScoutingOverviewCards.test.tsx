import { render, screen } from "@testing-library/react";
import { describe, expect, it } from "vitest";

import { createStaff } from "../../test-utils/factories";
import ScoutingOverviewCards from "./ScoutingOverviewCards";

describe("ScoutingOverviewCards", () => {
  it("renders scout, assignment, and free-slot counts", () => {
    render(
      <ScoutingOverviewCards
        scouts={[
          createStaff({ id: "staff-1", attributes: { coaching: 20, judgingAbility: 65, judgingPotential: 70, physiotherapy: 10 } }),
          createStaff({ id: "staff-2", attributes: { coaching: 20, judgingAbility: 80, judgingPotential: 75, physiotherapy: 10 } }),
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