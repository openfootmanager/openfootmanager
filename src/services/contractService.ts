import { invoke } from "@tauri-apps/api/core";

import type { GameStateData } from "../store/gameStore";
import { invokeForCareer } from "./careerInvoke";

export type SquadSafetyIssue =
  | "too_few_healthy_players"
  | "no_healthy_goalkeeper"
  | "incomplete_formation";

/** How near a contract's end is, as the daily warnings, morale pressure and renewals read it. */
export type ContractWarningStage = "twelve_months" | "six_months" | "three_months" | "final_weeks";

/** The warning stage of each player's contract; null beyond a year or once over. */
export async function getContractWarningStages(
  playerIds: string[],
): Promise<Record<string, ContractWarningStage | null>> {
  return invoke<Record<string, ContractWarningStage | null>>("get_contract_warning_stages", {
    playerIds,
  });
}

export interface SquadSafetyReportData {
  team_id: string;
  projected_roster_size: number;
  healthy_players: number;
  healthy_goalkeepers: number;
  effective_xi_size: number;
  can_field_matchday_squad: boolean;
  missing_reasons: SquadSafetyIssue[];
}

export interface ContractTerminationPreviewData {
  player_id: string;
  player_name: string;
  severance_cost: number;
  squad_safety: SquadSafetyReportData;
  /**
   * Why the squad floor would refuse the release, as the translation-key error the backend would
   * return (`resolveBackendError` turns it into words); `null` when the club keeps the floor.
   */
  squad_floor_refusal: string | null;
}

/**
 * What a contract does to the club's books. The "annual" names carry WEEKLY amounts (the backend
 * sets the bill equal to the weekly spend and the budget is on the same scale); they are not renamed
 * here (#300, money unit conventions), and the screens label them weekly.
 */
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

export interface RenewalProjectionResponseData {
  projection: RenewalProjection;
}

export interface ContractExitIntentResponseData {
  game: GameStateData;
}

export interface ContractTerminationPreviewResponseData {
  preview: ContractTerminationPreviewData;
}

export interface ContractTerminationResponseData {
  game: GameStateData;
  severance_cost: number;
  squad_safety: SquadSafetyReportData;
}

export async function setContractExitIntent(
  playerId: string,
  reason?: string,
): Promise<ContractExitIntentResponseData> {
  return invokeForCareer<ContractExitIntentResponseData>("set_contract_exit_intent", {
    playerId,
    reason: reason ?? null,
  });
}

export async function clearContractExitIntent(
  playerId: string,
): Promise<ContractExitIntentResponseData> {
  return invokeForCareer<ContractExitIntentResponseData>("clear_contract_exit_intent", {
    playerId,
  });
}

export async function previewContractTermination(
  playerId: string,
): Promise<ContractTerminationPreviewResponseData> {
  return invoke<ContractTerminationPreviewResponseData>("preview_contract_termination", {
    playerId,
  });
}

export async function terminateContractNow(
  playerId: string,
): Promise<ContractTerminationResponseData> {
  return invokeForCareer<ContractTerminationResponseData>("terminate_contract_now", {
    playerId,
  });
}

/** The club's wage bill, budget and cash runway with `weeklyWage` in place of his own: his own player's only. */
export async function previewRenewalProjection(
  playerId: string,
  weeklyWage: number,
): Promise<RenewalProjectionResponseData> {
  return invoke<RenewalProjectionResponseData>("preview_renewal_financial_impact", {
    playerId,
    weeklyWage,
  });
}
