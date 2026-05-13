import { invoke } from "@tauri-apps/api/core";

export type FinanceHealthLevelData =
    | "stable"
    | "watch"
    | "warning"
    | "critical";

interface BackendTeamFinanceSnapshotData {
    annual_wage_bill: number;
    weekly_wage_spend: number;
    weekly_wage_budget: number;
    weekly_recurring_income: number;
    weekly_sponsor_income: number;
    projected_weekly_net: number;
    cash_runway_weeks: number | null;
    wage_budget_usage_percent: number;
    currently_in_debt: boolean;
    currently_over_budget: boolean;
    wage_budget_status: FinanceHealthLevelData;
    runway_status: FinanceHealthLevelData;
    overall_status: FinanceHealthLevelData;
    marketing_campaign_cooldown_days_remaining: number;
}

interface BackendBoardSupportPreviewData {
    support_amount: number;
    transfer_budget_reduction: number;
    satisfaction_penalty: number;
}

interface BackendSponsorPitchPreviewData {
    sponsor_name: string;
    weekly_amount: number;
    duration_weeks: number;
}

interface BackendMarketingCampaignPreviewData {
    gross_revenue: number;
    campaign_cost: number;
    net_income: number;
    cooldown_days: number;
}

interface BackendFinanceActionPreviewsData {
    board_support?: BackendBoardSupportPreviewData | null;
    sponsor_pitch?: BackendSponsorPitchPreviewData | null;
    marketing_campaign?: BackendMarketingCampaignPreviewData | null;
}

interface BackendFinanceSnapshotResponseData {
    snapshot: BackendTeamFinanceSnapshotData;
    previews?: BackendFinanceActionPreviewsData | null;
}

export interface TeamFinanceSnapshotData {
    annualWageBill: number;
    weeklyWageSpend: number;
    weeklyWageBudget: number;
    weeklyRecurringIncome: number;
    weeklySponsorIncome: number;
    projectedWeeklyNet: number;
    cashRunwayWeeks: number | null;
    wageBudgetUsagePercent: number;
    currentlyInDebt: boolean;
    currentlyOverBudget: boolean;
    wageBudgetStatus: FinanceHealthLevelData;
    runwayStatus: FinanceHealthLevelData;
    overallStatus: FinanceHealthLevelData;
    marketingCampaignCooldownDaysRemaining: number;
}

export interface BoardSupportPreviewData {
    supportAmount: number;
    transferBudgetReduction: number;
    satisfactionPenalty: number;
}

export interface SponsorPitchPreviewData {
    sponsorName: string;
    weeklyAmount: number;
    durationWeeks: number;
}

export interface MarketingCampaignPreviewData {
    grossRevenue: number;
    campaignCost: number;
    netIncome: number;
    cooldownDays: number;
}

export interface FinanceRecoveryPreviewsData {
    boardSupport: BoardSupportPreviewData | null;
    sponsorPitch: SponsorPitchPreviewData | null;
    marketingCampaign: MarketingCampaignPreviewData | null;
}

export interface FinanceSnapshotData {
    snapshot: TeamFinanceSnapshotData;
    previews: FinanceRecoveryPreviewsData;
}

function mapSnapshot(
    snapshot: BackendTeamFinanceSnapshotData,
): TeamFinanceSnapshotData {
    return {
        annualWageBill: snapshot.annual_wage_bill,
        weeklyWageSpend: snapshot.weekly_wage_spend,
        weeklyWageBudget: snapshot.weekly_wage_budget,
        weeklyRecurringIncome: snapshot.weekly_recurring_income,
        weeklySponsorIncome: snapshot.weekly_sponsor_income,
        projectedWeeklyNet: snapshot.projected_weekly_net,
        cashRunwayWeeks: snapshot.cash_runway_weeks,
        wageBudgetUsagePercent: snapshot.wage_budget_usage_percent,
        currentlyInDebt: snapshot.currently_in_debt,
        currentlyOverBudget: snapshot.currently_over_budget,
        wageBudgetStatus: snapshot.wage_budget_status,
        runwayStatus: snapshot.runway_status,
        overallStatus: snapshot.overall_status,
        marketingCampaignCooldownDaysRemaining:
            snapshot.marketing_campaign_cooldown_days_remaining,
    };
}

function mapPreviews(
    previews?: BackendFinanceActionPreviewsData | null,
): FinanceRecoveryPreviewsData {
    return {
        boardSupport: previews?.board_support
            ? {
                supportAmount: previews.board_support.support_amount,
                transferBudgetReduction:
                    previews.board_support.transfer_budget_reduction,
                satisfactionPenalty:
                    previews.board_support.satisfaction_penalty,
            }
            : null,
        sponsorPitch: previews?.sponsor_pitch
            ? {
                sponsorName: previews.sponsor_pitch.sponsor_name,
                weeklyAmount: previews.sponsor_pitch.weekly_amount,
                durationWeeks: previews.sponsor_pitch.duration_weeks,
            }
            : null,
        marketingCampaign: previews?.marketing_campaign
            ? {
                grossRevenue: previews.marketing_campaign.gross_revenue,
                campaignCost: previews.marketing_campaign.campaign_cost,
                netIncome: previews.marketing_campaign.net_income,
                cooldownDays: previews.marketing_campaign.cooldown_days,
            }
            : null,
    };
}

export async function getFinanceSnapshot(
    teamId?: string,
): Promise<FinanceSnapshotData> {
    const response = await invoke<BackendFinanceSnapshotResponseData>(
        "get_finance_snapshot",
        {
            teamId: teamId ?? null,
        },
    );

    return {
        snapshot: mapSnapshot(response.snapshot),
        previews: mapPreviews(response.previews),
    };
}