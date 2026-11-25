import {
  pendingPromise,
  wireFacilityUpgrades,
  wireSnapshot,
  renderFinances,
  createGameState,
} from "./FinancesTab.testFixtures";
import { fireEvent, render, screen, waitFor } from "@testing-library/react";

import { beforeEach, describe, expect, it, vi } from "vitest";
import { invoke } from "@tauri-apps/api/core";
import type { GameStateData } from "../../store/gameStore";

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

describe("FinancesTab team guard", () => {
  beforeEach(() => {
    mockedInvoke.mockReset();
    mockedInvoke.mockImplementation((command) => {
      if (command === "get_finance_snapshot") {
        return Promise.resolve(wireSnapshot());
      }

      return Promise.reject(new Error(`Unexpected command: ${command}`));
    });
  });

  it("shows the no-team message when the manager has no club", () => {
    const gameState = createGameState();
    gameState.manager.team_id = null;

    render(<FinancesTab gameState={gameState} />);

    expect(screen.getByText("No team")).toBeInTheDocument();
  });

  it("survives the manager gaining a team without remounting", async () => {
    const unemployed = createGameState();
    unemployed.manager.team_id = null;

    const { rerender } = render(<FinancesTab gameState={unemployed} />);
    expect(screen.getByText("No team")).toBeInTheDocument();

    // Same component instance, team now resolves. When the guard sat above the
    // hooks this rerender threw "Rendered more hooks than during the previous
    // render" because the hook count went from 1 to 11.
    rerender(<FinancesTab gameState={createGameState()} />);

    expect(screen.queryByText("No team")).not.toBeInTheDocument();
    expect(await screen.findByText("Facilities")).toBeInTheDocument();
  });
});
describe("FinancesTab backend snapshot", () => {
  beforeEach(() => {
    mockedInvoke.mockReset();
  });

  /**
   * Given a club whose old financial ledger is empty (wages, gates and sponsorship never reach it)
   *       and a backend snapshot carrying two weeks of cash flow from the cash journal
   * When the tab renders
   * Then the chart is drawn from the snapshot's weeks, as the backend sent them (#1204).
   */
  it("draws the cash-flow chart from the snapshot's weeks, not the old ledger", async () => {
    chartWeeks.current = undefined;
    mockedInvoke.mockResolvedValue(
      wireSnapshot({
        cash_flow: [
          { week_start: "2025-01-06", income: 25000, expenses: 40000 },
          { week_start: "2025-01-13", income: 310000, expenses: 40000 },
        ],
      }),
    );

    await renderFinances(<FinancesTab gameState={createGameState({ financial_ledger: [] })} />);

    expect(screen.getByTestId("cash-flow-chart")).toBeInTheDocument();
    expect(chartWeeks.current).toEqual([
      { weekStart: "2025-01-06", income: 25000, expenses: 40000 },
      { weekStart: "2025-01-13", income: 310000, expenses: 40000 },
    ]);
    // Its weeks hold transfer and loan fees too, which the season figures leave out, so its series
    // are money in and out, not the season's income and expenses (review, #1232).
    expect(chartWeeks.labels).toEqual(["finances.moneyIn", "finances.moneyOut"]);
  });

  /**
   * Given a backend snapshot with no weeks of cash flow (a club whose money has not moved yet)
   * When the tab renders
   * Then no chart is drawn, rather than an empty one.
   */
  it("draws no cash-flow chart before any money has moved", async () => {
    mockedInvoke.mockResolvedValue(wireSnapshot());

    await renderFinances(<FinancesTab gameState={createGameState()} />);

    expect(screen.queryByTestId("cash-flow-chart")).not.toBeInTheDocument();
  });

  /**
   * Given the backend snapshot has not arrived
   * When the tab renders
   * Then it shows a placeholder and no figures, rather than a locally projected guess.
   */
  it("shows a placeholder until the backend snapshot arrives", () => {
    mockedInvoke.mockImplementation(() => pendingPromise());

    render(<FinancesTab gameState={createGameState()} />);

    expect(screen.getByText("common.loading")).toBeInTheDocument();
    expect(screen.queryByText("Facilities")).not.toBeInTheDocument();
  });

  /**
   * Given a rendered tab and a club whose state then changes
   * When the next snapshot is still in flight
   * Then the previous backend figures stay on screen instead of flipping to another source.
   */
  it("keeps the last backend figures while a newer snapshot is loading", async () => {
    let calls = 0;
    mockedInvoke.mockImplementation((command) => {
      if (command !== "get_finance_snapshot") {
        return Promise.reject(new Error(`Unexpected command: ${command}`));
      }
      calls += 1;
      return calls === 1
        ? Promise.resolve(wireSnapshot({ weekly_sponsor_income: 777_000 }))
        : pendingPromise();
    });

    const { rerender } = render(<FinancesTab gameState={createGameState()} />);
    await screen.findByText("Facilities");
    const sponsorFigure = screen.getByText(/777/);

    rerender(<FinancesTab gameState={createGameState({ finance: 123_456 })} />);

    await waitFor(() => expect(calls).toBe(2));
    expect(screen.getByText("Facilities")).toBeInTheDocument();
    expect(sponsorFigure).toBeInTheDocument();
    expect(screen.queryByText("common.loading")).not.toBeInTheDocument();
  });

  /**
   * Given the backend snapshot fails to load
   * When the player presses Retry
   * Then the snapshot is requested again and the tab renders.
   */
  it("offers a retry when the snapshot cannot be loaded", async () => {
    let calls = 0;
    mockedInvoke.mockImplementation((command) => {
      if (command !== "get_finance_snapshot") {
        return Promise.reject(new Error(`Unexpected command: ${command}`));
      }
      calls += 1;
      return calls === 1 ? Promise.reject(new Error("boom")) : Promise.resolve(wireSnapshot());
    });
    vi.spyOn(console, "error").mockImplementation(() => {});

    render(<FinancesTab gameState={createGameState()} />);

    expect(await screen.findByRole("alert")).toHaveTextContent("finances.snapshotLoadFailed");
    fireEvent.click(screen.getByRole("button", { name: "common.retry" }));

    expect(await screen.findByText("Facilities")).toBeInTheDocument();
  });

  /**
   * Given the backend offers board support and every facility upgrade
   * When the club state changes and the next snapshot is still in flight
   * Then no action is enabled: a verdict about the older state must not authorise anything.
   */
  it("never enables an action from previews that describe an older club state", async () => {
    let calls = 0;
    mockedInvoke.mockImplementation((command) => {
      if (command !== "get_finance_snapshot") {
        return Promise.reject(new Error(`Unexpected command: ${command}`));
      }
      calls += 1;
      return calls === 1
        ? Promise.resolve(
            wireSnapshot(
              {},
              {
                facility_upgrades: wireFacilityUpgrades(),
                board_support: {
                  support_amount: 1,
                  transfer_budget_reduction: 1,
                  satisfaction_penalty: 1,
                },
              },
            ),
          )
        : pendingPromise();
    });
    const { rerender } = render(<FinancesTab gameState={createGameState()} />);
    await screen.findByText("Facilities");
    const fresh = screen.getAllByRole("button", { name: "Upgrade" });
    expect(fresh.every((button) => !(button as HTMLButtonElement).disabled)).toBe(true);
    expect(screen.getByRole("button", { name: "finances.requestBoardSupport" })).toBeEnabled();

    rerender(<FinancesTab gameState={createGameState({ finance: 123_456 })} />);

    await waitFor(() => expect(calls).toBe(2));
    const stale = screen.getAllByRole("button", { name: "Upgrade" });
    expect(stale.every((button) => (button as HTMLButtonElement).disabled)).toBe(true);
    expect(screen.getByRole("button", { name: "finances.requestBoardSupport" })).toBeDisabled();
  });

  /**
   * Given a snapshot retained for one club
   * When the manager's club changes and the new club's snapshot has not arrived
   * Then the other club's figures are not shown.
   */
  it("does not render another club's retained snapshot", async () => {
    mockedInvoke.mockImplementationOnce(() =>
      Promise.resolve(wireSnapshot({ weekly_sponsor_income: 777_000 })),
    );
    mockedInvoke.mockImplementation(() => pendingPromise());
    const { rerender } = render(<FinancesTab gameState={createGameState()} />);
    await screen.findByText(/777/);

    const otherClub = createGameState({ id: "team-2", name: "Beta FC" });
    otherClub.manager.team_id = "team-2";
    rerender(<FinancesTab gameState={otherClub} />);

    expect(screen.queryByText(/777/)).not.toBeInTheDocument();
    expect(screen.getByText("common.loading")).toBeInTheDocument();
  });

  /**
   * Given figures on screen and a refetch that fails
   * When the tab renders
   * Then the figures stay and the failure shows with Retry, and a club change clears it.
   */
  it("shows a failed refetch beside the retained figures and clears it on a club change", async () => {
    let calls = 0;
    mockedInvoke.mockImplementation((command) => {
      if (command !== "get_finance_snapshot") {
        return Promise.reject(new Error(`Unexpected command: ${command}`));
      }
      calls += 1;
      if (calls === 1) return Promise.resolve(wireSnapshot({ weekly_sponsor_income: 777_000 }));
      if (calls === 2) return Promise.reject(new Error("boom"));
      return pendingPromise();
    });
    vi.spyOn(console, "error").mockImplementation(() => {});
    const { rerender } = render(<FinancesTab gameState={createGameState()} />);
    await screen.findByText(/777/);

    rerender(<FinancesTab gameState={createGameState({ finance: 123_456 })} />);

    expect(await screen.findByRole("alert")).toHaveTextContent("finances.snapshotLoadFailed");
    expect(screen.getByText(/777/)).toBeInTheDocument();
    expect(screen.getByRole("button", { name: "common.retry" })).toBeInTheDocument();

    rerender(<FinancesTab gameState={createGameState({ finance: 99 })} />);

    await waitFor(() => expect(screen.queryByRole("alert")).not.toBeInTheDocument());
  });

  /**
   * Given figures on screen
   * When a live match finishes: the team's form changes, but the game date does not move
   * Then the snapshot is fetched again, because sponsorship and matchday figures changed.
   */
  it("refetches when a finished match changes the form on an unchanged date", async () => {
    let calls = 0;
    mockedInvoke.mockImplementation((command) => {
      if (command !== "get_finance_snapshot") {
        return Promise.reject(new Error(`Unexpected command: ${command}`));
      }
      calls += 1;
      return Promise.resolve(wireSnapshot());
    });
    const { rerender } = render(<FinancesTab gameState={createGameState({ form: [] })} />);
    await waitFor(() => expect(calls).toBe(1));

    rerender(<FinancesTab gameState={createGameState({ form: ["W"] })} />);

    await waitFor(() => expect(calls).toBe(2));
  });

  /**
   * Given figures on screen
   * When a fixture of the club completes on an unchanged date and the form is unchanged
   * Then the snapshot is fetched again: standings and matchday income moved.
   */
  it("refetches when the club's completed fixtures change on an unchanged date", async () => {
    let calls = 0;
    mockedInvoke.mockImplementation((command) => {
      if (command !== "get_finance_snapshot") {
        return Promise.reject(new Error(`Unexpected command: ${command}`));
      }
      calls += 1;
      return Promise.resolve(wireSnapshot());
    });
    const before = createGameState();
    const after = createGameState();
    after.league = {
      ...after.league!,
      fixtures: [
        {
          id: "fx-1",
          date: "2025-01-20",
          home_team_id: "team-1",
          away_team_id: "team-2",
          status: "Completed",
          result: null,
        } as unknown as NonNullable<GameStateData["league"]>["fixtures"][number],
      ],
    };
    const { rerender } = render(<FinancesTab gameState={before} />);
    await waitFor(() => expect(calls).toBe(1));

    rerender(<FinancesTab gameState={after} />);

    await waitFor(() => expect(calls).toBe(2));
  });
});

/** Given a committed fee absent from a slim local player list, when a fresh career state arrives on the same date, then Finances refreshes its backend budget. */
it("refreshes commitments when a fresh slim career state has the same finance key", async () => {
  riskLevels.current = {};
  mockedInvoke.mockReset();
  mockedInvoke
    .mockResolvedValueOnce(wireSnapshot())
    .mockResolvedValueOnce(
      wireSnapshot({ committed_transfer_fees: 80000, available_transfer_budget: 220000 }),
    );
  const gameState = createGameState({}, [], []);
  const view = await renderFinances(<FinancesTab gameState={gameState} />);
  expect(screen.getByText(/300/)).toBeInTheDocument();
  view.rerender(<FinancesTab gameState={{ ...gameState }} />);
  expect(await screen.findByText(/220/)).toBeInTheDocument();
});
