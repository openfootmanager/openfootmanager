import { act, renderHook, waitFor } from "@testing-library/react";
import { beforeEach, describe, expect, it, vi } from "vitest";

import type { PlayerData } from "../../store/gameStore";
import { useContractSheetData } from "./useContractSheetData";

const invokeMock = vi.hoisted(() => vi.fn());
vi.mock("@tauri-apps/api/core", () => ({ invoke: invokeMock }));

const TERMINATION = {
  preview: {
    player_id: "p1",
    player_name: "Pat Player",
    severance_cost: 250_000,
    squad_safety: {
      team_id: "t1",
      projected_roster_size: 24,
      healthy_players: 18,
      healthy_goalkeepers: 2,
      effective_xi_size: 11,
      can_field_matchday_squad: true,
      missing_reasons: [],
    },
  },
};

const PROJECTION = {
  projection: {
    current_annual_wage_bill: 300_000,
    projected_annual_wage_bill: 300_000,
    annual_wage_budget: 400_000,
    annual_soft_cap: 440_000,
    current_weekly_wage_spend: 300_000,
    projected_weekly_wage_spend: 300_000,
    current_cash_runway_weeks: 40,
    projected_cash_runway_weeks: 40,
    currently_over_budget: false,
    policy_allows: true,
  },
};

const LOAN = {
  parent_team_id: "t1",
  loan_team_id: "t2",
  start_date: "2031-07-01",
  end_date: "2032-06-30",
  wage_contribution_pct: 50,
  buy_option_fee: null,
};

const player = (patch: Partial<PlayerData> = {}) =>
  ({
    id: "p1",
    wage: 10_000,
    contract_end: "2033-06-30",
    active_loan: null,
    ...patch,
  }) as PlayerData;

function answerBoth() {
  invokeMock.mockImplementation(async (command: string) => {
    if (command === "preview_contract_termination") return TERMINATION;
    if (command === "preview_renewal_financial_impact") return PROJECTION;
    throw new Error(`unexpected command ${command}`);
  });
}

const calls = () => invokeMock.mock.calls.map(([command]) => command as string);

beforeEach(() => {
  invokeMock.mockReset();
});

