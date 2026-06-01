import { fireEvent, render, screen } from "@testing-library/react";
import { describe, expect, it, vi } from "vitest";

import type {
  PlayerData,
  TeamData,
  TransferOfferData,
} from "../../store/gameStore";
import type { TransferNegotiationResponseData } from "../../services/transfersService";
import TransferCounterOfferModal from "./TransferCounterOfferModal";

vi.mock("react-i18next", () => ({
  useTranslation: () => ({
    t: (key: string, params?: Record<string, string | number>) => {
      if (key === "transfers.counterOffer") return "Counter Offer";
      if (key === "transfers.currentOffer") return `Current offer ${params?.fee}`;
      if (key === "transfers.resumeNegotiationHint") return "Talks are still live with this club.";
      if (key === "transfers.counterAmount") return "Counter Amount";
      if (key === "transfers.negotiationPulse") return "Negotiation pulse";
      if (key === "transfers.negotiationRound") return `Round ${params?.count}`;
      if (key === "transfers.negotiationPatience") return "Patience";
      if (key === "transfers.negotiationTension") return "Tension";
      if (key === "transfers.negotiationHistory") return "Recent exchange";
      if (key === "transfers.lastCounterLabel") return "Your last counter";
      if (key === "transfers.currentOfferLabel") return "Their current offer";
      if (key === "transfers.counterAccepted") return "Counter accepted";
      if (key === "transfers.counterRejected") return "Counter rejected";
      if (key === "transfers.counterCountered") return "Counter countered";
      if (key === "transfers.submitCounter") return "Submit Counter";
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
    team_id: "team-1",
    retired: false,
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
    from_team_id: "team-2",
    fee: 1400000,
    wage_offered: 0,
    last_manager_fee: 1600000,
    negotiation_round: 2,
    suggested_counter_fee: 1550000,
    status: "Pending",
    date: "2026-08-01",
    ...overrides,
  };
}

function createCounterTarget() {
  return {
    player: createPlayer(),
    offerId: "offer-1",
    fromTeamId: "team-2",
    fee: 1400000,
  };
}

describe("TransferCounterOfferModal", () => {
  it("renders the current offer context and negotiation history", () => {
    render(
      <TransferCounterOfferModal
        counterTarget={createCounterTarget()}
        teams={[createTeam(), createTeam({ id: "team-2", name: "Buyer FC" })]}
        counterAmount="1600000"
        onCounterAmountChange={vi.fn()}
        counterFeedback={{
          mood: "firm",
          headline_key: "headline",
          detail_key: "detail",
          tension: 42,
          patience: 64,
          round: 2,
          params: { fee: "1550000" },
        }}
        activeCounterOffer={createOffer()}
        counterResult={"counter_offer" as TransferNegotiationResponseData["decision"]}
        counterError={"Negotiation warning"}
        counterLoading={false}
        onSubmit={vi.fn()}
        onClose={vi.fn()}
      />,
    );

    expect(screen.getByText("John Smith")).toBeInTheDocument();
    expect(screen.getByText(/Current offer/)).toBeInTheDocument();
    expect(screen.getByText("Talks are still live with this club.")).toBeInTheDocument();
    expect(screen.getByText("Recent exchange")).toBeInTheDocument();
    expect(screen.getByText("Counter countered")).toBeInTheDocument();
    expect(screen.getByText("Negotiation warning")).toBeInTheDocument();
    expect(screen.getByText("€1,600,000")).toBeInTheDocument();
  });

  it("wires counter amount, submit, and close interactions through props", () => {
    const onCounterAmountChange = vi.fn();
    const onSubmit = vi.fn();
    const onClose = vi.fn();

    render(
      <TransferCounterOfferModal
        counterTarget={createCounterTarget()}
        teams={[createTeam(), createTeam({ id: "team-2", name: "Buyer FC" })]}
        counterAmount="1400000"
        onCounterAmountChange={onCounterAmountChange}
        counterFeedback={null}
        activeCounterOffer={null}
        counterResult={null}
        counterError={null}
        counterLoading={false}
        onSubmit={onSubmit}
        onClose={onClose}
      />,
    );

    fireEvent.change(screen.getByLabelText("Counter Amount"), {
      target: { value: "1800000" },
    });
    fireEvent.click(screen.getByRole("button", { name: "Submit Counter" }));
    fireEvent.click(screen.getByRole("button", { name: "Close" }));

    expect(onCounterAmountChange).toHaveBeenCalledWith("1800000");
    expect(onSubmit).toHaveBeenCalledTimes(1);
    expect(onClose).toHaveBeenCalledTimes(1);
  });
});