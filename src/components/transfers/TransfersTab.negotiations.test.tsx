import { fireEvent, render, screen, waitFor } from "@testing-library/react";
import { beforeEach, describe, expect, it, vi } from "vitest";
import { invoke, isTauri } from "@tauri-apps/api/core";
import type { InvokeArgs } from "@tauri-apps/api/core";

import TransfersTab from "./TransfersTab";

/**
 * The fields these mocks read out of an invoke payload. Tauri's IPC boundary is untyped by
 * nature — every command takes a different shape — so this names what is actually touched
 * rather than reaching for `any` and switching type-checking off for the whole object.
 */
type MockInvokePayload = {
  request?: { playerId?: string };
  fee?: number;
  weeklyWage?: number;
};

vi.mock("@tauri-apps/api/core", () => ({
  convertFileSrc: vi.fn((path: string) => path),
  invoke: vi.fn(),
  isTauri: vi.fn(() => false),
}));

vi.mock("../../utils/backendI18n", () => ({
  resolveBackendError: (error: unknown) => (error instanceof Error ? error.message : String(error)),
}));

vi.mock("react-i18next", () => ({
  useTranslation: () => ({ t: transferTestTranslation, i18n: { language: "en" } }),
}));

const mockedInvoke = vi.mocked(invoke);
const mockedIsTauri = vi.mocked(isTauri);

import {
  createPlayer,
  createGameState,
  transferTestTranslation,
} from "./TransfersTab.testFixtures";

