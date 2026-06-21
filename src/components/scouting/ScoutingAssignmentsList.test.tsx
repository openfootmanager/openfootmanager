import { fireEvent, render, screen } from "@testing-library/react";
import { describe, expect, it, vi } from "vitest";

import type {
  ScoutingAssignment,
} from "../../store/gameStore";
import { createPlayer, createStaff, createTeam } from "../../test-utils/factories";
import ScoutingAssignmentsList from "./ScoutingAssignmentsList";

vi.mock("react-i18next", () => ({
  useTranslation: () => ({
    t: (key: string, params?: Record<string, string | number>) => {
      if (key === "scouting.activeScoutingAssignments") {
        return "Active Scouting Assignments";
      }
      if (key === "squad.viewProfile") {
        return "View profile";
      }
      if (key === "common.viewTeam") {
        return "View team";
      }
      if (key === "scouting.scoutLabel") {
        return params?.name ? `Scout ${params.name}` : "Scout";
      }
      if (key === "scouting.daysLeft") {
        return `${params?.days} days left`;
      }
      if (key === "common.freeAgent") {
        return "Free Agent";
      }
      return key;
    },
  }),
}));

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

describe("ScoutingAssignmentsList", () => {
  it("renders active scouting assignments and handles player selection", () => {
    const onSelectPlayer = vi.fn();

    render(
      <ScoutingAssignmentsList
        assignments={[createAssignment()]}
        scouts={[createStaff()]}
        players={[createPlayer({ team_id: "team-2" })]}
        teams={[
          createTeam(),
          createTeam({ id: "team-2", name: "Beta FC", manager_id: "manager-2" }),
        ]}
        onSelectPlayer={onSelectPlayer}
      />,
    );

    expect(screen.getByText("Active Scouting Assignments")).toBeInTheDocument();
    expect(screen.getByText("John Smith")).toBeInTheDocument();
    expect(screen.getByText("Scout Sam Scout")).toBeInTheDocument();
    expect(screen.getByText("3 days left")).toBeInTheDocument();

    fireEvent.click(screen.getByRole("button", { name: "John Smith" }));

    expect(onSelectPlayer).toHaveBeenCalledWith("player-1");
  });

  it("offers a context menu action to view the assigned player's team", () => {
    const onSelectTeam = vi.fn();

    render(
      <ScoutingAssignmentsList
        assignments={[createAssignment()]}
        scouts={[createStaff()]}
        players={[createPlayer({ team_id: "team-2" })]}
        teams={[
          createTeam(),
          createTeam({ id: "team-2", name: "Beta FC", manager_id: "manager-2" }),
        ]}
        onSelectPlayer={vi.fn()}
        onSelectTeam={onSelectTeam}
      />,
    );

    fireEvent.contextMenu(screen.getByTestId("scouting-assignment-assignment-1"));
    fireEvent.click(screen.getByRole("button", { name: "View team" }));

    expect(onSelectTeam).toHaveBeenCalledWith("team-2");
  });
});