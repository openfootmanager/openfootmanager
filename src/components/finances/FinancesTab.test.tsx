import {
  wireFacilityUpgrades,
  wireSnapshot,
  renderFinances,
  createPlayer,
  createSponsorOfferMessage,
  createGameState,
} from "./FinancesTab.testFixtures";
import { fireEvent, screen, waitFor } from "@testing-library/react";
import { Profiler } from "react";
import { beforeEach, describe, expect, it, vi } from "vitest";
import { invoke } from "@tauri-apps/api/core";

import { resolveBackendError } from "../../utils/backendI18n";
import FinancesTab from "./FinancesTab";

// What the backend's contract warning stages read as on a badge, by player id; a test says which
// players are at risk. The hook that asks for them has its own test.
const riskLevels = vi.hoisted(() => ({
  current: {} as Record<string, "critical" | "warning" | "stable">,
}));

vi.mock("../../hooks/useContractWarningStages", () => ({
  useContractRiskLevels: () => (playerId: string) => riskLevels.current[playerId] ?? "stable",
}));

vi.mock("@tauri-apps/api/core", () => ({
  invoke: vi.fn(),
}));

// The weeks the tab hands the cash-flow chart, captured: the chart has its own test, and jsdom
// draws no recharts to read them from.
const chartWeeks = vi.hoisted(() => ({ current: undefined as unknown, labels: [] as string[] }));

vi.mock("./FinanceCashFlowChart", () => ({
  FinanceCashFlowChart: (props: { weeks: unknown; incomeLabel: string; expensesLabel: string }) => {
    chartWeeks.current = props.weeks;
    chartWeeks.labels = [props.incomeLabel, props.expensesLabel];
    return <div data-testid="cash-flow-chart" />;
  },
}));

vi.mock("react-i18next", async () => {
  const { translateFinanceTestKey } = await import("./FinancesTab.testFixtures");
  const { useRef } = await import("react");

  return {
    initReactI18next: {
      type: "3rdParty",
      init: () => {},
    },
    useTranslation: () => {
      // The real useTranslation consumes context and state, so it always costs
      // at least one hook slot. The mock has to do the same. A hook-free mock
      // lets a component that returns before its useState calls render *zero*
      // hooks on the guard path, and React then re-takes the mount path on the
      // next render instead of reporting the hook-count change — which silently
      // hides Rules-of-Hooks bugs from every test in this file.
      useRef(null);

      return {
        t: translateFinanceTestKey,
        i18n: { language: "en" },
      };
    },
  };
});

const mockedInvoke = vi.mocked(invoke);

