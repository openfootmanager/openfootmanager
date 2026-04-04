import { fireEvent, render, screen } from "@testing-library/react";
import { describe, expect, it, vi } from "vitest";

import type {
  PlayerData,
  TeamData,
  TransferOfferData,
} from "../../store/gameStore";
import type {
  TransferBidProjectionData,
} from "../../services/transfersService";
import TransferBidModal from "./TransferBidModal";

vi.mock("react-i18next", () => ({
  useTranslation: () => ({
    t: (key: string, params?: Record<string, string | number>) => {
      if (key === "transfers.makeBid") return "Make Transfer Bid";
      if (key === "transfers.playerValue") return `Value ${params?.value}`;
      if (key === "transfers.resumeNegotiationHint") return "Talks are still live with this club.";
      if (key === "transfers.bidAmount") return "Bid Amount";
      if (key === "transfers.bidImpactTitle") return "Projected impact";
      if (key === "transfers.bidImpactTransferBudget") {
        return `Transfer budget ${params?.before} -> ${params?.after}`;
      }
      if (key === "transfers.bidImpactBalance") {
        return `Club balance ${params?.before} -> ${params?.after}`;
      }
      if (key === "transfers.bidImpactWagePressure") {
        return `Projected wage budget usage ${params?.percent}%`;
      }
      if (key === "transfers.bidImpactOverTransferBudget") {
        return "This bid exceeds your transfer budget";
      }
      if (key === "transfers.bidImpactOverBalance") {
        return "This bid would push the club into debt";
      }
      if (key === "transfers.negotiationPulse") return "Negotiation pulse";
      if (key === "transfers.negotiationRound") return `Round ${params?.count}`;
      if (key === "transfers.negotiationPatience") return "Patience";
      if (key === "transfers.negotiationTension") return "Tension";
      if (key === "transfers.negotiationHistory") return "Recent exchange";
      if (key === "transfers.lastBidLabel") return "Your last bid";
      if (key === "transfers.lastClubSignalLabel") return "Their last signal";
      if (key === "transfers.bidAccepted") return "Bid accepted";
      if (key === "transfers.bidRejected") return "Bid rejected";
      if (key === "transfers.bidCountered") return "Bid countered";
      if (key === "transfers.submitBid") return "Submit Bid";
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
    colors: {
      primary: "#111111",
      secondary: "#ffffff",
    },
    facilities: {
      training: 1,
      medical: 1,
      scouting: 1,
    },
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
    match_name: "J. Smith",
    full_name: "John Smith",
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
    team_id: "team-2",
    contract_end: "2028-06-30",
    wage: 1000,
    market_value: 1000000,
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
    transfer_listed: true,
    loan_listed: false,
    transfer_offers: [],
    traits: [],
    ...overrides,
  };
}

function createOffer(overrides: Partial<TransferOfferData> = {}): TransferOfferData {
  return {
    id: "offer-1",
    from_team_id: "team-1",
    fee: 1200000,
    wage_offered: 0,
    last_manager_fee: 1000000,
    negotiation_round: 2,
    suggested_counter_fee: 1500000,
    status: "Pending",
    date: "2026-08-01",
    ...overrides,
  };
}

function createProjection(
  overrides: Partial<TransferBidProjectionData["projection"]> = {},
): TransferBidProjectionData["projection"] {
  return {
    transfer_budget_before: 2000000,
    transfer_budget_after: 800000,
    finance_before: 5000000,
    finance_after: 3800000,
    annual_wage_bill_before: 1000,
    annual_wage_bill_after: 2000,
    annual_wage_budget: 50000,
    projected_wage_budget_usage_pct: 4,
    exceeds_transfer_budget: false,
    exceeds_finance: false,
    ...overrides,
  };
}

describe("TransferBidModal", () => {
  it("renders the active negotiation state for an existing offer", () => {
    render(
      <TransferBidModal
        bidTarget={createPlayer()}
        teams={[createTeam(), createTeam({ id: "team-2", name: "Seller FC" })]}
        bidAmount="1.5"
        onBidAmountChange={vi.fn()}
        myTeam={createTeam()}
        bidFee={1500000}
        bidProjection={createProjection()}
        bidFeedback={{
          mood: "firm",
          headline_key: "headline",
          detail_key: "detail",
          tension: 40,
          patience: 70,
          round: 2,
          params: { fee: "1500000" },
        }}
        activeBidOffer={createOffer()}
        hasExistingOffer={true}
        bidResult={"counter_offer"}
        bidLoading={false}
        bidSubmitDisabled={false}
        onSubmit={vi.fn()}
        onClose={vi.fn()}
      />,
    );

    expect(screen.getByText("John Smith")).toBeInTheDocument();
    expect(screen.getByText("Talks are still live with this club.")).toBeInTheDocument();
    expect(screen.getByText("Recent exchange")).toBeInTheDocument();
    expect(screen.getByText("Bid countered")).toBeInTheDocument();
    expect(screen.getByText(/Transfer budget/)).toBeInTheDocument();
  });

  it("wires input, submit, and close interactions through props", () => {
    const onBidAmountChange = vi.fn();
    const onSubmit = vi.fn();
    const onClose = vi.fn();

    render(
      <TransferBidModal
        bidTarget={createPlayer()}
        teams={[createTeam(), createTeam({ id: "team-2", name: "Seller FC" })]}
        bidAmount="1.0"
        onBidAmountChange={onBidAmountChange}
        myTeam={createTeam()}
        bidFee={1000000}
        bidProjection={createProjection()}
        bidFeedback={null}
        activeBidOffer={null}
        hasExistingOffer={false}
        bidResult={null}
        bidLoading={false}
        bidSubmitDisabled={false}
        onSubmit={onSubmit}
        onClose={onClose}
      />,
    );

    fireEvent.change(screen.getByLabelText("Bid Amount"), {
      target: { value: "2.0" },
    });
    fireEvent.click(screen.getByRole("button", { name: "Submit Bid" }));
    fireEvent.click(screen.getByRole("button", { name: "Close" }));

    expect(onBidAmountChange).toHaveBeenCalledWith("2.0");
    expect(onSubmit).toHaveBeenCalledTimes(1);
    expect(onClose).toHaveBeenCalledTimes(1);
    expect(screen.getByRole("button", { name: "Submit Bid" })).not.toBeDisabled();
  });
});