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
  resolveBackendError: (error: unknown) => {
    const message = error instanceof Error ? error.message : String(error);
    return message.startsWith("be.error.transfers.loanBorrowerCannotAffordWages")
      ? "The borrowing club cannot afford this wage share. Its weekly wage budget is €40,000."
      : message;
  },
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
  it("submits a loan offer with a buy option", async (): Promise<void> => {
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

    await waitFor((): void => {
      expect(mockedInvoke).toHaveBeenCalledWith("make_loan_offer", {
        playerId: "loan-buy-target",
        endDate: "2027-06-30",
        wageContributionPct: 75,
        buyOptionFee: 1250000,
      });
    });
    expect(onGameUpdate).toHaveBeenCalledWith(updatedState);
  });
  it("accepts an incoming loan offer from the offers view", async (): Promise<void> => {
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
    expect(screen.getByText("Loan 75% wages until 2027-01-01 — Live")).toBeInTheDocument();
    fireEvent.click(screen.getByTitle("Accept Loan"));

    await waitFor((): void => {
      expect(mockedInvoke).toHaveBeenCalledWith("respond_to_loan_offer", {
        playerId: "loan-owned",
        offerId: "loan-offer-1",
        accept: true,
      });
    });
    expect(onGameUpdate).toHaveBeenCalledWith(updatedState);
  });
  // Given the backend refuses an Accept because the borrower can no longer afford the wage share, when the manager clicks Accept, then the translated refusal is shown and the game is untouched.
  it("shows the backend's refusal when accepting an incoming loan offer fails", async (): Promise<void> => {
    const gameState = createGameState([
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
    const onGameUpdate = vi.fn();
    mockedInvoke.mockRejectedValueOnce(
      "be.error.transfers.loanBorrowerCannotAffordWages?budget=40000",
    );

    render(
      <TransfersTab
        gameState={gameState}
        onSelectPlayer={vi.fn()}
        onSelectTeam={vi.fn()}
        onGameUpdate={onGameUpdate}
      />,
    );

    fireEvent.click(screen.getByRole("button", { name: /offers/i }));
    fireEvent.click(screen.getByTitle("Accept Loan"));

    await waitFor((): void => {
      expect(screen.getByRole("alert")).toHaveTextContent(
        "The borrowing club cannot afford this wage share. Its weekly wage budget is €40,000.",
      );
    });
    expect(onGameUpdate).not.toHaveBeenCalled();
  });

  // Given a refused Accept, when the manager then accepts successfully, then the stale refusal is cleared.
  it("clears the refusal once a later response succeeds", async (): Promise<void> => {
    const gameState = createGameState([
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
    mockedInvoke.mockRejectedValueOnce("be.error.transfers.playerAlreadyLoaned");
    mockedInvoke.mockResolvedValueOnce(gameState);

    render(
      <TransfersTab
        gameState={gameState}
        onSelectPlayer={vi.fn()}
        onSelectTeam={vi.fn()}
        onGameUpdate={vi.fn()}
      />,
    );

    fireEvent.click(screen.getByRole("button", { name: /offers/i }));
    fireEvent.click(screen.getByTitle("Accept Loan"));
    await screen.findByRole("alert");
    fireEvent.click(screen.getByTitle("Accept Loan"));

    await waitFor((): void => {
      expect(screen.queryByRole("alert")).not.toBeInTheDocument();
    });
  });

  it("submits a counter offer for an incoming loan offer", async (): Promise<void> => {
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
    fireEvent.click(screen.getByRole("button", { name: /counter loan offer/i }));
    fireEvent.change(screen.getByLabelText(/wage contribution/i), {
      target: { value: "85" },
    });
    fireEvent.click(screen.getByRole("button", { name: /submit counter/i }));

    await waitFor((): void => {
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
  it("exercises an accepted loan buy option from the offers view", async (): Promise<void> => {
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

    await waitFor((): void => {
      expect(mockedInvoke).toHaveBeenCalledWith("exercise_loan_buy_option", {
        playerId: "loan-buy-player",
      });
    });
    expect(onGameUpdate).toHaveBeenCalledWith(updatedState);
  });
  it("offers transfer-list actions from the my-list context menu", async (): Promise<void> => {
    const gameState = createGameState([createPlayer({ transfer_listed: true })]);
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
    fireEvent.click(screen.getByRole("menuitem", { name: "Remove from transfer list" }));

    await waitFor((): void => {
      expect(mockedInvoke).toHaveBeenCalledWith("toggle_transfer_list", {
        playerId: "player-1",
      });
      expect(onGameUpdate).toHaveBeenCalledWith(gameState);
    });
  });
  it("surfaces listing toggle failures from the my-list context menu", async (): Promise<void> => {
    const gameState = createGameState([
      createPlayer({ transfer_listed: true, loan_listed: false }),
    ]);
    const onGameUpdate = vi.fn();

    mockedInvoke.mockRejectedValueOnce("be.error.transfers.playerAlreadyLoaned");

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
    fireEvent.click(screen.getByRole("menuitem", { name: "Add to loan list" }));

    await waitFor((): void => {
      expect(screen.getByRole("alert")).toHaveTextContent("Player already loaned");
    });
    expect(onGameUpdate).not.toHaveBeenCalled();
  });
  it("shows wage budget in weekly units (/wk) matching the player wage display (regression #212)", (): void => {
    // Stored wages are weekly. After the unit lock the card must say /wk,
    // not /yr, or a 52,000 offer is charged every Monday.
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

    const wageBudgetCard = screen.getByTestId("wage-budget-card");
    expect(wageBudgetCard.textContent).toContain("/wk");
    expect(wageBudgetCard.textContent).not.toContain("/yr");
  });
  it("shows a dual-listed player once in the my-list view", (): void => {
    const gameState = createGameState([createPlayer({ transfer_listed: true, loan_listed: true })]);

    render(
      <TransfersTab
        gameState={gameState}
        onSelectPlayer={vi.fn()}
        onSelectTeam={vi.fn()}
        onGameUpdate={vi.fn()}
      />,
    );

    expect(screen.getByRole("button", { name: /my transfer list \(1\)/i })).toBeInTheDocument();
    fireEvent.click(screen.getByRole("button", { name: /my transfer list/i }));
    expect(screen.getAllByText("John Smith")).toHaveLength(1);
  });
});
