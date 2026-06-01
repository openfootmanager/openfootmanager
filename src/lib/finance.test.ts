import { describe, expect, it } from "vitest";

import type { PlayerData, StaffData, TeamData } from "../store/gameStore";
import {
  annualAmountToWeeklyCommitment,
  getAnnualWageBill,
  getCashRunwayWeeks,
  getTeamFinanceSnapshot,
  getWeeklyWageSpend,
} from "./finance";

function createTeam(overrides: Partial<TeamData> = {}): TeamData {
  return {
    id: "team-1",
    name: "Alpha FC",
    short_name: "ALP",
    country: "BR",
    city: "Rio",
    stadium_name: "Alpha Arena",
    stadium_capacity: 50000,
    finance: 180000,
    manager_id: "manager-1",
    reputation: 50,
    wage_budget: 520000,
    transfer_budget: 300000,
    season_income: 0,
    season_expenses: 0,
    formation: "4-4-2",
    play_style: "Balanced",
    training_focus: "Physical",
    training_intensity: "Medium",
    training_schedule: "Balanced",
    founded_year: 1900,
    colors: {
      primary: "#111111",
      secondary: "#ffffff",
    },
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

function createStaff(overrides: Partial<StaffData> = {}): StaffData {
  return {
    id: "staff-1",
    first_name: "Pat",
    last_name: "Coach",
    date_of_birth: "1980-01-01",
    nationality: "BR",
    role: "Coach",
    attributes: {
      coaching: 10,
      judging_ability: 10,
      judging_potential: 10,
      physiotherapy: 10,
    },
    team_id: "team-1",
    specialization: null,
    wage: 0,
    contract_end: null,
    ...overrides,
  };
}

describe("finance helpers", () => {
  it("converts annual contracts to weekly commitments using per-person flooring", () => {
    const players = [
      createPlayer({ wage: 51 }),
      createPlayer({ id: "player-2", wage: 51 }),
    ];
    const staff = [createStaff({ wage: 103 })];

    expect(annualAmountToWeeklyCommitment(103)).toBe(1);
    expect(getAnnualWageBill(players, staff)).toBe(205);
    expect(getWeeklyWageSpend(players, staff)).toBe(1);
  });

  it("computes runway from projected weekly net rather than wages alone", () => {
    expect(getCashRunwayWeeks(200000, -30000)).toBe(6);
    expect(getCashRunwayWeeks(200000, 5000)).toBeNull();
  });

  it("builds a finance snapshot with the worst status carried forward", () => {
    const team = createTeam({
      finance: 25000,
      wage_budget: 500000,
    });
    const players = [
      createPlayer({ wage: 300000 }),
      createPlayer({ id: "player-2", wage: 300000 }),
    ];

    const snapshot = getTeamFinanceSnapshot(team, players);

    expect(snapshot.annualWageBill).toBe(600000);
    expect(snapshot.weeklyWageSpend).toBe(11538);
    expect(snapshot.weeklyWageBudget).toBe(9615);
    expect(snapshot.projectedWeeklyNet).toBe(-11538);
    expect(snapshot.cashRunwayWeeks).toBe(2);
    expect(snapshot.wageBudgetUsagePercent).toBe(120);
    expect(snapshot.wageBudgetStatus).toBe("critical");
    expect(snapshot.runwayStatus).toBe("critical");
    expect(snapshot.overallStatus).toBe("critical");
  });
});