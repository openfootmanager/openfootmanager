import type { NegotiationFeedbackPanelData } from "../NegotiationFeedbackPanel";

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

export type NegotiationFeedbackData = NegotiationFeedbackPanelData;

export interface RenewalResponseData {
  outcome: "accepted" | "rejected" | "counter_offer";
  game: import("../../store/gameStore").GameStateData;
  suggested_wage: number | null;
  suggested_years: number | null;
  session_status: "idle" | "open" | "agreed" | "blocked" | "stalled";
  is_terminal: boolean;
  cooled_off?: boolean;
  feedback?: NegotiationFeedbackData | null;
}

export interface RenewalProjectionData {
  projection: RenewalProjection;
}

export interface DelegatedRenewalCaseData {
  player_id: string;
  status: "successful" | "failed" | "stalled";
  note: string;
  note_key?: string;
  note_params?: Record<string, string>;
}

export interface DelegatedRenewalResponseData {
  game: import("../../store/gameStore").GameStateData;
  report: {
    success_count: number;
    failure_count: number;
    stalled_count: number;
    cases: DelegatedRenewalCaseData[];
  };
}

export type RenewalStatus =
  | "idle"
  | "accepted"
  | "rejected"
  | "counter_offer"
  | "blocked"
  | "error";

type TranslateFn = (
  key: string,
  options?: Record<string, string | number>,
) => string;

interface RenewalStatusMessageContext {
  renewalSessionStatus: RenewalResponseData["session_status"];
  renewalStatus: RenewalStatus;
  renewalSuggestedWage: number | null;
  renewalSuggestedYears: number | null;
  renewalError: string | null;
}

interface RenewalSubmitState {
  renewalSubmitting: boolean;
  renewalIsTerminal: boolean;
  isRenewalWageValid: boolean;
  isRenewalLengthValid: boolean;
  renewalViolatesSoftCap: boolean;
}

export function getRenewalStatusMessage(
  context: RenewalStatusMessageContext,
  translate: TranslateFn,
): string | null {
  if (
    context.renewalSessionStatus === "blocked" ||
    context.renewalStatus === "blocked"
  ) {
    return translate("playerProfile.renewalBlocked");
  }

  if (context.renewalStatus === "accepted") {
    return translate("playerProfile.renewalAccepted");
  }

  if (context.renewalStatus === "rejected") {
    return translate("playerProfile.renewalRejected");
  }

  if (
    context.renewalStatus === "counter_offer" &&
    context.renewalSuggestedWage !== null &&
    context.renewalSuggestedYears !== null
  ) {
    return translate("playerProfile.renewalCounter", {
      wage: context.renewalSuggestedWage,
      years: context.renewalSuggestedYears,
    });
  }

  return context.renewalError;
}

export function getRenewalStatusClassName(renewalStatus: RenewalStatus): string {
  if (renewalStatus === "accepted") {
    return "text-primary-500";
  }

  if (renewalStatus === "rejected" || renewalStatus === "error") {
    return "text-red-500";
  }

  if (renewalStatus === "counter_offer") {
    return "text-accent-600 dark:text-accent-400";
  }

  return "text-gray-500 dark:text-gray-400";
}

export function shouldDisableRenewalSubmit(state: RenewalSubmitState): boolean {
  return (
    state.renewalSubmitting ||
    state.renewalIsTerminal ||
    !state.isRenewalWageValid ||
    !state.isRenewalLengthValid ||
    state.renewalViolatesSoftCap
  );
}