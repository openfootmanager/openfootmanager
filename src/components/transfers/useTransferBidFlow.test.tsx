import { act, cleanup, fireEvent, render, screen, waitFor } from "@testing-library/react";
import type { JSX } from "react";
import { beforeEach, describe, expect, it, vi } from "vitest";

import type { GameStateData, PlayerData, TeamData } from "../../store/gameStore";
import {
  type BidNegotiationStatusData,
  getBidNegotiationStatus,
  makeTransferBid,
  previewTransferBidFinancialImpact,
} from "../../services/transfersService";
import { useTransferBidFlow } from "./useTransferBidFlow";

vi.mock("../../services/transfersService", () => ({
  getBidNegotiationStatus: vi.fn(),
  makeTransferBid: vi.fn(),
  previewTransferBidFinancialImpact: vi.fn(),
}));

const mockedMakeTransferBid = vi.mocked(makeTransferBid);
const mockedNegotiationStatus = vi.mocked(getBidNegotiationStatus);

const noNegotiation: BidNegotiationStatusData = {
  offer_id: null,
  continues_talks: false,
  talks_resume_on: null,
};
const mockedPreviewTransferBidFinancialImpact = vi.mocked(previewTransferBidFinancialImpact);

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
    id: "player-market-1",
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
    retired: false,
    contract_end: "2028-06-30",
    wage: 1000,
    market_value: 1500000,
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

