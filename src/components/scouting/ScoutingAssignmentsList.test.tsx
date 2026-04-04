import { fireEvent, render, screen } from "@testing-library/react";
import { describe, expect, it, vi } from "vitest";

import type {
  PlayerData,
  ScoutingAssignment,
  StaffData,
  TeamData,
} from "../../store/gameStore";
import ScoutingAssignmentsList from "./ScoutingAssignmentsList";

vi.mock("react-i18next", () => ({
  useTranslation: () => ({
    t: (key: string, params?: Record<string, string | number>) => {
      if (key === "scouting.activeScoutingAssignments") {
        return "Active Scouting Assignments";
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

function createTeam(overrides: Partial<TeamData> = {}): TeamData {
  return {
    id: "team-1",
    name: "Alpha FC",
    short_name: "ALP",
    country: "GB",
    city: "London",
    stadium_name: "Alpha Ground",
    stadium_capacity: 30000,
    finance: 500000,
    manager_id: "manager-1",
    reputation: 50,
    wage_budget: 50000,
    transfer_budget: 250000,
    season_income: 0,
    season_expenses: 0,
    formation: "4-4-2",
    play_style: "Balanced",
    training_focus: "General",
    training_intensity: "Balanced",
    training_schedule: "Balanced",
    founded_year: 1900,
    colors: { primary: "#000000", secondary: "#ffffff" },
    starting_xi_ids: [],
    form: [],
    history: [],
    ...overrides,
  };
}

function createPlayer(overrides: Partial<PlayerData> = {}): PlayerData {
  return {
    id: "player-1",
    match_name: "J. Smith",
    full_name: "John Smith",
    date_of_birth: "2000-01-01",
    nationality: "GB",
    position: "Forward",
    natural_position: "Forward",
    alternate_positions: [],
    training_focus: null,
    attributes: {
      pace: 60,
      stamina: 60,
      strength: 60,
      agility: 60,
      passing: 60,
      shooting: 60,
      tackling: 60,
      dribbling: 60,
      defending: 60,
      positioning: 60,
      vision: 60,
      decisions: 60,
      composure: 60,
      aggression: 60,
      teamwork: 60,
      leadership: 60,
      handling: 20,
      reflexes: 20,
      aerial: 60,
    },
    condition: 80,
    morale: 75,
    injury: null,
    team_id: "team-2",
    contract_end: "2027-06-30",
    wage: 12000,
    market_value: 350000,
    stats: {
      appearances: 0,
      goals: 0,
      assists: 0,
      clean_sheets: 0,
      yellow_cards: 0,
      red_cards: 0,
      avg_rating: 0,
      minutes_played: 0,
    },
    career: [],
    transfer_listed: false,
    loan_listed: false,
    transfer_offers: [],
    traits: [],
    ...overrides,
  };
}

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

describe("ScoutingAssignmentsList", () => {
  it("renders active scouting assignments and handles player selection", () => {
    const onSelectPlayer = vi.fn();

    render(
      <ScoutingAssignmentsList
        assignments={[createAssignment()]}
        scouts={[createScout()]}
        players={[createPlayer()]}
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
});