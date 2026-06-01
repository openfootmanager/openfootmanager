import { fireEvent, render, screen, waitFor } from "@testing-library/react";
import { beforeEach, describe, expect, it, vi } from "vitest";
import { invoke } from "@tauri-apps/api/core";

import type { GameStateData, PlayerData, StaffData, TeamData } from "../../store/gameStore";
import TransfersTab from "./TransfersTab";

vi.mock("@tauri-apps/api/core", () => ({
  invoke: vi.fn(),
}));

vi.mock("../../utils/backendI18n", () => ({
  resolveBackendError: (error: unknown) =>
    error instanceof Error ? error.message : String(error),
}));

vi.mock("react-i18next", () => ({
  useTranslation: () => ({
    t: (key: string, params?: Record<string, string | number>) => {
      if (key === "finances.perWeekSuffix") return "/wk";
      if (key === "common.nResults") return `${params?.count} results`;
      if (key === "common.action") return "Action";
      if (key === "common.viewTeam") return "View team";
      if (key === "common.freeAgent") return "Free Agent";
      if (key === "transfers.transferMarket") return "Transfer Market";
      if (key === "transfers.freeAgents") return "Free Agents";
      if (key === "transfers.offers") return "Offers";
      if (key === "transfers.noFreeAgents") return "No free agents available.";
      if (key === "transfers.counterOffer") return "Counter Offer";
      if (key === "transfers.counterAmount") return "Counter Amount";
      if (key === "transfers.submitCounter") return "Submit Counter";
      if (key === "transfers.close") return "Close";
      if (key === "transfers.counter") return "Counter";
      if (key === "transfers.offerContract") return "Offer Contract";
      if (key === "transfers.bid") return "Bid";
      if (key === "transfers.makeBid") return "Make Transfer Bid";
      if (key === "transfers.bidAmount") return "Bid Amount (€M)";
      if (key === "transfers.submitBid") return "Submit Bid";
      if (key === "transfers.bidImpactTitle") return "Projected impact";
      if (key === "transfers.bidImpactTransferBudget")
        return `Transfer budget ${params?.before} -> ${params?.after}`;
      if (key === "transfers.bidImpactBalance")
        return `Club balance ${params?.before} -> ${params?.after}`;
      if (key === "transfers.bidImpactWagePressure")
        return `Projected wage budget usage ${params?.percent}%`;
      if (key === "transfers.bidImpactOverTransferBudget")
        return "This bid exceeds your transfer budget";
      if (key === "transfers.bidImpactOverBalance")
        return "This bid would push the club into debt";
      if (key === "transfers.resumeNegotiationHint") return "Talks are still live with this club.";
      if (key === "transfers.resumeNegotiationHeadline") return "The other club are waiting for your next move.";
      if (key === "transfers.resumeNegotiationDetail") return `Their last signal pointed toward ${params?.fee}.`;
      if (key === "transfers.negotiationHistory") return "Recent exchange";
      if (key === "transfers.lastBidLabel") return "Your last bid";
      if (key === "transfers.lastClubSignalLabel") return "Their last signal";
      if (key === "transfers.lastCounterLabel") return "Your last counter";
      if (key === "transfers.currentOfferLabel") return "Their current offer";
      if (key === "transfers.offerStatusPending") return "Live";
      if (key === "transfers.offerStatusAccepted") return "Accepted";
      if (key === "transfers.offerStatusRejected") return "Rejected";
      if (key === "transfers.offerStatusWithdrawn") return "Talks cooled off";
      if (key === "transfers.negotiationExpiredError") return "Talks cooled off before you could answer. Start a new negotiation if the club comes back.";
      if (key === "transfers.acceptOffer") return "Accept";
      if (key === "transfers.rejectOffer") return "Reject";
      if (key === "transfers.negotiationPulse") return "Negotiation pulse";
      if (key === "transfers.negotiationRound") return `Round ${params?.count}`;
      if (key === "transfers.negotiationPatience") return "Patience";
      if (key === "transfers.negotiationTension") return "Tension";
      if (key === "transfers.counterCountered") return "They pushed back with a lower number.";
      if (key === "transfers.transferFeedbackCounterHeadline") return "They want more before shaking hands.";
      if (key === "transfers.transferFeedbackCounterDetail") return `The bid was close enough to keep talking, but their side are signalling a price nearer ${params?.fee}.`;
      if (key === "squad.viewProfile") return "View profile";
      if (key === "squad.addToTransferList") return "Add to transfer list";
      if (key === "squad.removeFromTransferList") return "Remove from transfer list";
      if (key === "squad.addToLoanList") return "Add to loan list";
      if (key === "squad.removeFromLoanList") return "Remove from loan list";
      if (key === "scouting.scoutBtn") return "Scout";
      if (key === "scouting.scoutingInProgress") return "Scouting in progress";
      if (key === "scouting.noScoutsFree") return "No scouts free";
      if (key === "playerProfile.renewalWage") return "Offered Wage";
      if (key === "playerProfile.renewalLength") return "Contract Length";
      if (key === "playerProfile.renewalProjectionTitle") return "Projected financial impact";
      if (key === "playerProfile.renewalProjectionWageBill")
        return `Weekly wage bill ${params?.before} -> ${params?.after}`;
      if (key === "playerProfile.renewalProjectionBudgetUsage")
        return `Wage budget use ${params?.before}% -> ${params?.after}%`;
      if (key === "playerProfile.renewalProjectionRunway")
        return `Cash runway ${params?.before} -> ${params?.after}`;
      if (key === "playerProfile.renewalBudgetWarning") return "Budget warning";
      if (key === "playerProfile.renewalConversationTitle") return "Negotiation pulse";
      if (key === "playerProfile.renewalRound") return `Round ${params?.count}`;
      if (key === "playerProfile.renewalPatience") return "Patience";
      if (key === "playerProfile.renewalTension") return "Tension";
      if (key === "playerProfile.renewalSubmit") return "Submit Offer";
      if (key === "playerProfile.renewalAccepted") return "Offer accepted";
      if (key === "playerProfile.renewalRejected") return "Offer rejected";
      if (key === "playerProfile.renewalCounter")
        return `Wants more: ${params?.wage} for ${params?.years} years`;
      if (key === "playerProfile.renewalBlocked") return "Talks blocked";
      return key;
    },
    i18n: { language: "en" },
  }),
}));

