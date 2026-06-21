import type { GameStateData, MessageAction, MessageData } from "../../store/gameStore";
import { formatVal } from "../../lib/helpers";
import type { TeamFinanceSnapshot } from "../../lib/finance";
import type { FinanceSnapshotData, TeamFinanceSnapshotData } from "../../services/financeService";

export type FacilityId = "Training" | "Medical" | "Scouting";

export interface FacilityUpgradeErrorState {
  facilityId: FacilityId;
  message: string;
}

export interface FacilityDefinition {
  effectKey: string;
  id: FacilityId;
  levelKey: "training" | "medical" | "scouting";
  titleKey: string;
}

export const DEFAULT_FACILITIES = {
  training: 1,
  medical: 1,
  scouting: 1,
};

export const FACILITY_DEFINITIONS: FacilityDefinition[] = [
  {
    id: "Training",
    levelKey: "training",
    titleKey: "finances.facilityTraining",
    effectKey: "finances.facilityTrainingEffect",
  },
  {
    id: "Medical",
    levelKey: "medical",
    titleKey: "finances.facilityMedical",
    effectKey: "finances.facilityMedicalEffect",
  },
  {
    id: "Scouting",
    levelKey: "scouting",
    titleKey: "finances.facilityScouting",
    effectKey: "finances.facilityScoutingEffect",
  },
];

export function getFacilityUpgradeCost(level: number): number {
  return level * 250_000;
}

export function formatSignedAmount(value: number): string {
  const formatted = formatVal(Math.abs(value));
  return value < 0 ? `-${formatted}` : formatted;
}

export function facilityUpgradeBlockReason(
  snapshot: TeamFinanceSnapshotData,
): string | null {
  if (snapshot.currentlyOverBudget) {
    return "be.error.finance.facilityUpgradeOverBudget";
  }

  if (
    snapshot.overallStatus === "warning" ||
    snapshot.overallStatus === "critical"
  ) {
    return "be.error.finance.facilityUpgradeCritical";
  }

  return null;
}

export function boardSupportAvailable(snapshot: TeamFinanceSnapshotData): boolean {
  return (
    snapshot.currentlyInDebt ||
    snapshot.runwayStatus === "warning" ||
    snapshot.runwayStatus === "critical"
  );
}

export function sponsorPitchAvailable(snapshot: TeamFinanceSnapshotData): boolean {
  return (
    snapshot.currentlyOverBudget ||
    snapshot.currentlyInDebt ||
    snapshot.wageBudgetStatus === "warning" ||
    snapshot.wageBudgetStatus === "critical" ||
    snapshot.runwayStatus === "warning" ||
    snapshot.runwayStatus === "critical"
  );
}

export function marketingCampaignAvailable(snapshot: TeamFinanceSnapshotData): boolean {
  return sponsorPitchAvailable(snapshot);
}

export function mapLocalFinanceSnapshot(
  team: GameStateData["teams"][number],
  snapshot: TeamFinanceSnapshot,
): TeamFinanceSnapshotData {
  return {
    annualWageBill: snapshot.annualWageBill,
    weeklyWageSpend: snapshot.weeklyWageSpend,
    weeklyWageBudget: snapshot.weeklyWageBudget,
    weeklyRecurringIncome: snapshot.weeklySponsorIncome,
    weeklySponsorIncome: snapshot.weeklySponsorIncome,
    projectedWeeklyNet: snapshot.projectedWeeklyNet,
    cashRunwayWeeks: snapshot.cashRunwayWeeks,
    wageBudgetUsagePercent: snapshot.wageBudgetUsagePercent,
    currentlyInDebt: team.finance < 0,
    currentlyOverBudget: snapshot.annualWageBill > team.wage_budget,
    wageBudgetStatus: snapshot.wageBudgetStatus,
    runwayStatus: snapshot.runwayStatus,
    overallStatus: snapshot.overallStatus,
    marketingCampaignCooldownDaysRemaining:
      snapshot.marketingCampaignCooldownDaysRemaining,
  };
}

export interface ResolveMessageActionResult {
  game: GameStateData;
  effect: string | null;
  effect_i18n_key?: string | null;
  effect_i18n_params?: Record<string, string> | null;
}

export interface DelegatedRenewalResponseData {
  game: GameStateData;
  report: {
    success_count: number;
    failure_count: number;
    stalled_count: number;
  };
}

export interface TaggedFinanceSnapshotData {
  key: string;
  data: FinanceSnapshotData;
}

export interface BoardSupportResponseData {
  game: GameStateData;
  result: {
    support_amount: number;
    transfer_budget_reduction: number;
    satisfaction_penalty: number;
  };
}

export interface SponsorPitchResponseData {
  game: GameStateData;
  result: {
    message_id: string;
    sponsor_name: string;
    weekly_amount: number;
    duration_weeks: number;
  };
}

export interface MarketingCampaignResponseData {
  game: GameStateData;
  result: {
    message_id: string;
    gross_revenue: number;
    campaign_cost: number;
    net_income: number;
    cooldown_days: number;
  };
}

export function isChooseOptionAction(
  actionType: MessageAction["action_type"],
): actionType is {
  ChooseOption: {
    options: Array<{ id: string; label: string; description: string }>;
  };
} {
  return typeof actionType === "object" && "ChooseOption" in actionType;
}

export function isPendingSponsorOffer(message: MessageData): boolean {
  return (
    message.id.startsWith("sponsor_") &&
    message.category === "Finance" &&
    message.actions.some(
      (action) => !action.resolved && isChooseOptionAction(action.action_type),
    )
  );
}