describe("useContractSheetData", () => {
  /**
   * Given his own player and the Contract tab not selected (the profile opens on the Overview and
   *       keeps hidden tabs mounted)
   * When the hook runs
   * Then neither preview is asked for
   */
  it("asks for neither preview while the Contract tab is not the open one", () => {
    answerBoth();

    const { result } = renderHook(() =>
      useContractSheetData({
        player: player(),
        active: false,
        ownsContract: true,
        refreshKey: "a",
      }),
    );

    expect(calls()).toEqual([]);
    expect(result.current.termination.status).toBe("idle");
    expect(result.current.projection.status).toBe("idle");
  });

  /**
   * Given his own player with a contract, and the Contract tab selected
   * When the hook runs
   * Then it asks for the termination preview and for the wage bill at his current wage, and holds
   *      both answers
   */
  it("loads the termination preview and the wage bill when the tab is selected", async () => {
    answerBoth();

    const { result } = renderHook(() =>
      useContractSheetData({ player: player(), active: true, ownsContract: true, refreshKey: "a" }),
    );

    await waitFor(() => expect(result.current.termination.status).toBe("answered"));
    await waitFor(() => expect(result.current.projection.status).toBe("answered"));
    expect(invokeMock).toHaveBeenCalledWith("preview_contract_termination", { playerId: "p1" });
    expect(invokeMock).toHaveBeenCalledWith("preview_renewal_financial_impact", {
      playerId: "p1",
      weeklyWage: 10_000,
    });
    expect(result.current.termination).toMatchObject({
      value: { severance_cost: 250_000, squad_safety: { healthy_players: 18 } },
    });
    expect(result.current.projection).toMatchObject({
      value: { current_weekly_wage_spend: 300_000 },
    });
  });

  /**
   * Given a player on loan, and one with no contract end
   * When the hook runs on the open tab
   * Then no termination preview is asked for (the backend refuses both), and the wage bill is
   */
  it("does not ask to preview releasing a player who cannot be released", async () => {
    answerBoth();

    const loaned = renderHook(() =>
      useContractSheetData({
        player: player({ active_loan: LOAN }),
        active: true,
        ownsContract: true,
        refreshKey: "a",
      }),
    );
    await waitFor(() => expect(loaned.result.current.projection.status).toBe("answered"));
    expect(calls()).not.toContain("preview_contract_termination");
    expect(loaned.result.current.termination.status).toBe("idle");

    invokeMock.mockClear();
    const noContract = renderHook(() =>
      useContractSheetData({
        player: player({ contract_end: null }),
        active: true,
        ownsContract: true,
        refreshKey: "a",
      }),
    );
    await waitFor(() => expect(noContract.result.current.projection.status).toBe("answered"));
    expect(calls()).not.toContain("preview_contract_termination");
  });

  /**
   * Given another club's player (or a free agent), whose contract the manager does not hold
   * When the hook runs on the open tab
   * Then nothing is asked for: the backend refuses both previews for a player that is not his own
   */
  it("asks for nothing about a contract the manager does not hold", () => {
    answerBoth();

    const { result } = renderHook(() =>
      useContractSheetData({
        player: player(),
        active: true,
        ownsContract: false,
        refreshKey: "a",
      }),
    );

    expect(calls()).toEqual([]);
    expect(result.current.termination.status).toBe("idle");
    expect(result.current.projection.status).toBe("idle");
  });

  /**
   * Given a preview that fails, then the same preview asked for again
   * When the hook retries
   * Then the failure is reported as failed (never as "no answer"), and the retry asks again and
   *      holds the answer
   */
  it("reports a failed preview and asks again on retry", async () => {
    invokeMock.mockImplementation(async (command: string) => {
      if (command === "preview_renewal_financial_impact") return PROJECTION;
      throw new Error("backend unavailable");
    });
    const { result } = renderHook(() =>
      useContractSheetData({ player: player(), active: true, ownsContract: true, refreshKey: "a" }),
    );
    await waitFor(() => expect(result.current.termination.status).toBe("failed"));
    expect(result.current.projection.status).toBe("answered");

    answerBoth();
    act(() => result.current.retry());

    await waitFor(() => expect(result.current.termination.status).toBe("answered"));
  });

  /**
   * Given an answer held for one player
   * When the page opens another player on the same hook (it reuses the instance)
   * Then the first player's figures are not shown for the second: the answer is loading again
   */
  it("does not show one player's answer for the next", async () => {
    answerBoth();
    const { result, rerender } = renderHook(
      ({ id }) =>
        useContractSheetData({
          player: player({ id }),
          active: true,
          ownsContract: true,
          refreshKey: "a",
        }),
      { initialProps: { id: "p1" } },
    );
    await waitFor(() => expect(result.current.termination.status).toBe("answered"));

    invokeMock.mockImplementation(() => new Promise(() => {}));
    rerender({ id: "p2" });

    expect(result.current.termination.status).toBe("loading");
    expect(result.current.projection.status).toBe("loading");
  });

  /**
   * Given the answers held, then the game changing under the sheet (an action taken, a day gone)
   * When the refresh key changes
   * Then both are asked for again
   */
  it("asks again when the game changes under the sheet", async () => {
    answerBoth();
    const { result, rerender } = renderHook(
      ({ key }) =>
        useContractSheetData({
          player: player(),
          active: true,
          ownsContract: true,
          refreshKey: key,
        }),
      { initialProps: { key: "a" } },
    );
    await waitFor(() => expect(result.current.projection.status).toBe("answered"));
    invokeMock.mockClear();

    rerender({ key: "b" });

    await waitFor(() =>
      expect(calls().sort()).toEqual([
        "preview_contract_termination",
        "preview_renewal_financial_impact",
      ]),
    );
  });
});
