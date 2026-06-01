import { render, screen } from "@testing-library/react";
import { describe, expect, it, vi } from "vitest";

import type { PlayerData } from "../../store/gameStore";
import HomePlayerMomentumCard from "./HomePlayerMomentumCard";

vi.mock("react-i18next", () => ({
  useTranslation: () => ({
    t: (key: string) => {
      if (key === "dashboard.squad") return "Squad";
      if (key === "home.playerMomentum") return "Player Momentum";
      if (key === "home.inForm") return "In Form";
      if (key === "home.lowMorale") return "Low Morale";
      return key;
    },
  }),
}));

function createPlayer(overrides: Partial<PlayerData> = {}): PlayerData {
  return {
    id: "player-1",
    match_name: "J. Smith",
    full_name: "John Smith",
    date_of_birth: "2000-01-01",
    nationality: "BR",
    position: "Forward",
    natural_position: "Forward",
    alternate_positions: [],
    training_focus: null,
    attributes: {
      pace: 10,
      stamina: 10,
      strength: 10,
      agility: 10,
      passing: 10,
      shooting: 10,
      tackling: 10,
      dribbling: 10,
      defending: 10,
      positioning: 10,
      vision: 10,
      decisions: 10,
      composure: 10,
      aggression: 10,
      teamwork: 10,
      leadership: 10,
      handling: 10,
      reflexes: 10,
      aerial: 10,
    },
    condition: 80,
    morale: 88,
    injury: null,
    team_id: "team-1",
    retired: false,
    contract_end: null,
    wage: 0,
    market_value: 0,
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

describe("HomePlayerMomentumCard", () => {
  it("renders in-form and low-morale player groups", () => {
    render(
      <HomePlayerMomentumCard
        hotPlayers={[createPlayer()]}
        coldPlayers={[createPlayer({ id: "player-2", full_name: "Cold Player", morale: 30 })]}
      />,
    );

    expect(screen.getByText("Player Momentum")).toBeInTheDocument();
    expect(screen.getByText("In Form")).toBeInTheDocument();
    expect(screen.getByText("Low Morale")).toBeInTheDocument();
    expect(screen.getByText("John Smith")).toBeInTheDocument();
    expect(screen.getByText("Cold Player")).toBeInTheDocument();
  });
});