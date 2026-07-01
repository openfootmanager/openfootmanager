import {
  fireEvent,
  render,
  screen,
  waitFor,
} from "@testing-library/react";
import { beforeEach, describe, expect, it, vi } from "vitest";
import { invoke, isTauri } from "@tauri-apps/api/core";

import type {
  GameStateData,
  PlayerData,
  StaffData,
  TeamData,
} from "../../store/gameStore";
import TransfersTab from "./TransfersTab";

vi.mock("@tauri-apps/api/core", () => ({
  convertFileSrc: vi.fn((path: string) => path),
  invoke: vi.fn(),
  isTauri: vi.fn(() => false),
}));

vi.mock("../../utils/backendI18n", () => ({
  resolveBackendError: (error: unknown) =>
    error instanceof Error ? error.message : String(error),
}));

vi.mock("react-i18next", () => ({
  useTranslation: () => ({
    t: (key: string, params?: Record<string, string | number>) => {
      if (key === "finances.perWeekSuffix") return "/wk";
      if (key === "finances.perYearSuffix") return "/yr";
      if (key === "common.nResults") return `${params?.count} results`;
      if (key === "common.action") return "Action";
      if (key === "common.viewTeam") return "View team";
      if (key === "common.freeAgent") return "Free Agent";
      if (key === "scouting.nextPage") return "Next page";
      if (key === "scouting.previousPage") return "Previous page";
      if (key === "players.showingRange")
        return `Showing ${params?.from}-${params?.to} of ${params?.total}`;
      if (key === "dashboard.players") return "Players";
      if (key === "transfers.myTransferList") return "My Transfer List";
      if (key === "transfers.transferMarket") return "Transfer Market";
      if (key === "transfers.myTransferList") return "My Transfer List";
      if (key === "transfers.freeAgents") return "Free Agents";
      if (key === "transfers.transfer") return "TRANSFER";
      if (key === "transfers.loan") return "LOAN";
      if (key === "transfers.loanMarket") return "Loan Market";
      if (key === "transfers.offers") return "Offers";
      if (key === "transfers.noFreeAgents") return "No free agents available.";
      if (key === "transfers.counterOffer") return "Counter Offer";
      if (key === "transfers.counterAmount") return "Counter Amount";
      if (key === "transfers.submitCounter") return "Submit Counter";
      if (key === "transfers.close") return "Close";
      if (key === "transfers.counter") return "Counter";
      if (key === "transfers.offerContract") return "Offer Contract";
      if (key === "transfers.makeOffer") return "Make Offer";
      if (key === "transfers.bid") return "Bid";
      if (key === "transfers.loanOffer") return "Loan Offer";
      if (key === "transfers.counterLoanOffer") return "Counter Loan Offer";
      if (key === "transfers.makeBid") return "Make Transfer Bid";
      if (key === "transfers.makeLoanOffer") return "Make Loan Offer";
      if (key === "transfers.bidAmount") return "Bid Amount (€M)";
      if (key === "transfers.loanEndDate") return "Loan End Date";
      if (key === "transfers.loanPeriod") return "Loan Length";
      if (key === "transfers.loanPeriodThreeMonths") return "3 months";
      if (key === "transfers.loanPeriodJanuaryWindow")
        return "Until January window";
      if (key === "transfers.loanPeriodEndOfSeason")
        return "Until end of season";
      if (key === "transfers.loanPeriodTwelveMonths") return "12 months";
      if (key === "transfers.loanPeriodCurrentOffer")
        return "Current offer date";
      if (key === "transfers.loanEndsOn")
        return `Loan ends on ${params?.endDate}`;
      if (key === "transfers.noLoanPeriodAvailable")
        return "No valid loan length.";
      if (key === "transfers.loanPeriodUnavailableRules")
        return "outside loan rules";
      if (key === "transfers.loanPeriodUnavailableContract")
        return "contract expires first";
      if (key === "transfers.loanWageContribution")
        return "Wage Contribution (%)";
      if (key === "transfers.loanWageContributionManual")
        return "Manual percentage";
      if (key === "transfers.loanWageSummary")
        return `${params?.percent}% wages: ${params?.wage}`;
      if (key === "transfers.loanToBuyOption") return "Loan-to-buy option";
      if (key === "transfers.loanToBuyOptionDesc")
        return "Include a permanent purchase clause.";
      if (key === "transfers.buyOptionFee") return "Buy Option Fee";
      if (key === "transfers.buyOptionFeeShort") return `Option ${params?.fee}`;
      if (key === "transfers.loanBuyOptionSummary")
        return `Permanent option at ${params?.fee}`;
      if (key === "transfers.exerciseBuyOption") return "Exercise Option";
      if (key === "transfers.submitLoanOffer") return "Submit Loan Offer";
      if (key === "transfers.submitLoanCounter") return "Submit Counter";
      if (key === "transfers.loanOfferAccepted") return "Loan accepted";
      if (key === "transfers.loanOfferRejected") return "Loan rejected";
      if (key === "transfers.loanCounterAccepted")
        return "Loan counter accepted";
      if (key === "transfers.loanCounterRejected")
        return "Loan counter rejected";
      if (key === "transfers.loanCounterCountered")
        return "They pushed back with adjusted loan terms.";
      if (key === "transfers.loanCounterSuggestedTerms")
        return `Suggested terms: ${params?.percent}% wages until ${params?.endDate}`;
      if (key === "transfers.loanCounterSuggestedBuyOption")
        return `Suggested buy option: ${params?.fee}`;
      if (key === "transfers.loanOfferTerms")
        return `Loan ${params?.percent}% wages until ${params?.endDate}`;
      if (key === "transfers.acceptLoanOffer") return "Accept Loan";
      if (key === "transfers.rejectLoanOffer") return "Reject Loan";
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
      if (key === "transfers.resumeNegotiationHint")
        return "Talks are still live with this club.";
      if (key === "transfers.resumeNegotiationHeadline")
        return "The other club are waiting for your next move.";
      if (key === "transfers.resumeNegotiationDetail")
        return `Their last signal pointed toward ${params?.fee}.`;
      if (key === "transfers.negotiationHistory") return "Recent exchange";
      if (key === "transfers.lastBidLabel") return "Your last bid";
      if (key === "transfers.lastClubSignalLabel") return "Their last signal";
      if (key === "transfers.lastCounterLabel") return "Your last counter";
      if (key === "transfers.currentOfferLabel") return "Their current offer";
      if (key === "transfers.offerStatusPending") return "Live";
      if (key === "transfers.offerStatusPendingRegistration")
        return "Pending registration";
      if (key === "transfers.offerStatusAccepted") return "Accepted";
      if (key === "transfers.offerStatusRejected") return "Rejected";
      if (key === "transfers.offerStatusWithdrawn") return "Talks cooled off";
      if (key === "transfers.negotiationExpiredError")
        return "Talks cooled off before you could answer. Start a new negotiation if the club comes back.";
      if (key === "transfers.acceptOffer") return "Accept";
      if (key === "transfers.rejectOffer") return "Reject";
      if (key === "transfers.negotiationPulse") return "Negotiation pulse";
      if (key === "transfers.negotiationRound") return `Round ${params?.count}`;
      if (key === "transfers.negotiationPatience") return "Patience";
      if (key === "transfers.negotiationTension") return "Tension";
      if (key === "transfers.counterCountered")
        return "They pushed back with a lower number.";
      if (key === "transfers.transferFeedbackCounterHeadline")
        return "They want more before shaking hands.";
      if (key === "transfers.transferFeedbackCounterDetail")
        return `The bid was close enough to keep talking, but their side are signalling a price nearer ${params?.fee}.`;
      if (key === "transfers.transferFeedbackScheduledHeadline")
        return "Deal agreed for the next registration window.";
      if (key === "transfers.transferFeedbackScheduledDetail")
        return `The terms are accepted. Registration is scheduled for ${params?.date}.`;
      if (key === "season.windowClosed") return "Transfer window closed";
      if (key === "season.windowOpensInDays")
        return `${params?.count} days until the window opens`;
      if (key === "transfers.loanWindowClosedNoticeTitle")
        return "Transfer window closed";
      if (key === "transfers.loanWindowClosedNoticeDetail")
        return `If accepted, the loan will be registered on ${params?.date}.`;
      if (key === "transfers.loanWindowClosedUnavailableDetail")
        return "Loan registration is unavailable until the next transfer window is scheduled.";
      if (key === "transfers.loanOfferScheduled")
        return `Loan agreed. Registration scheduled for ${params?.date}.`;
      if (key === "transfers.loanCounterScheduled")
        return `Loan agreed. Registration scheduled for ${params?.date}.`;
      if (key === "squad.viewProfile") return "View profile";
      if (key === "squad.addToTransferList") return "Add to transfer list";
      if (key === "squad.removeFromTransferList")
        return "Remove from transfer list";
      if (key === "squad.addToLoanList") return "Add to loan list";
      if (key === "squad.removeFromLoanList") return "Remove from loan list";
      if (key === "scouting.scoutBtn") return "Scout";
      if (key === "scouting.scoutingInProgress") return "Scouting in progress";
      if (key === "scouting.noScoutsFree") return "No scouts free";
      if (key === "playerProfile.renewalWage") return "Offered Wage";
      if (key === "playerProfile.renewalLength") return "Contract Length";
      if (key === "playerProfile.renewalProjectionTitle")
        return "Projected financial impact";
      if (key === "playerProfile.renewalProjectionWageBill")
        return `Weekly wage bill ${params?.before} -> ${params?.after}`;
      if (key === "playerProfile.renewalProjectionBudgetUsage")
        return `Wage budget use ${params?.before}% -> ${params?.after}%`;
      if (key === "playerProfile.renewalProjectionRunway")
        return `Cash runway ${params?.before} -> ${params?.after}`;
      if (key === "playerProfile.renewalBudgetWarning") return "Budget warning";
      if (key === "playerProfile.renewalConversationTitle")
        return "Negotiation pulse";
      if (key === "playerProfile.renewalRound") return `Round ${params?.count}`;
      if (key === "playerProfile.renewalPatience") return "Patience";
      if (key === "playerProfile.renewalTension") return "Tension";
      if (key === "playerProfile.renewalSubmit") return "Submit Offer";
      if (key === "playerProfile.renewalAccepted") return "Offer accepted";
      if (key === "playerProfile.renewalRejected") return "Offer rejected";
      if (key === "playerProfile.renewalCounter")
        return `Wants more: ${params?.wage} for ${params?.years} years`;
      if (key === "playerProfile.renewalBlocked") return "Talks blocked";
      if (key === "be.error.transfers.playerAlreadyLoaned")
        return "Player already loaned";
      if (params && typeof params === "object" && "defaultValue" in params) {
        return String(params.defaultValue);
      }
      return key;
    },
    i18n: { language: "en" },
  }),
}));

