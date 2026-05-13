import type { PlayerData, StaffData, TeamData } from "../store/gameStore";

export type FinanceHealthLevel = "stable" | "watch" | "warning" | "critical";

export interface TeamFinanceSnapshot {
  annualWageBill: number;
  weeklyWageSpend: number;
  weeklyWageBudget: number;
  weeklySponsorIncome: number;
  projectedWeeklyNet: number;
  cashRunwayWeeks: number | null;
  wageBudgetUsagePercent: number;
  wageBudgetStatus: FinanceHealthLevel;
  runwayStatus: FinanceHealthLevel;
  overallStatus: FinanceHealthLevel;
  marketingCampaignCooldownDaysRemaining: number;
}

const MARKETING_CAMPAIGN_COOLDOWN_DAYS = 28;

const HEALTH_PRIORITY: Record<FinanceHealthLevel, number> = {
  stable: 0,
  watch: 1,
  warning: 2,
  critical: 3,
};

export function annualAmountToWeeklyCommitment(amount: number): number {
  return Math.floor(Math.max(0, amount) / 52);
}

export function getAnnualWageBill(
  players: PlayerData[],
  staff: StaffData[] = [],
): number {
  return [...players, ...staff].reduce((sum, person) => {
    return sum + Math.max(0, person.wage);
  }, 0);
}

export function getWeeklyWageSpend(
  players: PlayerData[],
  staff: StaffData[] = [],
): number {
  return [...players, ...staff].reduce((sum, person) => {
    return sum + annualAmountToWeeklyCommitment(person.wage);
  }, 0);
}

export function getCashRunwayWeeks(
  balance: number,
  projectedWeeklyNet: number,
): number | null {
  if (projectedWeeklyNet >= 0) {
    return null;
  }

  return Math.max(0, Math.floor(balance / Math.abs(projectedWeeklyNet)));
}

function getWageBudgetStatus(usagePercent: number): FinanceHealthLevel {
  if (usagePercent > 110) {
    return "critical";
  }

  if (usagePercent > 100) {
    return "warning";
  }

  if (usagePercent >= 85) {
    return "watch";
  }

  return "stable";
}

function getRunwayStatus(
  balance: number,
  runwayWeeks: number | null,
): FinanceHealthLevel {
  if (balance < 0) {
    return "critical";
  }

  if (runwayWeeks === null) {
    return "stable";
  }

  if (runwayWeeks <= 4) {
    return "critical";
  }

  if (runwayWeeks <= 8) {
    return "warning";
  }

  if (runwayWeeks <= 12) {
    return "watch";
  }

  return "stable";
}

function getMostSevereLevel(
  left: FinanceHealthLevel,
  right: FinanceHealthLevel,
): FinanceHealthLevel {
  return HEALTH_PRIORITY[left] >= HEALTH_PRIORITY[right] ? left : right;
}

function parseIsoDate(dateText: string | undefined): Date | null {
  if (!dateText) {
    return null;
  }

  const normalized = dateText.includes("T") ? dateText : `${dateText}T00:00:00Z`;
  const parsed = new Date(normalized);
  return Number.isNaN(parsed.getTime()) ? null : parsed;
}

function getMarketingCampaignCooldownDaysRemaining(
  team: TeamData,
  currentDate?: string,
): number {
  const today = parseIsoDate(currentDate);
  if (!today) {
    return 0;
  }

  const lastCampaign = (team.financial_ledger ?? [])
    .filter((entry) => entry.kind === "CommercialCampaign")
    .map((entry) => parseIsoDate(entry.date))
    .filter((entry): entry is Date => entry !== null)
    .sort((left, right) => right.getTime() - left.getTime())[0];

  if (!lastCampaign) {
    return 0;
  }

  const millisPerDay = 24 * 60 * 60 * 1000;
  const daysSince = Math.max(
    0,
    Math.floor((today.getTime() - lastCampaign.getTime()) / millisPerDay),
  );

  return Math.max(0, MARKETING_CAMPAIGN_COOLDOWN_DAYS - daysSince);
}

export function getTeamFinanceSnapshot(
  team: TeamData,
  players: PlayerData[],
  staff: StaffData[] = [],
  currentDate?: string,
): TeamFinanceSnapshot {
  const annualWageBill = getAnnualWageBill(players, staff);
  const weeklyWageSpend = getWeeklyWageSpend(players, staff);
  const weeklyWageBudget = annualAmountToWeeklyCommitment(team.wage_budget);
  const weeklySponsorIncome = team.sponsorship?.base_value ?? 0;
  const projectedWeeklyNet = weeklySponsorIncome - weeklyWageSpend;
  const cashRunwayWeeks = getCashRunwayWeeks(team.finance, projectedWeeklyNet);
  const wageBudgetUsagePercent = Math.round(
    (annualWageBill / Math.max(1, team.wage_budget)) * 100,
  );
  const wageBudgetStatus = getWageBudgetStatus(wageBudgetUsagePercent);
  const runwayStatus = getRunwayStatus(team.finance, cashRunwayWeeks);

  return {
    annualWageBill,
    weeklyWageSpend,
    weeklyWageBudget,
    weeklySponsorIncome,
    projectedWeeklyNet,
    cashRunwayWeeks,
    wageBudgetUsagePercent,
    wageBudgetStatus,
    runwayStatus,
    overallStatus: getMostSevereLevel(wageBudgetStatus, runwayStatus),
    marketingCampaignCooldownDaysRemaining:
      getMarketingCampaignCooldownDaysRemaining(team, currentDate),
  };
}