function createGameState(players: PlayerData[] = [createPlayer()]): GameStateData {
  return {
    clock: {
      current_date: "2026-08-01T12:00:00Z",
      start_date: "2026-07-01T12:00:00Z",
    },
    manager: {
      display_name: "Jane Doe",
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
        name: "Seller FC",
        short_name: "SEL",
        manager_id: "manager-2",
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

function HookHarness({
  gameState,
  target,
}: {
  gameState: GameStateData;
  target: PlayerData;
}): JSX.Element {
  const {
    bidAmount,
    bidResult,
    bidSubmitDisabled,
    setBidAmount,
    openBidNegotiation,
    handleMakeBid,
    activeBidOffer,
    hasExistingOffer,
    talksResumeOn,
    bidFeedback,
  } = useTransferBidFlow({ gameState });

  return (
    <div>
      <button type="button" onClick={() => openBidNegotiation(target)}>
        Open
      </button>
      <label htmlFor="bid-amount">Bid amount</label>
      <input
        id="bid-amount"
        value={bidAmount}
        onChange={(event) => setBidAmount(event.target.value)}
      />
      <button type="button" onClick={() => void handleMakeBid()}>
        Submit
      </button>
      <output aria-label="live-round">{activeBidOffer?.negotiation_round ?? "none"}</output>
      <output aria-label="feedback">{bidFeedback?.headline_key ?? "none"}</output>
      <output aria-label="has-existing">{String(hasExistingOffer)}</output>
      <output aria-label="resume-on">{talksResumeOn ?? "none"}</output>
      <output aria-label="submit-disabled">{String(bidSubmitDisabled)}</output>
      <output aria-label="bid-result">{bidResult ?? "idle"}</output>
    </div>
  );
}

function previewAnswer(overrides: { blocked_by_wage_policy?: boolean } = {}) {
  return {
    projection: {
      transfer_budget_before: 2000000,
      transfer_budget_after: 500000,
      finance_before: 5000000,
      finance_after: 3500000,
      annual_wage_bill_before: 1000,
      annual_wage_bill_after: 2000,
      annual_wage_budget: 50000,
      current_weekly_wage_spend: 20,
      projected_weekly_wage_spend: 40,
      weekly_wage_budget: 960,
      incoming_player_weekly_wage: 20,
      projected_wage_budget_usage_pct: 4,
      exceeds_transfer_budget: false,
      exceeds_finance: false,
      blocked_by_wage_policy: false,
      ...overrides,
    },
  };
}

describe("useTransferBidFlow", (): void => {
  beforeEach(function resetMocks(): void {
    mockedMakeTransferBid.mockReset();
    mockedPreviewTransferBidFinancialImpact.mockReset();
    mockedPreviewTransferBidFinancialImpact.mockResolvedValue(previewAnswer());
    mockedNegotiationStatus.mockReset();
    mockedNegotiationStatus.mockResolvedValue(noNegotiation);
  });

  /**
   * Given the backend flags the bid's wage as one the board would refuse
   * When the bid is previewed
   * Then submitting is disabled, as it is for a fee over the budget, and a bid the board accepts
   *      is not.
   */
  it("disables submitting a bid whose wage the board would refuse", async (): Promise<void> => {
    const open = async (blocked: boolean) => {
      const target = createPlayer();
      mockedPreviewTransferBidFinancialImpact.mockResolvedValue(
        previewAnswer({ blocked_by_wage_policy: blocked }),
      );
      render(<HookHarness gameState={createGameState([target])} target={target} />);
      fireEvent.click(screen.getByRole("button", { name: "Open" }));
      await waitFor((): void => {
        expect(mockedPreviewTransferBidFinancialImpact).toHaveBeenCalled();
      });
      // Let the preview's answer land in the hook before reading what it decided.
      await act(async () => {});
    };

    await open(false);
    expect(screen.getByLabelText("submit-disabled")).toHaveTextContent("false");
    cleanup();
    mockedPreviewTransferBidFinancialImpact.mockClear();

    await open(true);
    expect(screen.getByLabelText("submit-disabled")).toHaveTextContent("true");
  });

  /**
   * Given the bid panel is open on a player and a bid then advances the negotiation
   * When the game state the screen shows is a newer one with a later round on that player
   * Then the panel reads the live player: it shows the later round, not the one it opened with.
   */
  it("reads the live player, not the one it was opened with", async (): Promise<void> => {
    const offer = (round: number) => ({
      id: "offer-1",
      from_team_id: "team-1",
      fee: 1200000,
      wage_offered: 0,
      last_manager_fee: null,
      negotiation_round: round,
      suggested_counter_fee: null,
      status: "Pending" as const,
      date: "2026-08-01",
    });
    mockedNegotiationStatus.mockResolvedValue({
      offer_id: "offer-1",
      continues_talks: true,
      talks_resume_on: null,
    });
    const opened = createPlayer({ transfer_offers: [offer(1)] });
    const { rerender } = render(
      <HookHarness gameState={createGameState([opened])} target={opened} />,
    );
    fireEvent.click(screen.getByRole("button", { name: "Open" }));
    await waitFor(() => expect(screen.getByLabelText("live-round")).toHaveTextContent("1"));

    const later = createPlayer({ transfer_offers: [offer(3)] });
    rerender(<HookHarness gameState={createGameState([later])} target={opened} />);

    await waitFor(() => expect(screen.getByLabelText("live-round")).toHaveTextContent("3"));
  });

  /**
   * Given the player has an old rejected offer from the user's club that the backend does not
   *       continue (it names no negotiation)
   * When the panel opens
   * Then it shows no negotiation: the next bid starts over, as the engine will have it.
   */
  it("shows no negotiation when the backend names none, whatever offers the player holds", async (): Promise<void> => {
    const oldRefusal = createPlayer({
      transfer_offers: [
        {
          id: "old",
          from_team_id: "team-1",
          fee: 1000000,
          wage_offered: 0,
          last_manager_fee: null,
          negotiation_round: 4,
          suggested_counter_fee: null,
          status: "Rejected",
          date: "2026-05-01",
        },
      ],
    });
    render(<HookHarness gameState={createGameState([oldRefusal])} target={oldRefusal} />);

    fireEvent.click(screen.getByRole("button", { name: "Open" }));
    await waitFor(() => expect(mockedNegotiationStatus).toHaveBeenCalled());
    await act(async () => {});

    expect(screen.getByLabelText("live-round")).toHaveTextContent("none");
  });

  /**
   * Given the backend names a live offer, and separately a refused one it still continues
   * When the panel opens
   * Then "resume the negotiation" is offered only for the live offer.
   */
  it("offers to resume only a live negotiation", async (): Promise<void> => {
    const offerWith = (status: "Pending" | "Rejected") => ({
      id: "offer-1",
      from_team_id: "team-1",
      fee: 1000000,
      wage_offered: 0,
      last_manager_fee: null,
      negotiation_round: 2,
      suggested_counter_fee: null,
      status,
      date: "2026-08-01",
    });
    const check = async (status: "Pending" | "Rejected", expected: string) => {
      mockedNegotiationStatus.mockResolvedValue({
        offer_id: "offer-1",
        continues_talks: true,
        talks_resume_on: null,
      });
      const target = createPlayer({ transfer_offers: [offerWith(status)] });
      render(<HookHarness gameState={createGameState([target])} target={target} />);
      fireEvent.click(screen.getByRole("button", { name: "Open" }));
      await waitFor(() => expect(screen.getByLabelText("live-round")).toHaveTextContent("2"));
      expect(screen.getByLabelText("has-existing")).toHaveTextContent(expected);
      cleanup();
    };

    await check("Pending", "true");
    await check("Rejected", "false");
  });

  /**
   * Given the backend says talks are broken off until a day
   * When the panel opens
   * Then the day is exposed and submitting is disabled.
   */
  it("exposes the day broken-off talks resume and blocks submitting", async (): Promise<void> => {
    mockedNegotiationStatus.mockResolvedValue({
      offer_id: null,
      continues_talks: false,
      talks_resume_on: "2026-08-28",
    });
    const target = createPlayer();
    render(<HookHarness gameState={createGameState([target])} target={target} />);

    fireEvent.click(screen.getByRole("button", { name: "Open" }));

    await waitFor(() => expect(screen.getByLabelText("resume-on")).toHaveTextContent("2026-08-28"));
    expect(screen.getByLabelText("submit-disabled")).toHaveTextContent("true");
  });

  /**
   * Given broken-off talks, where the backend still names the broken round-4 offer and a day
   *       talks can resume
   * When the panel opens
   * Then it pre-fills nothing and shows no resumed feedback: "a new bid carries the talks on"
   *      would sit beside "talks can resume on …".
   */
  it("pre-fills nothing for broken-off talks", async (): Promise<void> => {
    mockedNegotiationStatus.mockResolvedValue({
      offer_id: "broken",
      continues_talks: false,
      talks_resume_on: "2026-08-28",
    });
    const target = createPlayer({
      market_value: 2000000,
      transfer_offers: [
        {
          id: "broken",
          from_team_id: "team-1",
          fee: 900000,
          wage_offered: 0,
          last_manager_fee: 900000,
          negotiation_round: 4,
          suggested_counter_fee: 1150000,
          status: "Rejected",
          date: "2026-07-20",
          closed_on: "2026-07-29",
        },
      ],
    });
    render(<HookHarness gameState={createGameState([target])} target={target} />);

    fireEvent.click(screen.getByRole("button", { name: "Open" }));
    await waitFor(() => expect(screen.getByLabelText("resume-on")).toHaveTextContent("2026-08-28"));
    await act(async () => {});

    expect(screen.getByLabelText("feedback")).toHaveTextContent("none");
    expect(screen.getByLabelText("live-round")).toHaveTextContent("4");
    expect((screen.getByLabelText("Bid amount") as HTMLInputElement).value).toBe("2.0");
  });

  /**
   * Given the panel is open on a player and the user opens the same player again
   * When the panel opens a second time
   * Then the backend is asked again: opening clears what the panel knew, so it must refill it.
   */
  it("asks the backend again every time the panel is opened", async (): Promise<void> => {
    const target = createPlayer();
    render(<HookHarness gameState={createGameState([target])} target={target} />);

    fireEvent.click(screen.getByRole("button", { name: "Open" }));
    await waitFor(() => expect(mockedNegotiationStatus).toHaveBeenCalledTimes(1));
    fireEvent.click(screen.getByRole("button", { name: "Open" }));

    await waitFor(() => expect(mockedNegotiationStatus).toHaveBeenCalledTimes(2));
  });

  it("does not submit a bid when the computed fee is invalid", async (): Promise<void> => {
    const target = createPlayer();
    const gameState = createGameState([target]);

    render(<HookHarness gameState={gameState} target={target} />);

    fireEvent.click(screen.getByRole("button", { name: "Open" }));

    await waitFor((): void => {
      expect(mockedPreviewTransferBidFinancialImpact).toHaveBeenCalledWith(target.id, 1500000);
    });

    fireEvent.change(screen.getByLabelText("Bid amount"), {
      target: { value: "abc" },
    });
    fireEvent.click(screen.getByRole("button", { name: "Submit" }));

    await waitFor((): void => {
      expect(mockedMakeTransferBid).not.toHaveBeenCalled();
    });
  });

  /**
   * Given the panel opened on a player with no negotiation, and a bid that the club counters
   * When the game update that carries the new offer arrives and the backend names it
   * Then the bid's own feedback stays: the generic "resumed negotiation" feedback is for an offer
   *      found on opening, not one the user has just made.
   */
  it("keeps a bid's own feedback when the game update brings its offer", async (): Promise<void> => {
    const target = createPlayer();
    const before = createGameState([target]);
    mockedMakeTransferBid.mockResolvedValue({
      decision: "counter_offer",
      suggested_fee: 1150000,
      is_terminal: false,
      feedback: {
        mood: "firm",
        headline_key: "transfers.transferFeedbackCounterHeadline",
        detail_key: "transfers.transferFeedbackCounterDetail",
        tension: 60,
        patience: 50,
        round: 2,
        params: { fee: "1150000" },
      },
      game: before,
    });
    const { rerender } = render(<HookHarness gameState={before} target={target} />);
    fireEvent.click(screen.getByRole("button", { name: "Open" }));
    await waitFor(() => expect(mockedNegotiationStatus).toHaveBeenCalled());
    fireEvent.click(screen.getByRole("button", { name: "Submit" }));
    await waitFor(() =>
      expect(screen.getByLabelText("feedback")).toHaveTextContent(
        "transfers.transferFeedbackCounterHeadline",
      ),
    );

    // The game update arrives carrying the offer the bid just made, and the backend names it.
    mockedNegotiationStatus.mockResolvedValue({
      offer_id: "offer-new",
      continues_talks: true,
      talks_resume_on: null,
    });
    const updated = createPlayer({
      transfer_offers: [
        {
          id: "offer-new",
          from_team_id: "team-1",
          fee: 1000000,
          wage_offered: 0,
          last_manager_fee: 1000000,
          negotiation_round: 2,
          suggested_counter_fee: 1150000,
          status: "Pending",
          date: "2026-08-01",
        },
      ],
    });
    rerender(<HookHarness gameState={createGameState([updated])} target={target} />);
    await waitFor(() => expect(screen.getByLabelText("live-round")).toHaveTextContent("2"));
    await act(async () => {});

    expect(screen.getByLabelText("feedback")).toHaveTextContent(
      "transfers.transferFeedbackCounterHeadline",
    );
  });

  it("keeps the acceptance state visible so the modal does not auto-close", async (): Promise<void> => {
    const target = createPlayer();
    const gameState = createGameState([target]);
    mockedMakeTransferBid.mockResolvedValue({
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
        params: { fee: "1500000" },
      },
      game: gameState,
    });

    render(<HookHarness gameState={gameState} target={target} />);

    fireEvent.click(screen.getByRole("button", { name: "Open" }));
    fireEvent.click(screen.getByRole("button", { name: "Submit" }));

    await waitFor((): void => {
      expect(screen.getByLabelText("bid-result")).toHaveTextContent("accepted");
    });

    // No auto-close: the accepted state must persist until the caller
    // explicitly closes the modal. Regressions here reintroduce the
    // 2-second flicker that hid the acceptance confirmation. Wait
    // past the old 2s timer so the guard actually catches its return.
    await new Promise((resolve) => setTimeout(resolve, 2100));
    expect(screen.getByLabelText("bid-result")).toHaveTextContent("accepted");
  });

  it("pre-fills a counter-offer suggestion in millions, not raw euros", async (): Promise<void> => {
    // Regression for #300: `suggested_fee` is raw euros, but this input is
    // millions-denominated. If it isn't converted, "650000" in a millions
    // input becomes a €650 billion resubmit (1,000,000× too large).
    const target = createPlayer();
    const gameState = createGameState([target]);
    mockedMakeTransferBid.mockResolvedValue({
      decision: "counter_offer",
      suggested_fee: 650000,
      is_terminal: false,
      feedback: {
        mood: "firm",
        headline_key: "transfers.transferFeedbackCounterHeadline",
        detail_key: "transfers.transferFeedbackCounterDetail",
        tension: 40,
        patience: 60,
        round: 1,
        params: { fee: "650000" },
      },
      game: gameState,
    });

    render(<HookHarness gameState={gameState} target={target} />);

    fireEvent.click(screen.getByRole("button", { name: "Open" }));
    fireEvent.click(screen.getByRole("button", { name: "Submit" }));

    await waitFor((): void => {
      expect(screen.getByLabelText("bid-result")).toHaveTextContent("counter_offer");
    });

    expect(screen.getByLabelText("Bid amount")).toHaveValue("0.65");
  });
});