describe("TransfersTab", (): void => {
  beforeEach(function resetMocks(): void {
    mockedInvoke.mockReset();
    mockedIsTauri.mockReturnValue(false);
    mockedInvoke.mockImplementation(async (command: string, payload?: InvokeArgs) => {
      const args = payload as MockInvokePayload | undefined;
      if (command === "generate_player_portrait") {
        const playerId = String(args?.request?.playerId ?? "player");
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
        const fee = Number(args?.fee ?? 0);
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
        const wage = Number(args?.weeklyWage ?? 0);
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
  // Given a voided pending registration, when viewing offers, then its funds failure replaces the cooled-off label.
  it("renders the persisted registration failure reason in the offers list", (): void => {
    const state = createGameState([
      createPlayer({
        transfer_offers: [
          {
            id: "offer-registration-failed",
            from_team_id: "team-2",
            fee: 850000,
            wage_offered: 0,
            last_manager_fee: null,
            negotiation_round: 1,
            suggested_counter_fee: null,
            status: "Withdrawn",
            date: "2026-06-01",
            registration_date: "2026-07-02",
            closed_on: "2026-07-02",
            registration_failure_reason: "InsufficientFunds",
          },
        ],
      }),
    ]);
    render(<TransfersTab gameState={state} onSelectPlayer={vi.fn()} onSelectTeam={vi.fn()} />);
    fireEvent.click(screen.getByRole("button", { name: /offers/i }));
    expect(screen.getByText(/Registration failed: insufficient buyer funds/)).toBeInTheDocument();
    expect(screen.queryByText(/Talks cooled off/i)).not.toBeInTheDocument();
  });

  it("renders withdrawn transfer offers with a localized cooled-off status", (): void => {
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
  it("shows bid impact preview and blocks impossible bids", async (): Promise<void> => {
    const state = createGameState([
      createPlayer({
        id: "player-market-1",
        team_id: "team-2",
        transfer_listed: true,
        transfer_offers: [],
        market_value: 1800000,
      }),
    ]);

    render(<TransfersTab gameState={state} onSelectPlayer={vi.fn()} onSelectTeam={vi.fn()} />);

    fireEvent.click(screen.getByRole("button", { name: /^bid$/i }));
    fireEvent.change(screen.getByLabelText(/bid amount/i), {
      target: { value: "9.0" },
    });

    await waitFor((): void => {
      expect(screen.getByText("Projected impact")).toBeInTheDocument();
      expect(screen.getByText("This bid exceeds your transfer budget")).toBeInTheDocument();
      expect(screen.getByRole("button", { name: /submit bid/i })).toBeDisabled();
    });
  });
  it("keeps the bid modal and deal workspace open after acceptance so the user can review the result", async (): Promise<void> => {
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

    mockedInvoke.mockImplementation(async (command: string, payload?: InvokeArgs) => {
      const args = payload as MockInvokePayload | undefined;
      if (command === "preview_transfer_bid_financial_impact") {
        const fee = Number(args?.fee ?? 0);
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
    expect(screen.getByRole("dialog", { name: /john smith/i })).toBeInTheDocument();

    await waitFor((): void => {
      expect(screen.getByRole("button", { name: /submit bid/i })).toBeEnabled();
    });
    fireEvent.click(screen.getByRole("button", { name: /submit bid/i }));

    await waitFor((): void => {
      expect(mockedInvoke).toHaveBeenCalledWith("make_transfer_bid", {
        playerId: "player-market-1",
        fee: 1000000,
      });
    });

    // Modal must stay open on acceptance so the user can read the
    // confirmation. Any auto-close reintroduces the 2-second flicker
    // and the double-unmount of the deal workspace parent. Wait past
    // the old 2s timer so a regression that reintroduces it fires
    // before this assertion runs.
    await new Promise((resolve) => setTimeout(resolve, 2100));
    expect(screen.getByRole("dialog", { name: /john smith/i })).toBeInTheDocument();

    fireEvent.click(screen.getByRole("button", { name: /^close$/i }));
    await waitFor((): void => {
      expect(screen.queryByRole("dialog", { name: /john smith/i })).not.toBeInTheDocument();
    });
  });
  it("filters free agents in the player market and opens the contract modal", async (): Promise<void> => {
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

    await waitFor((): void => {
      expect(screen.getByText("Projected financial impact")).toBeInTheDocument();
      expect(screen.getByLabelText("Offered Wage")).toBeInTheDocument();
    });
  });
  it("submits a loan offer from the player market", async (): Promise<void> => {
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

    await waitFor((): void => {
      expect(mockedInvoke).toHaveBeenCalledWith("make_loan_offer", {
        playerId: "loan-target",
        endDate: "2027-01-01",
        wageContributionPct: 75,
        buyOptionFee: null,
      });
    });
    expect(onGameUpdate).toHaveBeenCalledWith(updatedState);
  });
  it("explains deferred registration and allows closed-window loan negotiations", async (): Promise<void> => {
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
    expect(screen.getByRole("button", { name: /make transfer bid/i })).toBeEnabled();
    fireEvent.click(screen.getByRole("button", { name: /make loan offer/i }));

    expect(screen.getByRole("status")).toHaveTextContent("Transfer window closed");
    expect(screen.getByRole("status")).toHaveTextContent(
      "If accepted, the loan will be registered on",
    );
    expect(screen.getByRole("button", { name: /submit loan offer/i })).toBeEnabled();
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
  it("allows closed-window transfer bid submission when the next opening date is scheduled", async (): Promise<void> => {
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

    mockedInvoke.mockImplementation(async (command: string, payload?: InvokeArgs) => {
      const args = payload as MockInvokePayload | undefined;
      if (command === "preview_transfer_bid_financial_impact") {
        const fee = Number(args?.fee ?? 0);
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
  it("locks transfer and loan routes when the opening date is stale", (): void => {
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

    expect(screen.getByRole("button", { name: /make transfer bid/i })).toBeDisabled();
    expect(screen.getByRole("button", { name: /make loan offer/i })).toBeDisabled();
    expect(screen.getAllByText("Transfer window closed").length).toBeGreaterThan(0);
    expect(screen.queryByRole("button", { name: /submit loan offer/i })).not.toBeInTheDocument();
  });
});
