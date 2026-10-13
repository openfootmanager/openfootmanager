import { describe, expect, it } from "vitest";
import { formatVal } from "../../lib/helpers";
import type { TeamFinanceSnapshotData } from "../../services/financeService";
import type { MessageAction, MessageData } from "../../store/gameStore";
import {
  boardSupportAvailable,
  facilityUpgradeBlockReason,
  formatSignedAmount,
  getFacilityUpgradeCost,
  isChooseOptionAction,
  isPendingSponsorOffer,
  marketingCampaignAvailable,
  sponsorPitchAvailable,
} from "./FinancesTab.helpers";

/** A club in good financial health; each test disturbs only what it is about. */
function healthySnapshot(
  overrides: Partial<TeamFinanceSnapshotData> = {},
): TeamFinanceSnapshotData {
  return {
    annualWageBill: 1_000_000,
    weeklyWageSpend: 19_230,
    weeklyWageBudget: 30_000,
    weeklyRecurringIncome: 25_000,
    weeklySponsorIncome: 25_000,
    projectedWeeklyNet: 5_770,
    cashRunwayWeeks: null,
    wageBudgetUsagePercent: 64,
    currentlyInDebt: false,
    currentlyOverBudget: false,
    wageBudgetStatus: "stable",
    runwayStatus: "stable",
    overallStatus: "stable",
    marketingCampaignCooldownDaysRemaining: 0,
    ...overrides,
  };
}

describe("formatSignedAmount", () => {
  it("renders a negative amount with a leading minus", () => {
    expect(formatSignedAmount(-5_000_000).startsWith("-")).toBe(true);
    expect(formatSignedAmount(5_000_000).startsWith("-")).toBe(false);
  });

  it("defers sign placement to formatVal", () => {
    // Pins the reason the abs()-then-prepend-"-" version was dropped: formatVal
    // already renders the sign, so the two agree for every input.
    for (const value of [-5_000_000, -1_250, 0, 1_250, 5_000_000]) {
      expect(formatSignedAmount(value)).toBe(formatVal(value));
    }
  });
});

describe("getFacilityUpgradeCost", () => {
  it("scales linearly with the current level", () => {
    expect(getFacilityUpgradeCost(1)).toBe(250_000);
    expect(getFacilityUpgradeCost(4)).toBe(1_000_000);
  });
});

describe("facilityUpgradeBlockReason", () => {
  it("lets a healthy club spend", () => {
    expect(facilityUpgradeBlockReason(healthySnapshot())).toBeNull();
  });

  it("blocks a club already over its wage budget, and says which problem it is", () => {
    // Over-budget outranks distress: a club can be both, and the over-budget
    // message is the actionable one.
    expect(
      facilityUpgradeBlockReason(
        healthySnapshot({ currentlyOverBudget: true, overallStatus: "critical" }),
      ),
    ).toBe("be.error.finance.facilityUpgradeOverBudget");
  });

  it("blocks a club in financial distress", () => {
    for (const overallStatus of ["warning", "critical"] as const) {
      expect(facilityUpgradeBlockReason(healthySnapshot({ overallStatus }))).toBe(
        "be.error.finance.facilityUpgradeCritical",
      );
    }
  });

  it("treats a watch-level warning as still spendable", () => {
    expect(facilityUpgradeBlockReason(healthySnapshot({ overallStatus: "watch" }))).toBeNull();
  });
});

describe("board support and commercial actions", () => {
  it("keeps all three away from a healthy club", () => {
    const healthy = healthySnapshot();
    expect(boardSupportAvailable(healthy)).toBe(false);
    expect(sponsorPitchAvailable(healthy)).toBe(false);
    expect(marketingCampaignAvailable(healthy)).toBe(false);
  });

  it("offers board support on debt or a runway problem only", () => {
    expect(boardSupportAvailable(healthySnapshot({ currentlyInDebt: true }))).toBe(true);
    expect(boardSupportAvailable(healthySnapshot({ runwayStatus: "warning" }))).toBe(true);
    expect(boardSupportAvailable(healthySnapshot({ runwayStatus: "critical" }))).toBe(true);
    // Wage pressure alone is the commercial team's problem, not the board's.
    expect(boardSupportAvailable(healthySnapshot({ wageBudgetStatus: "critical" }))).toBe(false);
    expect(boardSupportAvailable(healthySnapshot({ currentlyOverBudget: true }))).toBe(false);
  });

  it("offers a sponsor pitch on wage pressure as well as cash pressure", () => {
    const triggers: Array<Partial<TeamFinanceSnapshotData>> = [
      { currentlyOverBudget: true },
      { currentlyInDebt: true },
      { wageBudgetStatus: "warning" },
      { wageBudgetStatus: "critical" },
      { runwayStatus: "warning" },
      { runwayStatus: "critical" },
    ];
    for (const trigger of triggers) {
      expect(sponsorPitchAvailable(healthySnapshot(trigger))).toBe(true);
    }
  });

  it("gates the marketing campaign exactly as the sponsor pitch", () => {
    for (const trigger of [{ currentlyInDebt: true }, { wageBudgetStatus: "warning" as const }]) {
      const snapshot = healthySnapshot(trigger);
      expect(marketingCampaignAvailable(snapshot)).toBe(sponsorPitchAvailable(snapshot));
    }
  });
});

describe("isChooseOptionAction", () => {
  it("recognises a choice action", () => {
    const actionType: MessageAction["action_type"] = {
      ChooseOption: { options: [{ id: "accept", label: "Accept", description: "" }] },
    };

    expect(isChooseOptionAction(actionType)).toBe(true);
  });

  it("rejects a string action type", () => {
    expect(isChooseOptionAction("Acknowledge" as MessageAction["action_type"])).toBe(false);
  });
});

describe("isPendingSponsorOffer", () => {
  function sponsorMessage(overrides: Partial<MessageData> = {}): MessageData {
    return {
      id: "sponsor_offer_1",
      category: "Finance",
      actions: [
        {
          resolved: false,
          action_type: {
            ChooseOption: { options: [{ id: "accept", label: "Accept", description: "" }] },
          },
        },
      ],
      ...overrides,
    } as MessageData;
  }

  it("accepts an unresolved sponsor choice", () => {
    expect(isPendingSponsorOffer(sponsorMessage())).toBe(true);
  });

  it("rejects an offer that has already been answered", () => {
    const answered = sponsorMessage();
    answered.actions[0].resolved = true;

    expect(isPendingSponsorOffer(answered)).toBe(false);
  });

  it("rejects a finance message that is not a sponsor offer", () => {
    expect(isPendingSponsorOffer(sponsorMessage({ id: "board_warning_1" }))).toBe(false);
  });

  it("rejects a sponsor-shaped message filed under another category", () => {
    expect(isPendingSponsorOffer(sponsorMessage({ category: "Transfer" }))).toBe(false);
  });
});
