import { describe, expect, it } from "vitest";

import type { PlayerData } from "../store/gameStore";
import {
  weeklyWageAmount,
  getPlayerAnnualWageCommitment,
} from "./finance";

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
    morale: 80,
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

describe("finance helpers", () => {
  it("treats stored wages as weekly commitments", () => {
    expect(weeklyWageAmount(5_000)).toBe(5_000);
  });

  it("splits active loan wage commitments by the agreed contribution", () => {
    const loanedPlayer = createPlayer({
      id: "loaned-player",
      team_id: "loan",
      wage: 520000,
      active_loan: {
        parent_team_id: "parent",
        loan_team_id: "loan",
        start_date: "2026-08-01",
        end_date: "2027-01-28",
        wage_contribution_pct: 60,
        buy_option_fee: null,
      },
    });

    expect(getPlayerAnnualWageCommitment(loanedPlayer, "parent")).toBe(208000);
    expect(getPlayerAnnualWageCommitment(loanedPlayer, "loan")).toBe(312000);
  });

  it("assigns the loan wage remainder to the parent club like the backend", () => {
    const loanedPlayer = createPlayer({
      team_id: "loan",
      wage: 101,
      active_loan: {
        parent_team_id: "parent",
        loan_team_id: "loan",
        start_date: "2026-08-01",
        end_date: "2027-01-28",
        wage_contribution_pct: 50,
        buy_option_fee: null,
      },
    });

    expect(getPlayerAnnualWageCommitment(loanedPlayer, "loan")).toBe(50);
    expect(getPlayerAnnualWageCommitment(loanedPlayer, "parent")).toBe(51);
  });
});
