import { describe, expect, it } from "vitest";
import type { TeamFinanceSnapshot } from "../../lib/finance";
import { createTeam } from "../../test-utils/factories";
import {
  getFacilityUpgradeCost,
  formatSignedAmount,
  facilityUpgradeBlockReason,
  boardSupportAvailable,
  sponsorPitchAvailable,
  marketingCampaignAvailable,
  mapLocalFinanceSnapshot,
  isChooseOptionAction,
  isPendingSponsorOffer,
} from "./FinancesTab.helpers";
import type { MessageData } from "../../store/gameStore";

// wage_budget must exceed annualWageBill (default 1_000_000) so the team is
// not over-budget unless a test explicitly sets annualWageBill higher.
function makeSolventTeam() {
  return createTeam({ finance: 500_000, wage_budget: 2_000_000 });
}

function makeSnapshot(
  overrides: Partial<TeamFinanceSnapshot> = {},
): TeamFinanceSnapshot {
  return {
    annualWageBill: 1_000_000,
    weeklyWageSpend: 20_000,
    weeklyWageBudget: 25_000,
    weeklySponsorIncome: 5_000,
    projectedWeeklyNet: -15_000,
    cashRunwayWeeks: 52,
    wageBudgetUsagePercent: 80,
    wageBudgetStatus: "stable",
    runwayStatus: "stable",
    overallStatus: "stable",
    marketingCampaignCooldownDaysRemaining: 0,
    ...overrides,
  };
}

describe("getFacilityUpgradeCost", () => {
  it("returns level × 250,000", () => {
    expect(getFacilityUpgradeCost(1)).toBe(250_000);
    expect(getFacilityUpgradeCost(3)).toBe(750_000);
    expect(getFacilityUpgradeCost(5)).toBe(1_250_000);
  });
});

describe("formatSignedAmount", () => {
  it("returns formatted value for positive amounts", () => {
    const result = formatSignedAmount(1_000_000);
    expect(result).not.toMatch(/^-/);
    expect(result).toMatch(/1/);
  });

  it("prepends minus sign for negative amounts", () => {
    const result = formatSignedAmount(-500_000);
    expect(result).toMatch(/^-/);
  });

  it("formats zero without a sign", () => {
    const result = formatSignedAmount(0);
    expect(result).not.toMatch(/^-/);
  });
});

describe("facilityUpgradeBlockReason", () => {
  it("returns null when finances are healthy", () => {
    expect(facilityUpgradeBlockReason(mapLocalFinanceSnapshot(makeSolventTeam(), makeSnapshot()))).toBeNull();
  });

  it("returns overBudget error when currently over wage budget", () => {
    const snap = makeSnapshot({ annualWageBill: 2_000_000 });
    const team = createTeam({ finance: 500_000, wage_budget: 1_000_000 });
    const mapped = mapLocalFinanceSnapshot(team, snap);
    expect(facilityUpgradeBlockReason(mapped)).toBe("be.error.finance.facilityUpgradeOverBudget");
  });

  it("returns critical error for warning overall status", () => {
    const snap = makeSnapshot({ overallStatus: "warning" });
    const mapped = mapLocalFinanceSnapshot(makeSolventTeam(), snap);
    expect(facilityUpgradeBlockReason(mapped)).toBe("be.error.finance.facilityUpgradeCritical");
  });

  it("returns critical error for critical overall status", () => {
    const snap = makeSnapshot({ overallStatus: "critical" });
    const mapped = mapLocalFinanceSnapshot(makeSolventTeam(), snap);
    expect(facilityUpgradeBlockReason(mapped)).toBe("be.error.finance.facilityUpgradeCritical");
  });
});

describe("boardSupportAvailable", () => {
  it("returns false when finances are healthy", () => {
    expect(boardSupportAvailable(mapLocalFinanceSnapshot(makeSolventTeam(), makeSnapshot()))).toBe(false);
  });

  it("returns true when club is in debt", () => {
    const team = createTeam({ finance: -1, wage_budget: 2_000_000 });
    expect(boardSupportAvailable(mapLocalFinanceSnapshot(team, makeSnapshot()))).toBe(true);
  });

  it("returns true when runway is warning", () => {
    const snap = makeSnapshot({ runwayStatus: "warning" });
    expect(boardSupportAvailable(mapLocalFinanceSnapshot(makeSolventTeam(), snap))).toBe(true);
  });

  it("returns true when runway is critical", () => {
    const snap = makeSnapshot({ runwayStatus: "critical" });
    expect(boardSupportAvailable(mapLocalFinanceSnapshot(makeSolventTeam(), snap))).toBe(true);
  });
});

