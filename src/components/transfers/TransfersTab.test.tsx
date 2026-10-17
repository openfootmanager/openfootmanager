import { fireEvent, render, screen, waitFor, within } from "@testing-library/react";
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
  createScout,
  createGameState,
  marketPageOf,
  routedResponses,
  transferTestTranslation,
} from "./TransfersTab.testFixtures";
import type { PlayersPage, PlayersPageQuery } from "../../services/playersService";
import type { PlayerData } from "../../store/gameStore";

describe("TransfersTab", (): void => {
  // What the backend answers `get_players_page` with; each test that shows market rows says which.
  let marketAnswer: (query: PlayersPageQuery) => PlayersPage;
  let routes: ReturnType<typeof routedResponses>;
  // The backend answers each list of the centre separately; a test says who is on the one it shows.
  const answerList =
    (scope: PlayersPageQuery["scope"], players: PlayerData[]) =>
    (query: PlayersPageQuery): PlayersPage =>
      marketPageOf(query.scope === scope ? players : []);
  beforeEach(function resetMocks(): void {
    mockedInvoke.mockReset();
    mockedIsTauri.mockReturnValue(false);
    marketAnswer = () => marketPageOf([]);
    mockedInvoke.mockImplementation(async (command: string, payload?: InvokeArgs) => {
      const args = payload as MockInvokePayload | undefined;
      if (command === "get_players_page") {
        return marketAnswer((payload as { query: PlayersPageQuery }).query);
      }
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
    routes = routedResponses(mockedInvoke);
  });
  it("renders a dual transfer and loan listed player once with both status badges", async (): Promise<void> => {
    const dualListed = createPlayer({
      id: "dual-listed",
      full_name: "Dual Listed",
      transfer_listed: true,
      loan_listed: true,
    });
    marketAnswer = answerList("user_listed", [dualListed]);
    render(
      <TransfersTab
        gameState={createGameState([dualListed])}
        onSelectPlayer={vi.fn()}
        onSelectTeam={vi.fn()}
        onGameUpdate={vi.fn()}
      />,
    );

    fireEvent.click(screen.getByRole("button", { name: /my transfer list/i }));

    // The filter chips carry the same words as the badges, so read the table.
    const table = within(await screen.findByRole("table"));
    expect(table.getAllByText("Dual Listed")).toHaveLength(1);
    expect(table.getByText("TRANSFER")).toBeInTheDocument();
    expect(table.getByText("LOAN")).toBeInTheDocument();
    expect(screen.getByText(/My Transfer List \(1\)/)).toBeInTheDocument();
  });
  it("paginates the transfer players list instead of mounting every market row", async (): Promise<void> => {
    const rows = Array.from({ length: 65 }, (_, index) =>
      createPlayer({
        id: `market-player-${index + 1}`,
        full_name: `Market Player ${index + 1}`,
        match_name: `M. Player ${index + 1}`,
        team_id: "team-2",
        transfer_listed: true,
        transfer_offers: [],
      }),
    );

    marketAnswer = (query) =>
      marketPageOf(rows.slice((query.page - 1) * 30, query.page * 30), {
        total: 65,
        page: query.page,
      });

    render(
      <TransfersTab
        gameState={createGameState(rows)}
        onSelectPlayer={vi.fn()}
        onSelectTeam={vi.fn()}
        onGameUpdate={vi.fn()}
      />,
    );

    expect(await screen.findAllByText(/^Market Player \d+$/)).toHaveLength(30);
    expect(screen.getByText("Market Player 30")).toBeInTheDocument();
    expect(screen.queryByText("Market Player 31")).not.toBeInTheDocument();
    expect(screen.getByText("Showing 1-30 of 65")).toBeInTheDocument();

    fireEvent.click(screen.getByRole("button", { name: "Next page" }));

    expect(await screen.findByText("Market Player 31")).toBeInTheDocument();
    expect(screen.queryByText("Market Player 1")).not.toBeInTheDocument();
    expect(screen.getByText("Showing 31-60 of 65")).toBeInTheDocument();
    expect(mockedInvoke).toHaveBeenCalledWith(
      "get_players_page",
      expect.objectContaining({ query: expect.objectContaining({ page: 2, scope: "market" }) }),
    );
  });
  /**
   * Given the transfer market is open
   * When the free-agent chip and then "affordable only" are pressed
   * Then the backend is asked for the market scope, narrowed to free agents and to what the club
   *      can pay for, back on the first page: the screen holds no filtering rule of its own.
   */
  it("asks the backend for the market scope with the chosen status and budget", async (): Promise<void> => {
    render(
      <TransfersTab
        gameState={createGameState([createPlayer({ team_id: "team-2", transfer_listed: true })])}
        onSelectPlayer={vi.fn()}
        onSelectTeam={vi.fn()}
        onGameUpdate={vi.fn()}
      />,
    );
    await waitFor((): void => {
      expect(mockedInvoke).toHaveBeenCalledWith(
        "get_players_page",
        expect.objectContaining({
          query: expect.objectContaining({ scope: "market", status: "available" }),
        }),
      );
    });

    fireEvent.click(screen.getByRole("button", { name: /^Free Agent/ }));
    fireEvent.click(screen.getByRole("button", { name: "transfers.affordableOnly" }));

    await waitFor((): void => {
      expect(mockedInvoke).toHaveBeenLastCalledWith(
        "get_players_page",
        expect.objectContaining({
          query: expect.objectContaining({
            scope: "market",
            status: "free_agent",
            in_budget: true,
            page: 1,
          }),
        }),
      );
    });
  });
  it("starts runtime portrait loading for the visible transfer market page", async (): Promise<void> => {
    mockedIsTauri.mockReturnValue(true);
    const rows = Array.from({ length: 35 }, (_, index) =>
      createPlayer({
        id: `portrait-player-${index + 1}`,
        full_name: `Portrait Player ${index + 1}`,
        match_name: `P. Player ${index + 1}`,
        team_id: "team-2",
        transfer_listed: true,
        transfer_offers: [],
      }),
    );

    marketAnswer = () => marketPageOf(rows.slice(0, 30), { total: 35 });

    render(
      <TransfersTab
        gameState={createGameState(rows)}
        onSelectPlayer={vi.fn()}
        onSelectTeam={vi.fn()}
        onGameUpdate={vi.fn()}
      />,
    );

    await waitFor((): void => {
      const portraitCalls = mockedInvoke.mock.calls.filter(
        ([command]) => command === "generate_player_portrait",
      );
      expect(portraitCalls).toHaveLength(30);
      expect(portraitCalls[0]?.[1]).toEqual({
        request: expect.objectContaining({ playerId: "portrait-player-1" }),
      });
      expect(
        portraitCalls.some(([, payload]) => {
          const portraitPayload = payload as { request?: { playerId?: string } } | undefined;
          return portraitPayload?.request?.playerId === "portrait-player-31";
        }),
      ).toBe(false);
    });
  });
  it("submits a counter offer for a pending incoming bid and publishes the updated game", async (): Promise<void> => {
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

    marketAnswer = answerList("with_offers", initialState.players);
    routes.resolveOnce("counter_offer", {
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
    fireEvent.click(await screen.findByRole("button", { name: /counter offer/i }));
    fireEvent.change(screen.getByLabelText(/counter amount/i), {
      target: { value: "1200000" },
    });
    fireEvent.click(screen.getByRole("button", { name: /submit counter/i }));

    await waitFor((): void => {
      expect(mockedInvoke).toHaveBeenCalledWith("counter_offer", {
        careerToken: 0,
        playerId: "player-1",
        offerId: "offer-1",
        requestedFee: 1200000,
      });
    });

    expect(onGameUpdate).toHaveBeenCalledWith(updatedState);
    expect(screen.getByText("Negotiation pulse")).toBeInTheDocument();
    expect(screen.getByText("They want more before shaking hands.")).toBeInTheDocument();
    expect(
      screen.getByText(
        "The bid was close enough to keep talking, but their side are signalling a price nearer €1,150,000.",
      ),
    ).toBeInTheDocument();
  });
  it("resumes an existing outgoing transfer negotiation when reopening the bid modal", async (): Promise<void> => {
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

    marketAnswer = () => marketPageOf(state.players);
    // The backend names the negotiation the next bid continues.
    routes.resolveOnce("get_bid_negotiation_status", {
      offer_id: "offer-user-1",
      continues_talks: true,
      talks_resume_on: null,
    });

    render(
      <TransfersTab
        gameState={state}
        onSelectPlayer={vi.fn()}
        onSelectTeam={vi.fn()}
        onGameUpdate={vi.fn()}
      />,
    );

    fireEvent.click(await screen.findByRole("button", { name: /^bid$/i }));

    await waitFor(() => {
      expect(mockedInvoke).toHaveBeenCalledWith("preview_transfer_bid_financial_impact", {
        fee: 1150000,
        playerId: "player-market-1",
      });
    });

    expect(await screen.findByText("Talks are still live with this club.")).toBeInTheDocument();
    expect(screen.getByText("The other club are waiting for your next move.")).toBeInTheDocument();
    expect(screen.getByText("Their last signal pointed toward €1,150,000.")).toBeInTheDocument();
    expect(screen.getByText("Recent exchange")).toBeInTheDocument();
    expect(screen.getByText("Your last bid")).toBeInTheDocument();
    expect(screen.getByText("Their last signal")).toBeInTheDocument();
    expect(screen.getAllByText("Round 2")).toHaveLength(2);
    expect(screen.getByDisplayValue("1.15")).toBeInTheDocument();
  });
  it("shows scout assignment errors inline on the player market", async (): Promise<void> => {
    const consoleErrorSpy = vi.spyOn(console, "error").mockImplementation(() => {});
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

      routes.rejectOnce(
        "send_scout",
        new Error("Scout is already assigned to another scouting task."),
      );
      marketAnswer = () => marketPageOf(state.players);

      render(
        <TransfersTab
          gameState={state}
          onSelectPlayer={vi.fn()}
          onSelectTeam={vi.fn()}
          onGameUpdate={vi.fn()}
        />,
      );

      const playerRow = (await screen.findByText("John Smith")).closest("tr");
      expect(playerRow).not.toBeNull();

      fireEvent.contextMenu(playerRow as HTMLTableRowElement);
      fireEvent.click(screen.getByRole("menuitem", { name: "Scout" }));

      await waitFor(() => {
        expect(screen.getByRole("alert")).toHaveTextContent(
          "Scout is already assigned to another scouting task.",
        );
      });
    } finally {
      consoleErrorSpy.mockRestore();
    }
  });
  it("resumes an incoming transfer negotiation when reopening the counter-offer modal", async (): Promise<void> => {
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
    marketAnswer = answerList("with_offers", state.players);

    render(
      <TransfersTab
        gameState={state}
        onSelectPlayer={vi.fn()}
        onSelectTeam={vi.fn()}
        onGameUpdate={vi.fn()}
      />,
    );

    fireEvent.click(screen.getByRole("button", { name: /offers/i }));
    fireEvent.click(await screen.findByRole("button", { name: /counter offer/i }));

    expect(screen.getByText("Talks are still live with this club.")).toBeInTheDocument();
    expect(screen.getByText("The other club are waiting for your next move.")).toBeInTheDocument();
    expect(screen.getByText("Their last signal pointed toward €1,150,000.")).toBeInTheDocument();
    expect(screen.getByText("Recent exchange")).toBeInTheDocument();
    expect(screen.getByText("Your last counter")).toBeInTheDocument();
    expect(screen.getByText("Their current offer")).toBeInTheDocument();
    expect(screen.getByText("Round 2")).toBeInTheDocument();
    expect(screen.getByDisplayValue("1150000")).toBeInTheDocument();
  });
  it("shows a localized message when a counter-offer expires before submission", async (): Promise<void> => {
    const state = createGameState();
    marketAnswer = answerList("with_offers", state.players);
    routes.rejectOnce("counter_offer", "Offer not found or not pending");

    render(
      <TransfersTab
        gameState={state}
        onSelectPlayer={vi.fn()}
        onSelectTeam={vi.fn()}
        onGameUpdate={vi.fn()}
      />,
    );

    fireEvent.click(screen.getByRole("button", { name: /offers/i }));
    fireEvent.click(await screen.findByRole("button", { name: /counter offer/i }));
    fireEvent.click(screen.getByRole("button", { name: /submit counter/i }));

    await waitFor((): void => {
      expect(
        screen.getByText(
          "Talks cooled off before you could answer. Start a new negotiation if the club comes back.",
        ),
      ).toBeInTheDocument();
    });
  });
});
