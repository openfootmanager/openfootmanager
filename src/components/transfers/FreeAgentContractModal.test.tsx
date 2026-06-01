import { fireEvent, render, screen } from "@testing-library/react";
import { describe, expect, it, vi } from "vitest";

import type { PlayerData, TeamData } from "../../store/gameStore";
import type { FreeAgentContractProjection } from "../../services/freeAgentService";
import FreeAgentContractModal from "./FreeAgentContractModal";

vi.mock("react-i18next", () => ({
  useTranslation: () => ({
    t: (key: string, params?: Record<string, string | number>) => {
      if (key === "transfers.offerContract") return "Offer Contract";
      if (key === "common.freeAgent") return "Free Agent";
      if (key === "transfers.playerValue") return `Value ${params?.value}`;
      if (key === "playerProfile.renewalWage") return "Offered Wage";
      if (key === "playerProfile.renewalLength") return "Contract Length";
      if (key === "playerProfile.renewalProjectionTitle") return "Projected financial impact";
      if (key === "playerProfile.renewalProjectionWageBill") {
        return `Weekly wage bill ${params?.before} -> ${params?.after}`;
      }
      if (key === "playerProfile.renewalProjectionBudgetUsage") {
        return `Wage budget use ${params?.before}% -> ${params?.after}%`;
      }
      if (key === "playerProfile.renewalProjectionRunway") {
        return `Cash runway ${params?.before} -> ${params?.after}`;
      }
      if (key === "playerProfile.renewalBudgetWarning") return "Budget warning";
      if (key === "playerProfile.renewalConversationTitle") return "Negotiation pulse";
      if (key === "playerProfile.renewalRound") return `Round ${params?.count}`;
      if (key === "playerProfile.renewalPatience") return "Patience";
      if (key === "playerProfile.renewalTension") return "Tension";
      if (key === "playerProfile.renewalSubmit") return "Submit Offer";
      if (key === "transfers.submitting") return "Submitting";
      if (key === "transfers.close") return "Close";
      return key;
    },
  }),
}));

function createTeam(overrides: Partial<TeamData> = {}): TeamData {
  return {
    id: "team-1",
    name: "User FC",
    short_name: "USR",
    country: "England",
    city: "London",
    stadium_name: "User Ground",
    stadium_capacity: 25000,
    finance: 5000000,
    manager_id: "manager-1",
    reputation: 50,
    wage_budget: 50000,
    transfer_budget: 2000000,
    season_income: 0,
    season_expenses: 0,
    formation: "4-4-2",
    play_style: "Balanced",
    training_focus: "Physical",
    training_intensity: "Medium",
    training_schedule: "Balanced",
    founded_year: 1900,
    colors: { primary: "#111111", secondary: "#ffffff" },
    facilities: { training: 1, medical: 1, scouting: 1 },
    starting_xi_ids: [],
    match_roles: {
      captain: null,
      vice_captain: null,
      penalty_taker: null,
      free_kick_taker: null,
      corner_taker: null,
    },
    form: [],
    history: [],
    ...overrides,
  };
}

function createPlayer(overrides: Partial<PlayerData> = {}): PlayerData {
  return {
    id: "player-1",
    match_name: "F. Agent",
    full_name: "Free Agent",
    date_of_birth: "2000-01-01",
    nationality: "England",
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
      handling: 30,
      reflexes: 30,
      aerial: 60,
    },
    condition: 90,
    morale: 70,
    injury: null,
    team_id: null,
    retired: false,
    contract_end: null,
    wage: 0,
    market_value: 600000,
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

function createProjection(
  overrides: Partial<FreeAgentContractProjection> = {},
): FreeAgentContractProjection {
  return {
    current_annual_wage_bill: 12000,
    projected_annual_wage_bill: 16000,
    annual_wage_budget: 50000,
    annual_soft_cap: 55000,
    current_weekly_wage_spend: 12000,
    projected_weekly_wage_spend: 16000,
    current_cash_runway_weeks: 42,
    projected_cash_runway_weeks: 31,
    currently_over_budget: false,
    policy_allows: true,
    ...overrides,
  };
}

describe("FreeAgentContractModal", () => {
  it("renders the contract offer state", () => {
    render(
      <FreeAgentContractModal
        player={createPlayer()}
        teams={[createTeam()]}
        wage="4000"
        onWageChange={vi.fn()}
        contractLength="3"
        onContractLengthChange={vi.fn()}
        projection={createProjection()}
        feedback={{
          mood: "firm",
          headline_key: "headline",
          detail_key: "detail",
          tension: 50,
          patience: 65,
          round: 1,
          params: {},
        }}
        statusMessage="Offer accepted"
        statusClassName="text-primary-500"
        submitting={false}
        submitDisabled={false}
        onSubmit={vi.fn()}
        onClose={vi.fn()}
      />,
    );

    expect(screen.getByRole("dialog", { name: "Offer Contract" })).toBeInTheDocument();
    expect(screen.getByText("Free Agent")).toBeInTheDocument();
    expect(screen.getByText("Projected financial impact")).toBeInTheDocument();
    expect(screen.getByText("Offer accepted")).toBeInTheDocument();
    expect(screen.getByLabelText("Contract Length")).toHaveAttribute("max", "5");
  });

  it("wires input, submit, and close interactions", () => {
    const onWageChange = vi.fn();
    const onContractLengthChange = vi.fn();
    const onSubmit = vi.fn();
    const onClose = vi.fn();

    render(
      <FreeAgentContractModal
        player={createPlayer()}
        teams={[createTeam()]}
        wage="4000"
        onWageChange={onWageChange}
        contractLength="3"
        onContractLengthChange={onContractLengthChange}
        projection={createProjection()}
        feedback={null}
        statusMessage={null}
        statusClassName="text-gray-500"
        submitting={false}
        submitDisabled={false}
        onSubmit={onSubmit}
        onClose={onClose}
      />,
    );

    fireEvent.change(screen.getByLabelText("Offered Wage"), {
      target: { value: "5000" },
    });
    fireEvent.change(screen.getByLabelText("Contract Length"), {
      target: { value: "2" },
    });
    fireEvent.click(screen.getByRole("button", { name: "Submit Offer" }));
    fireEvent.click(screen.getByRole("button", { name: "Close" }));

    expect(onWageChange).toHaveBeenCalledWith("5000");
    expect(onContractLengthChange).toHaveBeenCalledWith("2");
    expect(onSubmit).toHaveBeenCalledTimes(1);
    expect(onClose).toHaveBeenCalled();
  });
});