describe("FinancesTab facilities", () => {
  beforeEach(() => {
    riskLevels.current = { "player-critical": "critical", "player-warning": "warning" };
    mockedInvoke.mockReset();
    mockedInvoke.mockImplementation((command) => {
      if (command === "get_finance_snapshot") {
        return Promise.resolve(wireSnapshot());
      }

      return Promise.reject(new Error(`Unexpected command: ${command}`));
    });
  });

  it("renders facility cards with levels and disables upgrades when funds are insufficient", async () => {
    const gameState = createGameState({ finance: 200000 });
    mockedInvoke.mockResolvedValue(
      wireSnapshot({}, { facility_upgrades: wireFacilityUpgrades({ can_afford: false }) }),
    );

    await renderFinances(<FinancesTab gameState={gameState} />);

    expect(screen.getByText("Facilities")).toBeInTheDocument();
    expect(screen.getByText("Training Facility")).toBeInTheDocument();
    expect(screen.getByText("Medical Facility")).toBeInTheDocument();
    expect(screen.getByText("Scouting Facility")).toBeInTheDocument();
    expect(screen.getByText("Level 2")).toBeInTheDocument();
    expect(screen.getByText("Level 1")).toBeInTheDocument();
    expect(screen.getByText("Level 3")).toBeInTheDocument();

    const upgradeButtons = screen.getAllByRole("button", { name: "Upgrade" });
    expect(upgradeButtons).toHaveLength(3);
    expect(upgradeButtons[0]).toBeDisabled();
    expect(upgradeButtons[1]).toBeDisabled();
    expect(upgradeButtons[2]).toBeDisabled();
    expect(screen.getAllByText("Insufficient funds")).toHaveLength(3);
  });

  it("invokes facility upgrade and publishes the updated game state", async () => {
    const initialState = createGameState();
    const updatedState = createGameState({
      finance: 650000,
      facilities: {
        training: 2,
        medical: 2,
        scouting: 3,
      },
      season_expenses: 750000,
    });
    const onGameUpdate = vi.fn();
    mockedInvoke.mockImplementation((command) => {
      if (command === "get_finance_snapshot") {
        return Promise.resolve(wireSnapshot({}, { facility_upgrades: wireFacilityUpgrades() }));
      }

      if (command === "upgrade_facility") {
        return Promise.resolve(updatedState);
      }

      return Promise.reject(new Error(`Unexpected command: ${command}`));
    });

    await renderFinances(<FinancesTab gameState={initialState} onGameUpdate={onGameUpdate} />);

    const upgradeButtons = screen.getAllByRole("button", { name: "Upgrade" });
    fireEvent.click(upgradeButtons[1]);

    await waitFor(() => {
      expect(mockedInvoke).toHaveBeenCalledWith("upgrade_facility", {
        careerToken: 0,
        facility: "Medical",
      });
    });
    expect(onGameUpdate).toHaveBeenCalledWith(updatedState);
  });

  it("renders active sponsorship and pending sponsor offers", async () => {
    const gameState = createGameState(
      {
        sponsorship: {
          sponsor_name: "Acme Corp",
          base_value: 125000,
          remaining_weeks: 8,
          bonus_criteria: [],
        },
      },
      [createSponsorOfferMessage()],
    );

    await renderFinances(<FinancesTab gameState={gameState} />);

    expect(screen.getByText("Sponsors")).toBeInTheDocument();
    expect(screen.getByText("Active Sponsor")).toBeInTheDocument();
    expect(screen.getByText("Acme Corp")).toBeInTheDocument();
    expect(screen.getByText("Weekly value: €125,000")).toBeInTheDocument();
    expect(screen.getByText("8 weeks remaining")).toBeInTheDocument();
    expect(screen.getByText("Pending Offers")).toBeInTheDocument();
    expect(screen.getByText("Sponsorship Offer — GreenTech Industries")).toBeInTheDocument();
    expect(screen.getByText("Receive €100,000 in sponsorship income.")).toBeInTheDocument();
    expect(screen.getByRole("button", { name: "Accept the deal" })).toBeInTheDocument();
    expect(screen.getByRole("button", { name: "Decline politely" })).toBeInTheDocument();
  });

  it("accepts a sponsor offer through resolve_message_action and publishes the updated state", async () => {
    const initialState = createGameState({}, [createSponsorOfferMessage()]);
    const updatedState = createGameState(
      {
        sponsorship: {
          sponsor_name: "GreenTech Industries",
          base_value: 100000,
          remaining_weeks: 12,
          bonus_criteria: [],
        },
      },
      [],
    );
    const onGameUpdate = vi.fn();

    mockedInvoke.mockImplementation((command) => {
      if (command === "get_finance_snapshot") {
        return Promise.resolve(wireSnapshot());
      }

      if (command === "resolve_message_action") {
        return Promise.resolve({
          game: updatedState,
          effect: "Offer accepted",
        });
      }

      return Promise.reject(new Error(`Unexpected command: ${command}`));
    });

    await renderFinances(<FinancesTab gameState={initialState} onGameUpdate={onGameUpdate} />);

    fireEvent.click(screen.getByRole("button", { name: "Accept the deal" }));

    await waitFor(() => {
      expect(mockedInvoke).toHaveBeenCalledWith("resolve_message_action", {
        careerToken: 0,
        messageId: "sponsor_2025-06-15",
        actionId: "respond",
        optionId: "accept",
      });
    });

    expect(onGameUpdate).toHaveBeenCalledWith(updatedState);
  });

  // Given a sponsor offer that was already answered (a stale screen: the backend refuses a second
  // answer with a translated key)
  // When "Accept the deal" is pressed
  // Then the message is shown, translated, where the offer is, instead of the press doing nothing;
  //      and it goes once a later answer is accepted.
  it("shows the translated reason when the backend refuses a sponsor offer's answer", async () => {
    const initialState = createGameState({}, [createSponsorOfferMessage()]);
    const onGameUpdate = vi.fn();
    mockedInvoke.mockImplementation((command) => {
      if (command === "get_finance_snapshot") return Promise.resolve(wireSnapshot());
      if (command === "resolve_message_action") {
        return Promise.reject("be.error.actionAlreadyResolved");
      }
      return Promise.reject(new Error(`Unexpected command: ${command}`));
    });
    await renderFinances(<FinancesTab gameState={initialState} onGameUpdate={onGameUpdate} />);

    fireEvent.click(screen.getByRole("button", { name: "Accept the deal" }));

    const reason = resolveBackendError("be.error.actionAlreadyResolved");
    expect(reason).not.toBe("be.error.actionAlreadyResolved");
    expect(await screen.findByRole("alert")).toHaveTextContent(reason);
    expect(onGameUpdate).not.toHaveBeenCalled();

    mockedInvoke.mockImplementation((command) =>
      command === "get_finance_snapshot"
        ? Promise.resolve(wireSnapshot())
        : Promise.resolve({ game: createGameState({}, []), effect: "Offer accepted" }),
    );
    fireEvent.click(screen.getByRole("button", { name: "Accept the deal" }));
    await waitFor(() => expect(screen.queryByRole("alert")).toBeNull());
  });

  // Given two pending sponsor offers, and a refusal of the answer to the first
  // When the first's "Accept the deal" is pressed
  // Then the reason is shown within that offer only, and the second offer shows none.
  it("shows a refused sponsor answer's reason beside its own offer only", async () => {
    const first = createSponsorOfferMessage({ subject: "Sponsorship Offer — First Co" });
    const second = createSponsorOfferMessage({
      id: "sponsor_2025-06-22",
      subject: "Sponsorship Offer — Second Co",
    });
    mockedInvoke.mockImplementation((command) => {
      if (command === "get_finance_snapshot") return Promise.resolve(wireSnapshot());
      if (command === "resolve_message_action") {
        return Promise.reject("be.error.actionAlreadyResolved");
      }
      return Promise.reject(new Error(`Unexpected command: ${command}`));
    });
    await renderFinances(
      <FinancesTab gameState={createGameState({}, [first, second])} onGameUpdate={vi.fn()} />,
    );

    fireEvent.click(screen.getAllByRole("button", { name: "Accept the deal" })[0]);

    const alert = await screen.findByRole("alert");
    const cardOf = (subject: string) =>
      screen.getByRole("heading", { name: subject }).parentElement?.parentElement as HTMLElement;
    expect(cardOf("Sponsorship Offer — First Co")).toContainElement(alert);
    expect(cardOf("Sponsorship Offer — Second Co")).not.toContainElement(alert);
    expect(screen.getAllByRole("alert")).toHaveLength(1);
  });

  // Given a sponsor offer being answered (the backend has not replied)
  // When the offer's buttons are read
  // Then both answers wait, not only the one pressed: a second, opposite answer must not be sent
  //      while the first is on its way.
  it("disables both answers to a sponsor offer while either is on its way", async () => {
    const initialState = createGameState({}, [createSponsorOfferMessage()]);
    mockedInvoke.mockImplementation((command) => {
      if (command === "get_finance_snapshot") return Promise.resolve(wireSnapshot());
      if (command === "resolve_message_action") return new Promise(() => {});
      return Promise.reject(new Error(`Unexpected command: ${command}`));
    });
    await renderFinances(<FinancesTab gameState={initialState} onGameUpdate={vi.fn()} />);

    fireEvent.click(screen.getByRole("button", { name: "Accept the deal" }));

    await waitFor(() =>
      expect(screen.getByRole("button", { name: "Accept the deal" })).toBeDisabled(),
    );
    expect(screen.getByRole("button", { name: "Decline politely" })).toBeDisabled();
    fireEvent.click(screen.getByRole("button", { name: "Decline politely" }));
    expect(
      mockedInvoke.mock.calls.filter(([command]) => command === "resolve_message_action"),
    ).toHaveLength(1);
  });

  // Given a sponsor offer being answered (the backend has not replied)
  // When another finance action is started in the meantime (board support)
  // Then both answers to the offer are still disabled: the offer's own pending state is not the
  //      tab's one shared "loading" slot, which the other action takes over.
  it("keeps both sponsor answers disabled when another finance action starts meanwhile", async () => {
    const initialState = createGameState({}, [createSponsorOfferMessage()]);
    mockedInvoke.mockImplementation((command) => {
      if (command === "get_finance_snapshot") {
        return Promise.resolve(
          wireSnapshot(
            {},
            {
              board_support: {
                support_amount: 1,
                transfer_budget_reduction: 1,
                satisfaction_penalty: 1,
              },
            },
          ),
        );
      }
      if (command === "resolve_message_action" || command === "request_board_support") {
        return new Promise(() => {});
      }
      return Promise.reject(new Error(`Unexpected command: ${command}`));
    });
    await renderFinances(<FinancesTab gameState={initialState} onGameUpdate={vi.fn()} />);

    fireEvent.click(screen.getByRole("button", { name: "Accept the deal" }));
    fireEvent.click(screen.getByRole("button", { name: "finances.requestBoardSupport" }));

    await waitFor(() =>
      expect(screen.getByRole("button", { name: "finances.requestBoardSupport" })).toBeDisabled(),
    );
    expect(screen.getByRole("button", { name: "Accept the deal" })).toBeDisabled();
    expect(screen.getByRole("button", { name: "Decline politely" })).toBeDisabled();
    fireEvent.click(screen.getByRole("button", { name: "Decline politely" }));
    expect(
      mockedInvoke.mock.calls.filter(([command]) => command === "resolve_message_action"),
    ).toHaveLength(1);
  });

  it("requests a sponsor pitch for a pressured club and publishes the updated state", async () => {
    const initialState = createGameState(
      { wage_budget: 50000 },
      [],
      [createPlayer({ wage: 5200000 })],
    );
    const updatedState = createGameState(
      { wage_budget: 50000 },
      [createSponsorOfferMessage()],
      [createPlayer({ wage: 5200000 })],
    );
    const onGameUpdate = vi.fn();

    mockedInvoke.mockImplementation((command) => {
      if (command === "get_finance_snapshot") {
        return Promise.resolve({
          snapshot: {
            committed_transfer_fees: 0,
            available_transfer_budget: 300000,
            annual_wage_bill: 5200000,
            weekly_wage_spend: 100000,
            weekly_wage_budget: 962,
            weekly_recurring_income: 0,
            weekly_sponsor_income: 0,
            projected_weekly_net: -100000,
            cash_runway_weeks: 9,
            wage_budget_usage_percent: 10400,
            currently_in_debt: false,
            currently_over_budget: true,
            wage_budget_status: "critical",
            runway_status: "watch",
            overall_status: "critical",
            marketing_campaign_cooldown_days_remaining: 0,
            cash_flow: [],
          },
          previews: {
            sponsor_pitch: {
              sponsor_name: "Summit Capital",
              weekly_amount: 85000,
              duration_weeks: 12,
            },
          },
        });
      }

      if (command === "request_sponsor_pitch") {
        return Promise.resolve({
          game: updatedState,
          result: {
            message_id: "sponsor_pitch_2025-01-20",
            sponsor_name: "Summit Capital",
            weekly_amount: 85000,
            duration_weeks: 12,
          },
        });
      }

      return Promise.reject(new Error(`Unexpected command: ${command}`));
    });

    await renderFinances(<FinancesTab gameState={initialState} onGameUpdate={onGameUpdate} />);

    fireEvent.click(screen.getByRole("button", { name: "Pitch Sponsor" }));

    await waitFor(() => {
      expect(mockedInvoke).toHaveBeenCalledWith("request_sponsor_pitch", { careerToken: 0 });
    });

    expect(onGameUpdate).toHaveBeenCalledWith(updatedState);
    // The preview and the result summary read the same, so both are on screen.
    expect(screen.getAllByText("Summit Capital will pay €85,000 for 12 weeks")).toHaveLength(2);
  });

  it("launches a marketing campaign for a pressured club and publishes the updated state", async () => {
    const initialState = createGameState(
      { wage_budget: 50000, finance: -30000 },
      [],
      [createPlayer({ wage: 5200000 })],
    );
    const updatedState = createGameState(
      { wage_budget: 50000, finance: 82500 },
      [],
      [createPlayer({ wage: 5200000 })],
    );
    const onGameUpdate = vi.fn();

    mockedInvoke.mockImplementation((command) => {
      if (command === "get_finance_snapshot") {
        return Promise.resolve({
          snapshot: {
            committed_transfer_fees: 0,
            available_transfer_budget: 300000,
            annual_wage_bill: 5200000,
            weekly_wage_spend: 100000,
            weekly_wage_budget: 962,
            weekly_recurring_income: 0,
            weekly_sponsor_income: 0,
            projected_weekly_net: -100000,
            cash_runway_weeks: 3,
            wage_budget_usage_percent: 10400,
            currently_in_debt: true,
            currently_over_budget: true,
            wage_budget_status: "critical",
            runway_status: "critical",
            overall_status: "critical",
            marketing_campaign_cooldown_days_remaining: 0,
            cash_flow: [],
          },
          previews: {
            marketing_campaign: {
              gross_revenue: 150000,
              campaign_cost: 50000,
              net_income: 100000,
              cooldown_days: 28,
            },
          },
        });
      }

      if (command === "request_marketing_campaign") {
        return Promise.resolve({
          game: updatedState,
          result: {
            message_id: "marketing_campaign_2025-06-16",
            gross_revenue: 150000,
            campaign_cost: 37500,
            net_income: 112500,
            cooldown_days: 28,
          },
        });
      }

      return Promise.reject(new Error(`Unexpected command: ${command}`));
    });

    await renderFinances(<FinancesTab gameState={initialState} onGameUpdate={onGameUpdate} />);

    fireEvent.click(screen.getByRole("button", { name: "Launch Campaign" }));

    await waitFor(() => {
      expect(mockedInvoke).toHaveBeenCalledWith("request_marketing_campaign", { careerToken: 0 });
    });

    expect(onGameUpdate).toHaveBeenCalledWith(updatedState);
    expect(
      screen.getByText(
        (_, node) =>
          node?.textContent ===
          "Campaign netted €112,500 after €37,500 in spend (€150,000 gross). Cooldown: 28 days",
      ),
    ).toBeInTheDocument();
  });

  it("renders recovery previews from the backend finance snapshot", async () => {
    const initialState = createGameState(
      { wage_budget: 50000, finance: -30000 },
      [],
      [createPlayer({ wage: 5200000 })],
    );

    mockedInvoke.mockImplementation((command) => {
      if (command === "get_finance_snapshot") {
        return Promise.resolve({
          snapshot: {
            committed_transfer_fees: 0,
            available_transfer_budget: 300000,
            annual_wage_bill: 5200000,
            weekly_wage_spend: 100000,
            weekly_wage_budget: 962,
            weekly_recurring_income: 0,
            weekly_sponsor_income: 0,
            projected_weekly_net: -100000,
            cash_runway_weeks: 3,
            wage_budget_usage_percent: 10400,
            currently_in_debt: true,
            currently_over_budget: true,
            wage_budget_status: "critical",
            runway_status: "critical",
            overall_status: "critical",
            marketing_campaign_cooldown_days_remaining: 0,
            cash_flow: [],
          },
          previews: {
            board_support: {
              support_amount: 150000,
              transfer_budget_reduction: 75000,
              satisfaction_penalty: 12,
            },
            sponsor_pitch: {
              sponsor_name: "Summit Capital",
              weekly_amount: 85000,
              duration_weeks: 12,
            },
            marketing_campaign: {
              gross_revenue: 150000,
              campaign_cost: 37500,
              net_income: 112500,
              cooldown_days: 28,
            },
          },
        });
      }

      return Promise.reject(new Error(`Unexpected command: ${command}`));
    });

    await renderFinances(<FinancesTab gameState={initialState} />);

    await waitFor(() => {
      expect(
        screen.getByText(
          "Board could inject €150,000, cut transfer budget by €75,000, confidence -12",
        ),
      ).toBeInTheDocument();
    });

    expect(screen.getByText("Summit Capital will pay €85,000 for 12 weeks")).toBeInTheDocument();
    expect(
      screen.getByText(
        (_, node) =>
          node?.textContent ===
          "Campaign netted €112,500 after €37,500 in spend (€150,000 gross). Cooldown: 28 days",
      ),
    ).toBeInTheDocument();
  });

  it("disables the marketing campaign action while the campaign is cooling down", async () => {
    const initialState = createGameState(
      { wage_budget: 50000, finance: -30000 },
      [],
      [createPlayer({ wage: 5200000 })],
    );

    mockedInvoke.mockImplementation((command) => {
      if (command === "get_finance_snapshot") {
        return Promise.resolve({
          snapshot: {
            committed_transfer_fees: 0,
            available_transfer_budget: 300000,
            annual_wage_bill: 5200000,
            weekly_wage_spend: 100000,
            weekly_wage_budget: 962,
            weekly_recurring_income: 0,
            weekly_sponsor_income: 0,
            projected_weekly_net: -100000,
            cash_runway_weeks: 3,
            wage_budget_usage_percent: 10400,
            currently_in_debt: true,
            currently_over_budget: true,
            wage_budget_status: "critical",
            runway_status: "critical",
            overall_status: "critical",
            marketing_campaign_cooldown_days_remaining: 9,
            cash_flow: [],
          },
        });
      }

      return Promise.reject(new Error(`Unexpected command: ${command}`));
    });

    await renderFinances(<FinancesTab gameState={initialState} />);

    await waitFor(() => {
      expect(screen.getByRole("button", { name: "Launch Campaign" })).toBeDisabled();
    });

    expect(screen.getByText("Marketing campaign available again in 9 days")).toBeInTheDocument();
  });

  /**
   * Given the backend blocks every facility upgrade because the club is in financial distress,
   * and, as the control, the same club state with nothing blocked,
   * when the Finances tab shows the facilities,
   * then the blocked upgrades are disabled and say why, and the unblocked ones are enabled.
   * The rule (which statuses block) is Rust's and is tested in finances_tests.rs; this tab only
   * presents the verdict the previews carry.
   */
  it.each([
    { blockReason: "be.error.finance.facilityUpgradeCritical", blocked: true },
    { blockReason: null, blocked: false },
  ])(
    "presents the backend's facility-upgrade block (blocked: $blocked)",
    async ({ blockReason, blocked }) => {
      const gameState = createGameState({ finance: 1000000 });
      mockedInvoke.mockImplementation((command) => {
        if (command === "get_finance_snapshot") {
          return Promise.resolve(
            wireSnapshot(
              { runway_status: "warning", overall_status: "warning" },
              { facility_upgrades: wireFacilityUpgrades({ block_reason: blockReason }) },
            ),
          );
        }

        return Promise.reject(new Error(`Unexpected command: ${command}`));
      });

      await renderFinances(<FinancesTab gameState={gameState} />);

      const upgradeButtons = await screen.findAllByRole("button", { name: "Upgrade" });
      expect(upgradeButtons).toHaveLength(3);
      await waitFor(() => {
        for (const button of upgradeButtons) {
          if (blocked) expect(button).toBeDisabled();
          else expect(button).toBeEnabled();
        }
      });
      const reason = resolveBackendError("be.error.finance.facilityUpgradeCritical");
      expect(screen.queryAllByText(reason)).toHaveLength(blocked ? 3 : 0);
    },
  );

  it("renders a cash-flow projection panel using wages, sponsorship income, and runway", async () => {
    const gameState = createGameState(
      {
        finance: 280000,
        sponsorship: {
          sponsor_name: "Acme Corp",
          base_value: 10000,
          remaining_weeks: 8,
          bonus_criteria: [],
        },
      },
      [],
      [createPlayer({ wage: 40000, market_value: 200000 })],
    );

    mockedInvoke.mockResolvedValue(
      wireSnapshot({
        weekly_wage_spend: 40000,
        weekly_sponsor_income: 10000,
        projected_weekly_net: -30000,
        cash_runway_weeks: 9,
      }),
    );

    await renderFinances(<FinancesTab gameState={gameState} />);

    expect(screen.getByText("Cash Flow")).toBeInTheDocument();
    expect(screen.getByText("Weekly Wage Spend")).toBeInTheDocument();
    expect(screen.getByText("Weekly Sponsor Income")).toBeInTheDocument();
    expect(screen.getByText("Projected Weekly Net")).toBeInTheDocument();
    expect(screen.getByText("Cash Runway")).toBeInTheDocument();
    expect(screen.getByText("€10K/wk")).toBeInTheDocument();
    expect(screen.getByText("-€30K/wk")).toBeInTheDocument();
    expect(screen.getByText("9 weeks at current pace")).toBeInTheDocument();
  });

  it("renders wage pressure and contract risk indicators for expiring players", async () => {
    const onSelectPlayer = vi.fn();
    const gameState = createGameState(
      {
        wage_budget: 50000,
      },
      [],
      [
        createPlayer({
          id: "player-critical",
          full_name: "Alex Critical",
          wage: 35000,
          contract_end: "2025-04-30",
        }),
        createPlayer({
          id: "player-warning",
          full_name: "Ben Warning",
          wage: 25000,
          contract_end: "2025-10-15",
        }),
        createPlayer({
          id: "player-stable",
          full_name: "Carl Stable",
          wage: 5000,
          contract_end: "2027-06-30",
        }),
      ],
    );

    mockedInvoke.mockResolvedValue(
      wireSnapshot({ weekly_wage_spend: 60000, wage_budget_usage_percent: 130 }),
    );

    await renderFinances(<FinancesTab gameState={gameState} onSelectPlayer={onSelectPlayer} />);

    expect(screen.getAllByText("Wage Pressure").length).toBeGreaterThan(0);
    expect(screen.getByText("130% of wage budget used")).toBeInTheDocument();
    expect(screen.getByText("Contract Risk")).toBeInTheDocument();
    expect(screen.getAllByText("Alex Critical").length).toBeGreaterThan(0);
    expect(screen.getAllByText("Ben Warning").length).toBeGreaterThan(0);
    expect(screen.getByText("Critical")).toBeInTheDocument();
    expect(screen.getByText("Warning")).toBeInTheDocument();
    expect(screen.getByText("Expires April 30, 2025")).toBeInTheDocument();
    expect(screen.getByText("Expires October 15, 2025")).toBeInTheDocument();
    expect(screen.getByText("€60,000/wk at risk")).toBeInTheDocument();
    expect(screen.getAllByRole("button", { name: "Renew Contract" })).toHaveLength(2);

    fireEvent.click(screen.getAllByRole("button", { name: "Renew Contract" })[0]);

    expect(onSelectPlayer).toHaveBeenCalledWith("player-critical", {
      openRenewal: true,
    });
  });

  /**
   * Given a squad with two players whose contracts are at risk
   * When the finance panel first appears on screen
   * Then both are already selected, so a click that lands before any effect has run cannot flip
   * the default (the delegate test below clicks the moment "Facilities" shows).
   */
  it("shows every at-risk player selected from the first frame they are on screen", async () => {
    const riskyPlayers = [
      createPlayer({
        id: "player-critical",
        full_name: "Alex Critical",
        contract_end: "2025-04-30",
      }),
      createPlayer({ id: "player-warning", full_name: "Ben Warning", contract_end: "2025-10-15" }),
    ];
    mockedInvoke.mockImplementation((command) =>
      command === "get_finance_snapshot"
        ? Promise.resolve(wireSnapshot())
        : Promise.reject(new Error(`Unexpected command: ${command}`)),
    );
    // A Profiler's onRender runs while a commit is being made, after the DOM has changed and before
    // that commit's passive effects, whatever the scheduler does. Reading the checkbox there sees
    // the first frame by construction rather than by how React happens to order its tasks.
    let checkedOnFirstAppearance: boolean | null = null;
    const readFirstFrame = () => {
      const box = document.querySelector<HTMLInputElement>(
        'input[aria-label="Select Ben Warning"]',
      );
      if (box && checkedOnFirstAppearance === null) checkedOnFirstAppearance = box.checked;
    };

    await renderFinances(
      <Profiler id="finances" onRender={readFirstFrame}>
        <FinancesTab gameState={createGameState({ wage_budget: 50000 }, [], riskyPlayers)} />
      </Profiler>,
    );

    expect(checkedOnFirstAppearance).toBe(true);
  });

  /**
   * Given two at-risk players, both selected
   * When "Select all" is pressed twice
   * Then the first press clears the selection and the second selects everyone again.
   */
  it("toggles every at-risk player off and back on with Select all", async () => {
    const riskyPlayers = [
      createPlayer({
        id: "player-critical",
        full_name: "Alex Critical",
        contract_end: "2025-04-30",
      }),
      createPlayer({ id: "player-warning", full_name: "Ben Warning", contract_end: "2025-10-15" }),
    ];
    mockedInvoke.mockImplementation((command) =>
      command === "get_finance_snapshot"
        ? Promise.resolve(wireSnapshot())
        : Promise.reject(new Error(`Unexpected command: ${command}`)),
    );
    await renderFinances(
      <FinancesTab gameState={createGameState({ wage_budget: 50000 }, [], riskyPlayers)} />,
    );
    const critical = screen.getByLabelText("Select Alex Critical");
    const warning = screen.getByLabelText("Select Ben Warning");

    fireEvent.click(screen.getByRole("button", { name: "Select all" }));
    expect(critical).not.toBeChecked();
    expect(warning).not.toBeChecked();

    fireEvent.click(screen.getByRole("button", { name: "Select all" }));
    expect(critical).toBeChecked();
    expect(warning).toBeChecked();
  });

  /**
   * Given two at-risk players, both selected
   * When Ben is switched off and then on again
   * Then the delegate call carries both players.
   */
  it("delegates a player who was switched off and then on again", async () => {
    const riskyPlayers = [
      createPlayer({
        id: "player-critical",
        full_name: "Alex Critical",
        contract_end: "2025-04-30",
      }),
      createPlayer({ id: "player-warning", full_name: "Ben Warning", contract_end: "2025-10-15" }),
    ];
    mockedInvoke.mockImplementation((command) => {
      if (command === "get_finance_snapshot") return Promise.resolve(wireSnapshot());
      if (command === "delegate_renewals") {
        return Promise.resolve({
          game: createGameState({ wage_budget: 50000 }, [], riskyPlayers),
          report: { success_count: 2, failure_count: 0, stalled_count: 0, cases: [] },
        });
      }
      return Promise.reject(new Error(`Unexpected command: ${command}`));
    });
    await renderFinances(
      <FinancesTab gameState={createGameState({ wage_budget: 50000 }, [], riskyPlayers)} />,
    );

    fireEvent.click(screen.getByLabelText("Select Ben Warning"));
    fireEvent.click(screen.getByLabelText("Select Ben Warning"));
    fireEvent.click(screen.getByRole("button", { name: "Delegate Selected Renewals" }));

    await waitFor(() => {
      expect(mockedInvoke).toHaveBeenCalledWith("delegate_renewals", {
        careerToken: 0,
        playerIds: ["player-critical", "player-warning"],
        maxWageIncreasePct: 35,
        maxContractYears: 3,
      });
    });
  });

  it("delegates only the selected risky renewals to the assistant and publishes the updated state", async () => {
    const riskyPlayers = [
      createPlayer({
        id: "player-critical",
        full_name: "Alex Critical",
        wage: 35000,
        contract_end: "2025-04-30",
      }),
      createPlayer({
        id: "player-warning",
        full_name: "Ben Warning",
        wage: 25000,
        contract_end: "2025-10-15",
      }),
    ];
    const initialState = createGameState({ wage_budget: 50000 }, [], riskyPlayers);
    const updatedState = createGameState(
      { wage_budget: 50000 },
      [],
      [
        createPlayer({
          id: "player-critical",
          full_name: "Alex Critical",
          wage: 36000,
          contract_end: "2028-01-20",
        }),
        createPlayer({
          id: "player-warning",
          full_name: "Ben Warning",
          wage: 25000,
          contract_end: "2025-10-15",
        }),
      ],
    );
    const onGameUpdate = vi.fn();

    mockedInvoke.mockImplementation((command) => {
      if (command === "get_finance_snapshot") {
        return Promise.resolve(wireSnapshot());
      }

      if (command === "delegate_renewals") {
        return Promise.resolve({
          game: updatedState,
          report: {
            success_count: 1,
            failure_count: 0,
            stalled_count: 1,
            cases: [],
          },
        });
      }

      return Promise.reject(new Error(`Unexpected command: ${command}`));
    });

    await renderFinances(<FinancesTab gameState={initialState} onGameUpdate={onGameUpdate} />);

    fireEvent.click(screen.getByLabelText("Select Ben Warning"));

    fireEvent.click(screen.getByRole("button", { name: "Delegate Selected Renewals" }));

    await waitFor(() => {
      expect(mockedInvoke).toHaveBeenCalledWith("delegate_renewals", {
        careerToken: 0,
        playerIds: ["player-critical"],
        maxWageIncreasePct: 35,
        maxContractYears: 3,
      });
    });

    expect(onGameUpdate).toHaveBeenCalledWith(updatedState);
    expect(screen.getByText("1 done, 1 pending, 0 failed")).toBeInTheDocument();
  });
});