const mockedInvoke = vi.mocked(invoke);
const mockedIsTauri = vi.mocked(isTauri);

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
      judgingAbility: 65,
      judgingPotential: 70,
      physiotherapy: 10,
    },
    team_id: "team-1",
    specialization: null,
    wage: 1000,
    contract_end: "2027-06-30",
    ...overrides,
  };
}

function createGameState(
  players: PlayerData[] = [createPlayer()],
): GameStateData {
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
    season_context: {
      phase: "InSeason",
      season_start: "2026-07-01",
      season_end: "2027-05-31",
      days_until_season_start: null,
      transfer_window: {
        status: "Open",
        opens_on: "2026-06-01",
        closes_on: "2026-08-31",
        days_until_opens: null,
        days_remaining: 30,
      },
    },
  };
}

describe("TransfersTab", function (): void {
  beforeEach(function resetMocks(): void {
    mockedInvoke.mockReset();
    mockedIsTauri.mockReturnValue(false);
    mockedInvoke.mockImplementation(async (command: string, payload?: any) => {
      if (command === "generate_player_portrait") {
        const playerId = String(payload?.request?.playerId ?? "player");
        return {
          generator: "test",
          cacheKey: playerId,
          sourceId: playerId,
          cachePath: `/tmp/${playerId}.png`,
          dataUrl: null,
          generated: true,
          renderMs: 10,
          elapsedMs: 10,
          width: 128,
          height: 128,
        };
      }

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
    });
  });

  it("renders a dual transfer and loan listed player once with both status badges", function (): void {
    render(
      <TransfersTab
        gameState={createGameState([
          createPlayer({
            id: "dual-listed",
            full_name: "Dual Listed",
            transfer_listed: true,
            loan_listed: true,
          }),
        ])}
        onSelectPlayer={vi.fn()}
        onSelectTeam={vi.fn()}
        onGameUpdate={vi.fn()}
      />,
    );

    fireEvent.click(screen.getByRole("button", { name: /my transfer list/i }));

    expect(screen.getAllByText("Dual Listed")).toHaveLength(1);
    expect(screen.getByText("TRANSFER")).toBeInTheDocument();
    expect(screen.getByText("LOAN")).toBeInTheDocument();
    expect(screen.getByText(/My Transfer List \(1\)/)).toBeInTheDocument();
  });

  it("paginates the transfer players list instead of mounting every market row", function (): void {
    const marketPlayers = Array.from({ length: 65 }, (_, index) =>
      createPlayer({
        id: `market-player-${index + 1}`,
        full_name: `Market Player ${index + 1}`,
        match_name: `M. Player ${index + 1}`,
        team_id: "team-2",
        transfer_listed: true,
        transfer_offers: [],
      }),
    );

    render(
      <TransfersTab
        gameState={createGameState(marketPlayers)}
        onSelectPlayer={vi.fn()}
        onSelectTeam={vi.fn()}
        onGameUpdate={vi.fn()}
      />,
    );

    expect(screen.getAllByText(/^Market Player \d+$/)).toHaveLength(30);
    expect(screen.getByText("Market Player 30")).toBeInTheDocument();
    expect(screen.queryByText("Market Player 31")).not.toBeInTheDocument();
    expect(screen.getByText("Showing 1-30 of 65")).toBeInTheDocument();

    fireEvent.click(screen.getByRole("button", { name: "Next page" }));

    expect(screen.queryByText("Market Player 1")).not.toBeInTheDocument();
    expect(screen.getByText("Market Player 31")).toBeInTheDocument();
    expect(screen.getByText("Showing 31-60 of 65")).toBeInTheDocument();
  });

  it("starts runtime portrait loading for the visible transfer market page", async function (): Promise<void> {
    mockedIsTauri.mockReturnValue(true);
    const marketPlayers = Array.from({ length: 35 }, (_, index) =>
      createPlayer({
        id: `portrait-player-${index + 1}`,
        full_name: `Portrait Player ${index + 1}`,
        match_name: `P. Player ${index + 1}`,
        team_id: "team-2",
        transfer_listed: true,
        transfer_offers: [],
      }),
    );

    render(
      <TransfersTab
        gameState={createGameState(marketPlayers)}
        onSelectPlayer={vi.fn()}
        onSelectTeam={vi.fn()}
        onGameUpdate={vi.fn()}
      />,
    );

    await waitFor(function (): void {
      const portraitCalls = mockedInvoke.mock.calls.filter(
        ([command]) => command === "generate_player_portrait",
      );
      expect(portraitCalls).toHaveLength(30);
      expect(portraitCalls[0]?.[1]).toEqual({
        request: expect.objectContaining({ playerId: "portrait-player-1" }),
      });
      expect(
        portraitCalls.some(([, payload]) => {
          const portraitPayload = payload as
            { request?: { playerId?: string } } | undefined;
          return portraitPayload?.request?.playerId === "portrait-player-31";
        }),
      ).toBe(false);
    });
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

    expect(
      screen.getByText("Talks are still live with this club."),
    ).toBeInTheDocument();
    expect(
      screen.getByText("The other club are waiting for your next move."),
    ).toBeInTheDocument();
    expect(
      screen.getByText("Their last signal pointed toward €1,150,000."),
    ).toBeInTheDocument();
    expect(screen.getByText("Recent exchange")).toBeInTheDocument();
    expect(screen.getByText("Your last bid")).toBeInTheDocument();
    expect(screen.getByText("Their last signal")).toBeInTheDocument();
    expect(screen.getByText("Round 2")).toBeInTheDocument();
    expect(screen.getByDisplayValue("1.15")).toBeInTheDocument();
  });

  it("shows scout assignment errors inline on the player market", async function (): Promise<void> {
    const consoleErrorSpy = vi
      .spyOn(console, "error")
      .mockImplementation(() => {});
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

    expect(
      screen.getByText("Talks are still live with this club."),
    ).toBeInTheDocument();
    expect(
      screen.getByText("The other club are waiting for your next move."),
    ).toBeInTheDocument();
    expect(
      screen.getByText("Their last signal pointed toward €1,150,000."),
    ).toBeInTheDocument();
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

    fireEvent.click(screen.getByRole("button", { name: /^bid$/i }));
    fireEvent.change(screen.getByLabelText(/bid amount/i), {
      target: { value: "9.0" },
    });

    await waitFor(function (): void {
      expect(screen.getByText("Projected impact")).toBeInTheDocument();
      expect(
        screen.getByText("This bid exceeds your transfer budget"),
      ).toBeInTheDocument();
      expect(
        screen.getByRole("button", { name: /submit bid/i }),
      ).toBeDisabled();
    });
  });

  it("keeps the bid modal and deal workspace open after acceptance so the user can review the result", async function (): Promise<void> {
    const state = createGameState([
      createPlayer({
        id: "player-market-1",
        team_id: "team-2",
        transfer_listed: true,
        transfer_offers: [],
        market_value: 1000000,
      }),
    ]);
    const updatedState = createGameState([
      createPlayer({
        id: "player-market-1",
        team_id: "team-1",
        transfer_listed: false,
        transfer_offers: [],
        market_value: 1000000,
      }),
    ]);

    mockedInvoke.mockImplementation(async (command: string, payload?: any) => {
      if (command === "preview_transfer_bid_financial_impact") {
        const fee = Number(payload?.fee ?? 0);
        return {
          projection: {
            transfer_budget_before: 2000000,
            transfer_budget_after: 2000000 - fee,
            finance_before: 5000000,
            finance_after: 5000000 - fee,
            annual_wage_bill_before: 1000,
            annual_wage_bill_after: 2000,
            annual_wage_budget: 50000,
            projected_wage_budget_usage_pct: 4,
            exceeds_transfer_budget: false,
            exceeds_finance: false,
          },
        };
      }

      if (command === "make_transfer_bid") {
        return {
          decision: "accepted",
          suggested_fee: null,
          is_terminal: true,
          feedback: {
            mood: "positive",
            headline_key: "transfers.transferFeedbackAcceptedHeadline",
            detail_key: "transfers.transferFeedbackAcceptedDetail",
            tension: 20,
            patience: 80,
            round: 1,
            params: { fee: "1000000" },
          },
          game: updatedState,
        };
      }

      return {};
    });

    render(
      <TransfersTab
        gameState={state}
        onSelectPlayer={vi.fn()}
        onSelectTeam={vi.fn()}
        onGameUpdate={vi.fn()}
      />,
    );

    fireEvent.click(screen.getByRole("button", { name: /^bid$/i }));
    expect(
      screen.getByRole("dialog", { name: /john smith/i }),
    ).toBeInTheDocument();

    await waitFor(function (): void {
      expect(
        screen.getByRole("button", { name: /submit bid/i }),
      ).toBeEnabled();
    });
    fireEvent.click(screen.getByRole("button", { name: /submit bid/i }));

    await waitFor(function (): void {
      expect(mockedInvoke).toHaveBeenCalledWith("make_transfer_bid", {
        playerId: "player-market-1",
        fee: 1000000,
      });
    });

    // Modal must stay open on acceptance so the user can read the
    // confirmation. Any auto-close reintroduces the 2-second flicker
    // and the double-unmount of the deal workspace parent.
    await new Promise((resolve) => setTimeout(resolve, 50));
    expect(
      screen.getByRole("dialog", { name: /john smith/i }),
    ).toBeInTheDocument();

    fireEvent.click(screen.getByRole("button", { name: /^close$/i }));
    await waitFor(function (): void {
      expect(
        screen.queryByRole("dialog", { name: /john smith/i }),
      ).not.toBeInTheDocument();
    });
  });

  it("filters free agents in the player market and opens the contract modal", async function (): Promise<void> {
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

    fireEvent.click(screen.getByRole("button", { name: /free agent \(1\)/i }));

    expect(screen.getByText("John Smith")).toBeInTheDocument();
    expect(screen.getAllByText("Free Agent").length).toBeGreaterThan(0);

    fireEvent.click(screen.getByRole("button", { name: /offer contract/i }));

    await waitFor(function (): void {
      expect(
        screen.getByText("Projected financial impact"),
      ).toBeInTheDocument();
      expect(screen.getByLabelText("Offered Wage")).toBeInTheDocument();
    });
  });

  it("submits a loan offer from the player market", async function (): Promise<void> {
    const initialState = createGameState([
      createPlayer({
        id: "loan-target",
        team_id: "team-2",
        loan_listed: true,
        transfer_offers: [],
      }),
    ]);
    const updatedState = createGameState([
      createPlayer({
        id: "loan-target",
        team_id: "team-1",
        loan_listed: false,
        transfer_offers: [],
        active_loan: {
          parent_team_id: "team-2",
          loan_team_id: "team-1",
          start_date: "2026-08-01",
          end_date: "2027-01-01",
          wage_contribution_pct: 75,
        },
      }),
    ]);
    const onGameUpdate = vi.fn();

    mockedInvoke.mockResolvedValueOnce({
      decision: "accepted",
      offer_id: "loan-offer-1",
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

    fireEvent.click(screen.getByRole("button", { name: /loan \(1\)/i }));
    fireEvent.click(screen.getByRole("button", { name: /loan offer/i }));
    expect(screen.getByLabelText(/loan length/i)).toHaveValue("january_window");
    fireEvent.change(screen.getByLabelText(/wage contribution/i), {
      target: { value: "75" },
    });
    fireEvent.click(screen.getByRole("button", { name: /submit loan offer/i }));

    await waitFor(function (): void {
      expect(mockedInvoke).toHaveBeenCalledWith("make_loan_offer", {
        playerId: "loan-target",
        endDate: "2027-01-01",
        wageContributionPct: 75,
        buyOptionFee: null,
      });
    });
    expect(onGameUpdate).toHaveBeenCalledWith(updatedState);
  });

  it("explains deferred registration and allows closed-window loan negotiations", async function (): Promise<void> {
    const state = createGameState([
      createPlayer({
        id: "loan-target",
        team_id: "team-2",
        loan_listed: true,
        transfer_listed: true,
        transfer_offers: [],
      }),
    ]);
    state.season_context!.transfer_window = {
      status: "Closed",
      opens_on: "2027-01-01",
      closes_on: null,
      days_until_opens: 12,
      days_remaining: null,
    };

    const updatedState = structuredClone(state);
    updatedState.players[0].loan_listed = false;
    updatedState.players[0].loan_offers = [
      {
        id: "scheduled-loan",
        from_team_id: "team-1",
        parent_team_id: "team-2",
        start_date: "2027-01-01",
        end_date: "2027-06-01",
        wage_contribution_pct: 40,
        status: "PendingRegistration",
        date: "2026-12-20",
      },
    ];
    mockedInvoke.mockResolvedValueOnce({
      decision: "accepted",
      offer_id: "scheduled-loan",
      suggested_wage_contribution_pct: null,
      suggested_end_date: null,
      suggested_buy_option_fee: null,
      is_terminal: true,
      game: updatedState,
    });

    render(
      <TransfersTab
        gameState={state}
        onSelectPlayer={vi.fn()}
        onSelectTeam={vi.fn()}
        onGameUpdate={vi.fn()}
      />,
    );

    fireEvent.click(screen.getByRole("button", { name: /make offer/i }));
    expect(
      screen.getByRole("button", { name: /make transfer bid/i }),
    ).toBeEnabled();
    fireEvent.click(screen.getByRole("button", { name: /make loan offer/i }));

    expect(screen.getByRole("status")).toHaveTextContent(
      "Transfer window closed",
    );
    expect(screen.getByRole("status")).toHaveTextContent(
      "If accepted, the loan will be registered on",
    );
    expect(
      screen.getByRole("button", { name: /submit loan offer/i }),
    ).toBeEnabled();
    fireEvent.click(screen.getByRole("button", { name: /submit loan offer/i }));
    await waitFor(() => {
      expect(mockedInvoke).toHaveBeenCalledWith("make_loan_offer", {
        playerId: "loan-target",
        endDate: "2027-06-30",
        wageContributionPct: 100,
        buyOptionFee: null,
      });
    });

    fireEvent.click(screen.getByRole("button", { name: /close/i }));
    fireEvent.click(screen.getByRole("button", { name: /make offer/i }));
  });

  it("allows closed-window transfer bid submission when the next opening date is scheduled", async function (): Promise<void> {
    const state = createGameState([
      createPlayer({
        id: "transfer-target",
        team_id: "team-2",
        transfer_listed: true,
        loan_listed: true,
        transfer_offers: [],
        market_value: 1_000_000,
      }),
    ]);
    state.season_context!.transfer_window = {
      status: "Closed",
      opens_on: "2027-01-01",
      closes_on: null,
      days_until_opens: 12,
      days_remaining: null,
    };
    const updatedState = structuredClone(state);
    updatedState.players[0].transfer_offers = [
      {
        id: "scheduled-transfer",
        from_team_id: "team-1",
        fee: 1_000_000,
        wage_offered: 0,
        last_manager_fee: 1_000_000,
        negotiation_round: 1,
        suggested_counter_fee: null,
        status: "PendingRegistration",
        date: "2026-12-20",
        registration_date: "2027-01-01",
      },
    ];

    mockedInvoke.mockImplementation(async (command: string, payload?: any) => {
      if (command === "preview_transfer_bid_financial_impact") {
        const fee = Number(payload?.fee ?? 0);
        return {
          projection: {
            transfer_budget_before: 2_000_000,
            transfer_budget_after: 2_000_000 - fee,
            finance_before: 5_000_000,
            finance_after: 5_000_000 - fee,
            annual_wage_bill_before: 1_000,
            annual_wage_bill_after: 2_000,
            annual_wage_budget: 50_000,
            projected_wage_budget_usage_pct: 4,
            exceeds_transfer_budget: false,
            exceeds_finance: false,
          },
        };
      }

      if (command === "make_transfer_bid") {
        return {
          decision: "accepted",
          suggested_fee: null,
          is_terminal: true,
          registration_date: "2027-01-01",
          feedback: {
            mood: "positive",
            headline_key: "transfers.transferFeedbackScheduledHeadline",
            detail_key: "transfers.transferFeedbackScheduledDetail",
            tension: 20,
            patience: 80,
            round: 1,
            params: { date: "2027-01-01" },
          },
          game: updatedState,
        };
      }

      return {};
    });

    render(
      <TransfersTab
        gameState={state}
        onSelectPlayer={vi.fn()}
        onSelectTeam={vi.fn()}
        onGameUpdate={vi.fn()}
      />,
    );

    fireEvent.click(screen.getByRole("button", { name: /make offer/i }));
    fireEvent.click(screen.getByRole("button", { name: /make transfer bid/i }));

    await waitFor(() => {
      expect(screen.getByRole("button", { name: /submit bid/i })).toBeEnabled();
    });
    fireEvent.click(screen.getByRole("button", { name: /submit bid/i }));

    await waitFor(() => {
      expect(mockedInvoke).toHaveBeenCalledWith("make_transfer_bid", {
        playerId: "transfer-target",
        fee: 1_000_000,
      });
    });
  });

  it("locks transfer and loan routes when the opening date is stale", function (): void {
    const state = createGameState([
      createPlayer({
        id: "loan-target",
        team_id: "team-2",
        loan_listed: true,
        transfer_listed: true,
        transfer_offers: [],
      }),
    ]);
    state.clock.current_date = "2026-09-15T12:00:00Z";
    state.season_context!.transfer_window = {
      status: "Closed",
      opens_on: "2026-07-02",
      closes_on: "2026-08-31",
      days_until_opens: null,
      days_remaining: null,
    };

    render(
      <TransfersTab
        gameState={state}
        onSelectPlayer={vi.fn()}
        onSelectTeam={vi.fn()}
        onGameUpdate={vi.fn()}
      />,
    );

    fireEvent.click(screen.getByRole("button", { name: /make offer/i }));

    expect(
      screen.getByRole("button", { name: /make transfer bid/i }),
    ).toBeDisabled();
    expect(
      screen.getByRole("button", { name: /make loan offer/i }),
    ).toBeDisabled();
    expect(
      screen.getAllByText("Transfer window closed").length,
    ).toBeGreaterThan(0);
    expect(
      screen.queryByRole("button", { name: /submit loan offer/i }),
    ).not.toBeInTheDocument();
  });

  it("submits a loan offer with a buy option", async function (): Promise<void> {
    const initialState = createGameState([
      createPlayer({
        id: "loan-buy-target",
        team_id: "team-2",
        loan_listed: true,
        transfer_offers: [],
      }),
    ]);
    const updatedState = createGameState([
      createPlayer({
        id: "loan-buy-target",
        team_id: "team-1",
        loan_listed: false,
        transfer_offers: [],
        active_loan: {
          parent_team_id: "team-2",
          loan_team_id: "team-1",
          start_date: "2026-08-01",
          end_date: "2027-06-30",
          wage_contribution_pct: 75,
          buy_option_fee: 1250000,
        },
      }),
    ]);
    const onGameUpdate = vi.fn();

    mockedInvoke.mockResolvedValueOnce({
      decision: "accepted",
      offer_id: "loan-offer-1",
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

    fireEvent.click(screen.getByRole("button", { name: /loan \(1\)/i }));
    fireEvent.click(screen.getByRole("button", { name: /loan offer/i }));
    fireEvent.change(screen.getByLabelText(/loan length/i), {
      target: { value: "end_of_season" },
    });
    fireEvent.change(screen.getByLabelText(/wage contribution/i), {
      target: { value: "75" },
    });
    fireEvent.click(screen.getByLabelText(/loan-to-buy option/i));
    fireEvent.change(screen.getByLabelText(/buy option fee/i), {
      target: { value: "1250000" },
    });
    fireEvent.click(screen.getByRole("button", { name: /submit loan offer/i }));

    await waitFor(function (): void {
      expect(mockedInvoke).toHaveBeenCalledWith("make_loan_offer", {
        playerId: "loan-buy-target",
        endDate: "2027-06-30",
        wageContributionPct: 75,
        buyOptionFee: 1250000,
      });
    });
    expect(onGameUpdate).toHaveBeenCalledWith(updatedState);
  });

  it("accepts an incoming loan offer from the offers view", async function (): Promise<void> {
    const initialState = createGameState([
      createPlayer({
        id: "loan-owned",
        transfer_offers: [],
        loan_offers: [
          {
            id: "loan-offer-1",
            from_team_id: "team-2",
            parent_team_id: "team-1",
            start_date: "2026-08-01",
            end_date: "2027-01-01",
            wage_contribution_pct: 75,
            status: "Pending",
            date: "2026-08-01",
          },
        ],
      }),
    ]);
    const updatedState = createGameState([
      createPlayer({
        id: "loan-owned",
        team_id: "team-2",
        transfer_offers: [],
      }),
    ]);
    const onGameUpdate = vi.fn();

    mockedInvoke.mockResolvedValueOnce(updatedState);

    render(
      <TransfersTab
        gameState={initialState}
        onSelectPlayer={vi.fn()}
        onSelectTeam={vi.fn()}
        onGameUpdate={onGameUpdate}
      />,
    );

    fireEvent.click(screen.getByRole("button", { name: /offers/i }));
    expect(
      screen.getByText("Loan 75% wages until 2027-01-01 — Live"),
    ).toBeInTheDocument();
    fireEvent.click(screen.getByTitle("Accept Loan"));

    await waitFor(function (): void {
      expect(mockedInvoke).toHaveBeenCalledWith("respond_to_loan_offer", {
        playerId: "loan-owned",
        offerId: "loan-offer-1",
        accept: true,
      });
    });
    expect(onGameUpdate).toHaveBeenCalledWith(updatedState);
  });

  it("submits a counter offer for an incoming loan offer", async function (): Promise<void> {
    const initialState = createGameState([
      createPlayer({
        id: "loan-counter-owned",
        transfer_offers: [],
        loan_offers: [
          {
            id: "loan-offer-counter",
            from_team_id: "team-2",
            parent_team_id: "team-1",
            start_date: "2026-08-01",
            end_date: "2027-01-28",
            wage_contribution_pct: 65,
            buy_option_fee: null,
            status: "Pending",
            date: "2026-08-01",
          },
        ],
      }),
    ]);
    const updatedState = createGameState([
      createPlayer({
        id: "loan-counter-owned",
        team_id: "team-2",
        transfer_offers: [],
      }),
    ]);
    const onGameUpdate = vi.fn();

    mockedInvoke.mockResolvedValueOnce({
      decision: "accepted",
      offer_id: "loan-offer-counter",
      suggested_wage_contribution_pct: null,
      suggested_end_date: null,
      suggested_buy_option_fee: null,
      is_terminal: true,
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
    fireEvent.click(
      screen.getByRole("button", { name: /counter loan offer/i }),
    );
    fireEvent.change(screen.getByLabelText(/wage contribution/i), {
      target: { value: "85" },
    });
    fireEvent.click(screen.getByRole("button", { name: /submit counter/i }));

    await waitFor(function (): void {
      expect(mockedInvoke).toHaveBeenCalledWith("counter_loan_offer", {
        playerId: "loan-counter-owned",
        offerId: "loan-offer-counter",
        endDate: "2027-01-28",
        wageContributionPct: 85,
        buyOptionFee: null,
      });
    });
    expect(onGameUpdate).toHaveBeenCalledWith(updatedState);
  });

  it("exercises an accepted loan buy option from the offers view", async function (): Promise<void> {
    const initialState = createGameState([
      createPlayer({
        id: "loan-buy-player",
        team_id: "team-1",
        transfer_offers: [],
        active_loan: {
          parent_team_id: "team-2",
          loan_team_id: "team-1",
          start_date: "2026-08-01",
          end_date: "2027-01-01",
          wage_contribution_pct: 75,
          buy_option_fee: 1250000,
        },
        loan_offers: [
          {
            id: "loan-offer-1",
            from_team_id: "team-1",
            parent_team_id: "team-2",
            start_date: "2026-08-01",
            end_date: "2027-01-01",
            wage_contribution_pct: 75,
            buy_option_fee: 1250000,
            status: "Accepted",
            date: "2026-08-01",
          },
        ],
      }),
    ]);
    const updatedState = createGameState([
      createPlayer({
        id: "loan-buy-player",
        team_id: "team-1",
        transfer_offers: [],
        active_loan: null,
      }),
    ]);
    const onGameUpdate = vi.fn();

    mockedInvoke.mockResolvedValueOnce(updatedState);

    render(
      <TransfersTab
        gameState={initialState}
        onSelectPlayer={vi.fn()}
        onSelectTeam={vi.fn()}
        onGameUpdate={onGameUpdate}
      />,
    );

    fireEvent.click(screen.getByRole("button", { name: /offers/i }));
    fireEvent.click(screen.getByRole("button", { name: /exercise option/i }));

    await waitFor(function (): void {
      expect(mockedInvoke).toHaveBeenCalledWith("exercise_loan_buy_option", {
        playerId: "loan-buy-player",
      });
    });
    expect(onGameUpdate).toHaveBeenCalledWith(updatedState);
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

    fireEvent.click(screen.getByRole("button", { name: /my transfer list/i }));

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

  it("surfaces listing toggle failures from the my-list context menu", async function (): Promise<void> {
    const gameState = createGameState([
      createPlayer({ transfer_listed: true, loan_listed: false }),
    ]);
    const onGameUpdate = vi.fn();

    mockedInvoke.mockRejectedValueOnce(
      "be.error.transfers.playerAlreadyLoaned",
    );

    render(
      <TransfersTab
        gameState={gameState}
        onSelectPlayer={vi.fn()}
        onSelectTeam={vi.fn()}
        onGameUpdate={onGameUpdate}
      />,
    );

    fireEvent.click(screen.getByRole("button", { name: /my transfer list/i }));

    const playerRow = screen.getByText("John Smith").closest("tr");
    expect(playerRow).not.toBeNull();

    fireEvent.contextMenu(playerRow as HTMLTableRowElement);
    fireEvent.click(screen.getByRole("button", { name: "Add to loan list" }));

    await waitFor(function (): void {
      expect(screen.getByRole("alert")).toHaveTextContent(
        "Player already loaned",
      );
    });
    expect(onGameUpdate).not.toHaveBeenCalled();
  });

  it("shows wage budget in annual units (/yr) matching the player wage display (regression #212)", function (): void {
    // wage_budget = 52000 annual → should render as "50K/yr" style value
    // If shown weekly: floor(52000/52) = 1000 → "1K/wk" — a clear unit mismatch
    // Player.wage = 52000 annual → displayed as "50K/yr" in the player row
    const state = createGameState([createPlayer({ wage: 52000 })]);
    state.teams[0].wage_budget = 52000;

    render(
      <TransfersTab
        gameState={state}
        onSelectPlayer={vi.fn()}
        onSelectTeam={vi.fn()}
        onGameUpdate={vi.fn()}
      />,
    );

    const wkElements = document.querySelectorAll("*");
    const hasWeeklySuffix = Array.from(wkElements).some(
      (el) => el.children.length === 0 && el.textContent?.includes("/wk"),
    );
    expect(hasWeeklySuffix).toBe(false);

    const wageBudgetCard = screen.getByTestId("wage-budget-card");
    expect(wageBudgetCard.textContent).toContain("/yr");
  });

  it("shows a dual-listed player once in the my-list view", function (): void {
    const gameState = createGameState([
      createPlayer({ transfer_listed: true, loan_listed: true }),
    ]);

    render(
      <TransfersTab
        gameState={gameState}
        onSelectPlayer={vi.fn()}
        onSelectTeam={vi.fn()}
        onGameUpdate={vi.fn()}
      />,
    );

    expect(
      screen.getByRole("button", { name: /my transfer list \(1\)/i }),
    ).toBeInTheDocument();
    fireEvent.click(screen.getByRole("button", { name: /my transfer list/i }));
    expect(screen.getAllByText("John Smith")).toHaveLength(1);
  });
});
