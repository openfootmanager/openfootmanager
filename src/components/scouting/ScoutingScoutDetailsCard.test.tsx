import { render, screen } from "@testing-library/react";
import { describe, expect, it, vi } from "vitest";

import type {
  PlayerData,
  ScoutingAssignment,
  StaffData,
} from "../../store/gameStore";
import ScoutingScoutDetailsCard from "./ScoutingScoutDetailsCard";

vi.mock("react-i18next", () => ({
  useTranslation: () => ({
    t: (key: string, params?: Record<string, string | number>) => {
      if (key === "scouting.yourScouts") return "Your Scouts";
      if (key === "scouting.slots") return "slots";
      if (key === "scouting.judgingAbility") return "Judging Ability";
      if (key === "scouting.judgingPotential") return "Judging Potential";
      if (key === "scouting.scoutLabel") {
        return params?.name ? `Scout ${params.name}` : "Scout ";
      }
      return key;
    },
    i18n: { language: "en" },
  }),
}));

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
    retired: false,
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

function createAssignment(
  overrides: Partial<ScoutingAssignment> = {},
): ScoutingAssignment {
  return {
    id: "assignment-1",
    scout_id: "staff-1",
    player_id: "player-1",
    days_remaining: 4,
    ...overrides,
  };
}

describe("ScoutingScoutDetailsCard", () => {
  it("renders scout details and current assignments", () => {
    render(
      <ScoutingScoutDetailsCard
        scouts={[createScout()]}
        assignments={[createAssignment()]}
        players={[createPlayer()]}
      />,
    );

    expect(screen.getByText("Your Scouts")).toBeInTheDocument();
    expect(screen.getByText("Sam Scout")).toBeInTheDocument();
    expect(screen.getByText(/1\/1 slots/i)).toBeInTheDocument();
    expect(screen.getByText(/John Smith/)).toBeInTheDocument();
    expect(screen.getByText(/4d/)).toBeInTheDocument();
  });
});