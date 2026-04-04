import { invokeCommand } from "./tauriClient";
import type { GameStateData } from "../store/gameStore";

export interface RenewalProjection {
    current_annual_wage_bill: number;
    projected_annual_wage_bill: number;
    annual_wage_budget: number;
    annual_soft_cap: number;
    current_weekly_wage_spend: number;
    projected_weekly_wage_spend: number;
    current_cash_runway_weeks: number | null;
    projected_cash_runway_weeks: number | null;
    currently_over_budget: boolean;
    policy_allows: boolean;
}

export interface RenewalProjectionData {
    projection: RenewalProjection;
}

export interface NegotiationFeedbackData {
    mood: string;
    headline_key?: string | null;
    detail_key?: string | null;
    tension: number;
    patience: number;
    round: number;
    params?: Record<string, string>;
}

export interface RenewalResponseData {
    outcome: "accepted" | "rejected" | "counter_offer";
    game: GameStateData;
    suggested_wage: number | null;
    suggested_years: number | null;
    session_status: "idle" | "open" | "agreed" | "blocked" | "stalled";
    is_terminal: boolean;
    cooled_off?: boolean;
    feedback?: NegotiationFeedbackData | null;
}

export interface DelegatedRenewalCaseData {
    player_id: string;
    status: "successful" | "failed" | "stalled";
    note: string;
    note_key?: string;
    note_params?: Record<string, string>;
}

export interface DelegatedRenewalResponseData {
    game: GameStateData;
    report: {
        success_count: number;
        failure_count: number;
        stalled_count: number;
        cases: DelegatedRenewalCaseData[];
    };
}

export interface DelegateRenewalsInput {
    playerIds: string[];
    maxWageIncreasePct: number;
    maxContractYears: number;
}

export async function previewRenewalFinancialImpact(
    playerId: string,
    weeklyWage: number,
): Promise<RenewalProjectionData> {
    return invokeCommand<RenewalProjectionData>(
        "preview_renewal_financial_impact",
        {
            playerId,
            weeklyWage,
        },
    );
}

export async function proposeRenewal(
    playerId: string,
    weeklyWage: number,
    contractYears: number,
): Promise<RenewalResponseData> {
    return invokeCommand<RenewalResponseData>("propose_renewal", {
        playerId,
        weeklyWage,
        contractYears,
    });
}

export async function delegateRenewals(
    input: DelegateRenewalsInput,
): Promise<DelegatedRenewalResponseData> {
    return invokeCommand<DelegatedRenewalResponseData>("delegate_renewals", input);
}