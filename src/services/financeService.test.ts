import { beforeEach, describe, expect, it, vi } from "vitest";
import { invoke } from "@tauri-apps/api/core";

import { getFinanceSnapshot, reallocateBoardEnvelopes } from "./financeService";

vi.mock("@tauri-apps/api/core", () => ({
  invoke: vi.fn(),
}));

const mockedInvoke = vi.mocked(invoke);

describe("financeService", () => {
  beforeEach(() => {
    mockedInvoke.mockReset();
  });

  /** Given a weekly wage delta, when envelopes are reallocated, then the screen-write command receives the camelCase argument and the career token. */
  it("calls reallocate_board_envelopes with a camelCase weekly delta", async () => {
    const response = {
      game: { manager: { id: "manager-1" } },
      result: { wage_budget: 125_000, transfer_budget: 40_000 },
    };
    mockedInvoke.mockResolvedValueOnce(response);

    await expect(reallocateBoardEnvelopes(10_000)).resolves.toBe(response);
    expect(mockedInvoke).toHaveBeenCalledWith("reallocate_board_envelopes", {
      careerToken: 0,
      wageDeltaWeekly: 10_000,
    });
  });

  /** Given the backend refuses the move, when the service is called, then the translation key is surfaced. */
  it("surfaces backend errors from reallocate_board_envelopes", async () => {
    mockedInvoke.mockRejectedValueOnce(new Error("be.error.finance.reallocationInvalid"));

    await expect(reallocateBoardEnvelopes(10_000)).rejects.toThrow(
      "be.error.finance.reallocationInvalid",
    );
  });

  it("calls get_finance_snapshot", async () => {
    mockedInvoke.mockResolvedValueOnce({
      snapshot: {
        committed_transfer_fees: 0,
        available_transfer_budget: 0,
        annual_wage_bill: 1,
        weekly_wage_spend: 1,
        weekly_wage_budget: 1,
        weekly_recurring_income: 1,
        weekly_sponsor_income: 1,
        projected_weekly_net: 0,
        cash_runway_weeks: null,
        wage_budget_usage_percent: 0,
        currently_in_debt: false,
        currently_over_budget: false,
        wage_budget_status: "stable",
        runway_status: "stable",
        overall_status: "stable",
        marketing_campaign_cooldown_days_remaining: 0,
        cash_flow: [],
      },
      previews: null,
    });

    await getFinanceSnapshot("team-1");
    expect(mockedInvoke).toHaveBeenCalledWith("get_finance_snapshot", { teamId: "team-1" });
  });
});