describe("sponsorPitchAvailable", () => {
  it("returns false when all finances are healthy", () => {
    expect(sponsorPitchAvailable(mapLocalFinanceSnapshot(makeSolventTeam(), makeSnapshot()))).toBe(false);
  });

  it("returns true when over budget", () => {
    const snap = makeSnapshot({ annualWageBill: 2_000_000 });
    const team = createTeam({ finance: 500_000, wage_budget: 1_000_000 });
    expect(sponsorPitchAvailable(mapLocalFinanceSnapshot(team, snap))).toBe(true);
  });

  it("returns true when wage budget is warning", () => {
    const snap = makeSnapshot({ wageBudgetStatus: "warning" });
    expect(sponsorPitchAvailable(mapLocalFinanceSnapshot(makeSolventTeam(), snap))).toBe(true);
  });

  it("returns true when wage budget is critical", () => {
    const snap = makeSnapshot({ wageBudgetStatus: "critical" });
    expect(sponsorPitchAvailable(mapLocalFinanceSnapshot(makeSolventTeam(), snap))).toBe(true);
  });
});

describe("marketingCampaignAvailable", () => {
  it("returns false when finances are healthy", () => {
    expect(marketingCampaignAvailable(mapLocalFinanceSnapshot(makeSolventTeam(), makeSnapshot()))).toBe(false);
  });

  it("returns true when runway is critical", () => {
    const snap = makeSnapshot({ runwayStatus: "critical" });
    expect(marketingCampaignAvailable(mapLocalFinanceSnapshot(makeSolventTeam(), snap))).toBe(true);
  });
});

describe("mapLocalFinanceSnapshot", () => {
  it("derives currentlyInDebt from team.finance < 0", () => {
    const inDebt = mapLocalFinanceSnapshot(createTeam({ finance: -1 }), makeSnapshot());
    expect(inDebt.currentlyInDebt).toBe(true);

    const solvent = mapLocalFinanceSnapshot(createTeam({ finance: 0 }), makeSnapshot());
    expect(solvent.currentlyInDebt).toBe(false);
  });

  it("derives currentlyOverBudget from annualWageBill > team.wage_budget", () => {
    const snap = makeSnapshot({ annualWageBill: 100_001 });
    const over = mapLocalFinanceSnapshot(createTeam({ wage_budget: 100_000 }), snap);
    expect(over.currentlyOverBudget).toBe(true);

    const under = mapLocalFinanceSnapshot(createTeam({ wage_budget: 200_000 }), snap);
    expect(under.currentlyOverBudget).toBe(false);
  });

  it("maps weeklySponsorIncome to both weeklyRecurringIncome and weeklySponsorIncome", () => {
    const snap = makeSnapshot({ weeklySponsorIncome: 7_500 });
    const result = mapLocalFinanceSnapshot(makeSolventTeam(), snap);
    expect(result.weeklyRecurringIncome).toBe(7_500);
    expect(result.weeklySponsorIncome).toBe(7_500);
  });
});

describe("isChooseOptionAction", () => {
  it("returns true for an object with ChooseOption key", () => {
    expect(isChooseOptionAction({ ChooseOption: { options: [] } })).toBe(true);
  });

  it("returns false for string action types", () => {
    expect(isChooseOptionAction("Dismiss" as never)).toBe(false);
    expect(isChooseOptionAction("Acknowledge" as never)).toBe(false);
  });

  it("returns false for objects without ChooseOption key", () => {
    expect(isChooseOptionAction({} as never)).toBe(false);
  });
});

describe("isPendingSponsorOffer", () => {
  function makeMessage(overrides: Partial<MessageData> = {}): MessageData {
    return {
      id: "sponsor_offer_abc",
      subject: "Sponsor offer",
      body: "Details",
      sender: "Finance",
      sender_role: "board",
      date: "2025-07-01",
      read: false,
      category: "Finance",
      priority: "normal",
      actions: [
        {
          id: "act-1",
          label: "Accept",
          action_type: { ChooseOption: { options: [{ id: "accept", label: "Accept", description: "" }] } },
          resolved: false,
        },
      ],
      context: { team_id: null, player_id: null, fixture_id: null, match_result: null },
      ...overrides,
    };
  }

  it("returns true for an unresolved sponsor offer message", () => {
    expect(isPendingSponsorOffer(makeMessage())).toBe(true);
  });

  it("returns false when message id does not start with sponsor_", () => {
    expect(isPendingSponsorOffer(makeMessage({ id: "other_offer_abc" }))).toBe(false);
  });

  it("returns false when category is not Finance", () => {
    expect(isPendingSponsorOffer(makeMessage({ category: "General" as never }))).toBe(false);
  });

  it("returns false when all actions are resolved", () => {
    const msg = makeMessage();
    msg.actions[0].resolved = true;
    expect(isPendingSponsorOffer(msg)).toBe(false);
  });

  it("returns false when action type is not ChooseOption", () => {
    const msg = makeMessage();
    msg.actions[0].action_type = "Acknowledge" as never;
    expect(isPendingSponsorOffer(msg)).toBe(false);
  });
});
