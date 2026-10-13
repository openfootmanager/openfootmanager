import { fireEvent, render, screen } from "@testing-library/react";
import { describe, expect, it, vi } from "vitest";

import type { StaffData, YouthScoutingAssignment } from "../../store/gameStore";
import ScoutingYouthRecruitmentCard from "./ScoutingYouthRecruitmentCard";

vi.mock("react-i18next", () => ({
  useTranslation: () => ({
    t: (key: string, params?: Record<string, string | number>) => {
      if (key === "common.positions.Goalkeeper") return "Goalkeeper";
      if (key === "common.positions.Defender") return "Defender";
      if (key === "common.positions.Midfielder") return "Midfielder";
      if (key === "common.positions.Forward") return "Forward";
      if (key === "scouting.youthAnyPosition") return "Any position";
      if (key === "scouting.scoutLabel") return `Scout ${params?.name}`;
      if (key === "scouting.activeYouthSearches") {
        return `Active: ${params?.count ?? 0}`;
      }
      return key;
    },
    i18n: { language: "en" },
  }),
}));

function createScout(overrides: Partial<StaffData> = {}): StaffData {
  return {
    id: "scout-1",
    first_name: "Sam",
    last_name: "Scout",
    display_name: "Sam Scout",
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
    wage: 0,
    contract_end: null,
    ...overrides,
  };
}

function renderCard(
  overrides: Partial<React.ComponentProps<typeof ScoutingYouthRecruitmentCard>> = {},
) {
  const defaults: React.ComponentProps<typeof ScoutingYouthRecruitmentCard> = {
    youthAssignments: [] as YouthScoutingAssignment[],
    scouts: [createScout()],
    availableScouts: [createScout()],
    isStarting: false,
    selectedScoutId: "scout-1",
    region: "Domestic",
    objective: "Balanced",
    targetPosition: "",
    onScoutChange: () => {},
    onRegionChange: () => {},
    onObjectiveChange: () => {},
    onTargetPositionChange: () => {},
    onStartSearch: () => {},
    onCancelSearch: () => {},
    onReassignSearch: () => {},
  };
  return render(<ScoutingYouthRecruitmentCard {...defaults} {...overrides} />);
}

describe("ScoutingYouthRecruitmentCard target-position dropdown", () => {
  it("offers Goalkeeper as a target position", () => {
    renderCard();

    fireEvent.click(screen.getByRole("combobox", { name: "scouting.youthTargetLabel" }));

    expect(screen.getByRole("option", { name: "Goalkeeper" })).toBeInTheDocument();
  });

  /**
   * Given a scout known as "Cafu" (Marcos Evangelista) available for a youth search
   * When the scout picker is drawn
   * Then the chosen scout reads as the backend's display name.
   */
  it("lists an available scout by display name", () => {
    const known = createScout({
      first_name: "Marcos",
      last_name: "Evangelista",
      display_name: "Cafu",
    });
    renderCard({ scouts: [known], availableScouts: [known], selectedScoutId: known.id });

    expect(
      screen.getByRole("combobox", { name: "scouting.youthSearchScoutLabel" }),
    ).toHaveTextContent("Cafu");
  });

  /**
   * Given a youth search run by a scout known as "Cafu" (Marcos Evangelista)
   * When the running searches are drawn
   * Then the search names the scout by display name.
   */
  it("names the scout of a running search by display name", () => {
    const known = createScout({
      first_name: "Marcos",
      last_name: "Evangelista",
      display_name: "Cafu",
    });
    renderCard({
      scouts: [known],
      availableScouts: [],
      youthAssignments: [
        { id: "search-1", scout_id: known.id, days_remaining: 5 } as YouthScoutingAssignment,
      ],
    });

    expect(screen.getByText("Scout Cafu")).toBeInTheDocument();
  });

  /**
   * Given a running youth search and another scout known as "Cafu" (Marcos Evangelista) who is free
   * When the search's reassign picker is opened
   * Then the other scout is offered by display name.
   */
  it("offers the free scout by display name when reassigning a search", () => {
    const busy = createScout({ id: "scout-busy" });
    const free = createScout({
      id: "scout-free",
      first_name: "Marcos",
      last_name: "Evangelista",
      display_name: "Cafu",
    });
    renderCard({
      scouts: [busy, free],
      availableScouts: [free],
      youthAssignments: [
        { id: "search-1", scout_id: busy.id, days_remaining: 5 } as YouthScoutingAssignment,
      ],
    });

    expect(
      screen.getByRole("combobox", { name: "scouting.reassignSearch search-1" }),
    ).toHaveTextContent("Cafu");
  });
});