const mockedInvoke = vi.mocked(invoke);

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
    transfer_listed: false,
    loan_listed: false,
    transfer_offers: [
      {
        id: "offer-1",
        from_team_id: "team-2",
        fee: 900000,
        wage_offered: 0,
        last_manager_fee: null,
        negotiation_round: 1,
        suggested_counter_fee: null,
        status: "Pending",
        date: "2026-08-01",
      },
    ],
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
    nationality: "England",
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

function createGameState(players: PlayerData[] = [createPlayer()]): GameStateData {
  return {
    clock: {
      current_date: "2026-08-01T12:00:00Z",
      start_date: "2026-07-01T12:00:00Z",
    },
    manager: {
      id: "manager-1",
      first_name: "Jane",
      last_name: "Doe",
      date_of_birth: "1980-01-01",
      nationality: "England",
      reputation: 50,
      satisfaction: 50,
      fan_approval: 50,
      team_id: "team-1",
      career_stats: {
        matches_managed: 0,
        wins: 0,
        draws: 0,
        losses: 0,
        trophies: 0,
        best_finish: null,
      },
      career_history: [],
    },
    teams: [
      createTeam(),
      createTeam({
        id: "team-2",
        name: "Buyer FC",
        short_name: "BUY",
        manager_id: null,
      }),
    ],
    players,
    staff: [],
    messages: [],
    news: [],
    league: {
      id: "league-1",
      name: "Premier Division",
      season: 1,
      fixtures: [],
      standings: [],
    },
    scouting_assignments: [],
    board_objectives: [],
  };
}

describe("TransfersTab", function (): void {
  beforeEach(function resetMocks(): void {
    mockedInvoke.mockReset();
    mockedInvoke.mockImplementation(
      async (command: string, payload?: any) => {
        if (command === "preview_transfer_bid_financial_impact") {
          const fee = Number(payload?.fee ?? 0);
          const transferBudgetBefore = 2000000;
          const financeBefore = 5000000;
          return {
            projection: {
              transfer_budget_before: transferBudgetBefore,
              transfer_budget_after: transferBudgetBefore - fee,
              finance_before: financeBefore,
              finance_after: financeBefore - fee,
              annual_wage_bill_before: 1000,
              annual_wage_bill_after: 2000,
              annual_wage_budget: 50000,
              projected_wage_budget_usage_pct: 4,
              exceeds_transfer_budget: transferBudgetBefore - fee < 0,
              exceeds_finance: financeBefore - fee < 0,
            },
          };
        }

        if (command === "preview_free_agent_contract_impact") {
          const wage = Number(payload?.weeklyWage ?? 0);
          return {
            projection: {
              current_annual_wage_bill: 0,
              projected_annual_wage_bill: wage,
              annual_wage_budget: 50000,
              annual_soft_cap: 55000,
              current_weekly_wage_spend: 0,
              projected_weekly_wage_spend: wage,
              current_cash_runway_weeks: 40,
              projected_cash_runway_weeks: 30,
              currently_over_budget: false,
              policy_allows: true,
            },
          };
        }

        return {};
      },
    );
  });

  it("submits a counter offer for a pending incoming bid and publishes the updated game", async function (): Promise<void> {
    const initialState = createGameState();
    const updatedState = createGameState([
      createPlayer({
        transfer_offers: [
          {
            id: "offer-1",
            from_team_id: "team-2",
            fee: 1200000,
            wage_offered: 0,
            last_manager_fee: 1200000,
            negotiation_round: 2,
            suggested_counter_fee: null,
            status: "Rejected",
            date: "2026-08-01",
          },
        ],
      }),
    ]);
    const onGameUpdate = vi.fn();

    mockedInvoke.mockResolvedValue({
      decision: "counter_offer",
      suggested_fee: 1150000,
      is_terminal: false,
      feedback: {
        mood: "firm",
        headline_key: "transfers.transferFeedbackCounterHeadline",
        detail_key: "transfers.transferFeedbackCounterDetail",
        tension: 63,
        patience: 54,
        round: 2,
        params: { fee: "1150000" },
      },
      game: updatedState,
    });

    render(
      <TransfersTab
        gameState={initialState}
        onSelectPlayer={vi.fn()}
        onSelectTeam={vi.fn()}
        onGameUpdate={onGameUpdate}
      />,
    );

    fireEvent.click(screen.getByRole("button", { name: /offers/i }));
    fireEvent.click(screen.getByRole("button", { name: /counter offer/i }));
    fireEvent.change(screen.getByLabelText(/counter amount/i), {
      target: { value: "1200000" },
    });
    fireEvent.click(screen.getByRole("button", { name: /submit counter/i }));

    await waitFor(function (): void {
      expect(mockedInvoke).toHaveBeenCalledWith("counter_offer", {
        playerId: "player-1",
        offerId: "offer-1",
        requestedFee: 1200000,
      });
    });

    expect(onGameUpdate).toHaveBeenCalledWith(updatedState);
    expect(screen.getByText("Negotiation pulse")).toBeInTheDocument();
    expect(
      screen.getByText("They want more before shaking hands."),
    ).toBeInTheDocument();
    expect(
      screen.getByText(
        "The bid was close enough to keep talking, but their side are signalling a price nearer €1,150,000.",
      ),
    ).toBeInTheDocument();
  });

  it("resumes an existing outgoing transfer negotiation when reopening the bid modal", async function (): Promise<void> {
    const state = createGameState([
      createPlayer({
        id: "player-market-1",
        team_id: "team-2",
        transfer_listed: true,
        transfer_offers: [
          {
            id: "offer-user-1",
            from_team_id: "team-1",
            fee: 900000,
            wage_offered: 0,
            last_manager_fee: 900000,
            negotiation_round: 2,
            suggested_counter_fee: 1150000,
            status: "Pending",
            date: "2026-08-01",
          },
        ],
      }),
    ]);

    render(
      <TransfersTab
        gameState={state}
        onSelectPlayer={vi.fn()}
        onSelectTeam={vi.fn()}
        onGameUpdate={vi.fn()}
      />,
    );

    fireEvent.click(screen.getByRole("button", { name: /transfer market/i }));
    fireEvent.click(screen.getByRole("button", { name: /^bid$/i }));

    await waitFor(() => {
      expect(mockedInvoke).toHaveBeenCalledWith(
        "preview_transfer_bid_financial_impact",
        {
          fee: 1150000,
          playerId: "player-market-1",
        },
      );
    });

    expect(screen.getByText("Talks are still live with this club.")).toBeInTheDocument();
    expect(screen.getByText("The other club are waiting for your next move.")).toBeInTheDocument();
    expect(screen.getByText("Their last signal pointed toward €1,150,000.")).toBeInTheDocument();
    expect(screen.getByText("Recent exchange")).toBeInTheDocument();
    expect(screen.getByText("Your last bid")).toBeInTheDocument();
    expect(screen.getByText("Their last signal")).toBeInTheDocument();
    expect(screen.getByText("Round 2")).toBeInTheDocument();
    expect(screen.getByDisplayValue("1.15")).toBeInTheDocument();
  });

  it("shows scout assignment errors inline on the transfer market", async function (): Promise<void> {
    const consoleErrorSpy = vi
      .spyOn(console, "error")
      .mockImplementation(() => { });
    try {
      const state = createGameState([
        createPlayer({
          id: "player-market-1",
          team_id: "team-2",
          transfer_listed: true,
          transfer_offers: [],
        }),
      ]);
      state.staff = [createScout()];

      mockedInvoke.mockRejectedValueOnce(
        new Error("Scout is already assigned to another scouting task."),
      );

      render(
        <TransfersTab
          gameState={state}
          onSelectPlayer={vi.fn()}
          onSelectTeam={vi.fn()}
          onGameUpdate={vi.fn()}
        />,
      );

      fireEvent.click(screen.getByRole("button", { name: /transfer market/i }));
      const playerRow = screen.getByText("John Smith").closest("tr");
      expect(playerRow).not.toBeNull();

      fireEvent.contextMenu(playerRow as HTMLTableRowElement);
      fireEvent.click(screen.getByRole("button", { name: "Scout" }));

      await waitFor(() => {
        expect(screen.getByRole("alert")).toHaveTextContent(
          "Scout is already assigned to another scouting task.",
        );
      });
    } finally {
      consoleErrorSpy.mockRestore();
    }
  });

  it("resumes an incoming transfer negotiation when reopening the counter-offer modal", function (): void {
    const state = createGameState([
      createPlayer({
        transfer_offers: [
          {
            id: "offer-1",
            from_team_id: "team-2",
            fee: 1150000,
            wage_offered: 0,
            last_manager_fee: 1200000,
            negotiation_round: 2,
            suggested_counter_fee: 1150000,
            status: "Pending",
            date: "2026-08-01",
          },
        ],
      }),
    ]);

    render(
      <TransfersTab
        gameState={state}
        onSelectPlayer={vi.fn()}
        onSelectTeam={vi.fn()}
        onGameUpdate={vi.fn()}
      />,
    );

    fireEvent.click(screen.getByRole("button", { name: /offers/i }));
    fireEvent.click(screen.getByRole("button", { name: /counter offer/i }));

    expect(screen.getByText("Talks are still live with this club.")).toBeInTheDocument();
    expect(screen.getByText("The other club are waiting for your next move.")).toBeInTheDocument();
    expect(screen.getByText("Their last signal pointed toward €1,150,000.")).toBeInTheDocument();
    expect(screen.getByText("Recent exchange")).toBeInTheDocument();
    expect(screen.getByText("Your last counter")).toBeInTheDocument();
    expect(screen.getByText("Their current offer")).toBeInTheDocument();
    expect(screen.getByText("Round 2")).toBeInTheDocument();
    expect(screen.getByDisplayValue("1150000")).toBeInTheDocument();
  });

  it("shows a localized message when a counter-offer expires before submission", async function (): Promise<void> {
    mockedInvoke.mockRejectedValue("Offer not found or not pending");

    render(
      <TransfersTab
        gameState={createGameState()}
        onSelectPlayer={vi.fn()}
        onSelectTeam={vi.fn()}
        onGameUpdate={vi.fn()}
      />,
    );

    fireEvent.click(screen.getByRole("button", { name: /offers/i }));
    fireEvent.click(screen.getByRole("button", { name: /counter offer/i }));
    fireEvent.click(screen.getByRole("button", { name: /submit counter/i }));

    await waitFor(function (): void {
      expect(
        screen.getByText(
          "Talks cooled off before you could answer. Start a new negotiation if the club comes back.",
        ),
      ).toBeInTheDocument();
    });
  });

  it("renders withdrawn transfer offers with a localized cooled-off status", function (): void {
    const state = createGameState([
      createPlayer({
        transfer_offers: [
          {
            id: "offer-withdrawn",
            from_team_id: "team-2",
            fee: 850000,
            wage_offered: 0,
            last_manager_fee: 900000,
            negotiation_round: 2,
            suggested_counter_fee: null,
            status: "Withdrawn",
            date: "2026-08-01",
          },
        ],
      }),
    ]);

    render(
      <TransfersTab
        gameState={state}
        onSelectPlayer={vi.fn()}
        onSelectTeam={vi.fn()}
        onGameUpdate={vi.fn()}
      />,
    );

    fireEvent.click(screen.getByRole("button", { name: /offers/i }));

    expect(screen.getByText(/Talks cooled off/i)).toBeInTheDocument();
  });

  it("shows bid impact preview and blocks impossible bids", async function (): Promise<void> {
    const state = createGameState([
      createPlayer({
        id: "player-market-1",
        team_id: "team-2",
        transfer_listed: true,
        transfer_offers: [],
        market_value: 1800000,
      }),
    ]);

    render(
      <TransfersTab
        gameState={state}
        onSelectPlayer={vi.fn()}
        onSelectTeam={vi.fn()}
      />,
    );

    fireEvent.click(screen.getByRole("button", { name: /transfer market/i }));
    fireEvent.click(screen.getByRole("button", { name: /^bid$/i }));
    fireEvent.change(screen.getByLabelText(/bid amount/i), {
      target: { value: "9.0" },
    });

    await waitFor(function (): void {
      expect(screen.getByText("Projected impact")).toBeInTheDocument();
      expect(
        screen.getByText("This bid exceeds your transfer budget"),
      ).toBeInTheDocument();
      expect(screen.getByRole("button", { name: /submit bid/i })).toBeDisabled();
    });
  });

  it("shows free agents in a dedicated view and opens the contract modal", async function (): Promise<void> {
    const state = createGameState([
      createPlayer({
        id: "free-agent-1",
        team_id: null,
        contract_end: null,
        transfer_offers: [],
        market_value: 600000,
      }),
    ]);

    render(
      <TransfersTab
        gameState={state}
        onSelectPlayer={vi.fn()}
        onSelectTeam={vi.fn()}
        onGameUpdate={vi.fn()}
      />,
    );

    fireEvent.click(screen.getByRole("button", { name: /free agents/i }));

    expect(screen.getByText("John Smith")).toBeInTheDocument();
    expect(screen.getAllByText("Free Agent").length).toBeGreaterThan(0);

    fireEvent.click(screen.getByRole("button", { name: /offer contract/i }));

    await waitFor(function (): void {
      expect(screen.getByText("Projected financial impact")).toBeInTheDocument();
      expect(screen.getByLabelText("Offered Wage")).toBeInTheDocument();
    });
  });

  it("offers transfer-list actions from the my-list context menu", async function (): Promise<void> {
    const gameState = createGameState([
      createPlayer({ transfer_listed: true }),
    ]);
    const onGameUpdate = vi.fn();

    mockedInvoke.mockResolvedValueOnce(gameState);

    render(
      <TransfersTab
        gameState={gameState}
        onSelectPlayer={vi.fn()}
        onSelectTeam={vi.fn()}
        onGameUpdate={onGameUpdate}
      />,
    );

    const playerRow = screen.getByText("John Smith").closest("tr");
    expect(playerRow).not.toBeNull();

    fireEvent.contextMenu(playerRow as HTMLTableRowElement);
    fireEvent.click(
      screen.getByRole("button", { name: "Remove from transfer list" }),
    );

    await waitFor(function (): void {
      expect(mockedInvoke).toHaveBeenCalledWith("toggle_transfer_list", {
        playerId: "player-1",
      });
      expect(onGameUpdate).toHaveBeenCalledWith(gameState);
    });
  });
});
