import { invoke } from "@tauri-apps/api/core";

import type { GameStateData } from "../store/gameStore";

export interface TransferNegotiationFeedbackData {
  mood: "calm" | "firm" | "tense" | "positive" | "guarded";
  headline_key: string;
  detail_key?: string | null;
  tension: number;
  patience: number;
  round: number;
  params?: Record<string, string>;
}

export interface TransferNegotiationResponseData {
  decision: "accepted" | "rejected" | "counter_offer";
  suggested_fee: number | null;
  is_terminal: boolean;
  registration_date?: string | null;
  feedback: TransferNegotiationFeedbackData;
  game: GameStateData;
}

export interface TransferBidProjectionData {
  projection: {
    transfer_budget_before: number;
    transfer_budget_after: number;
    finance_before: number;
    finance_after: number;
    annual_wage_bill_before: number;
    annual_wage_bill_after: number;
    annual_wage_budget: number;
    projected_wage_budget_usage_pct: number;
    exceeds_transfer_budget: boolean;
    exceeds_finance: boolean;
    /** Debit fires on this date (window closed → PendingRegistration). Absent when the deal would execute immediately. */
    pending_registration_date?: string | null;
  };
}

export interface LoanOfferResponseData {
  decision: "accepted" | "rejected" | "counter_offer";
  offer_id: string;
  suggested_wage_contribution_pct: number | null;
  suggested_end_date: string | null;
  suggested_buy_option_fee: number | null;
  is_terminal: boolean;
  game: GameStateData;
}

export async function makeTransferBid(
  playerId: string,
  fee: number,
): Promise<TransferNegotiationResponseData> {
  return invoke<TransferNegotiationResponseData>("make_transfer_bid", {
    playerId,
    fee,
  });
}

export async function makeLoanOffer(
  playerId: string,
  endDate: string,
  wageContributionPct: number,
  buyOptionFee: number | null = null,
): Promise<LoanOfferResponseData> {
  return invoke<LoanOfferResponseData>("make_loan_offer", {
    playerId,
    endDate,
    wageContributionPct,
    buyOptionFee,
  });
}

export async function exerciseLoanBuyOption(
  playerId: string,
): Promise<GameStateData> {
  return invoke<GameStateData>("exercise_loan_buy_option", {
    playerId,
  });
}

export async function respondToOffer(
  playerId: string,
  offerId: string,
  accept: boolean,
): Promise<GameStateData> {
  return invoke<GameStateData>("respond_to_offer", {
    playerId,
    offerId,
    accept,
  });
}

export async function respondToLoanOffer(
  playerId: string,
  offerId: string,
  accept: boolean,
): Promise<GameStateData> {
  return invoke<GameStateData>("respond_to_loan_offer", {
    playerId,
    offerId,
    accept,
  });
}

export async function counterLoanOffer(
  playerId: string,
  offerId: string,
  endDate: string,
  wageContributionPct: number,
  buyOptionFee: number | null = null,
): Promise<LoanOfferResponseData> {
  return invoke<LoanOfferResponseData>("counter_loan_offer", {
    playerId,
    offerId,
    endDate,
    wageContributionPct,
    buyOptionFee,
  });
}

export async function counterOffer(
  playerId: string,
  offerId: string,
  requestedFee: number,
): Promise<TransferNegotiationResponseData> {
  return invoke<TransferNegotiationResponseData>("counter_offer", {
    playerId,
    offerId,
    requestedFee,
  });
}

export async function previewTransferBidFinancialImpact(
  playerId: string,
  fee: number,
): Promise<TransferBidProjectionData> {
  return invoke<TransferBidProjectionData>(
    "preview_transfer_bid_financial_impact",
    {
      playerId,
      fee,
    },
  );
}

export async function toggleTransferList(
  playerId: string,
): Promise<GameStateData> {
  return invoke<GameStateData>("toggle_transfer_list", {
    playerId,
  });
}

export async function toggleLoanList(
  playerId: string,
): Promise<GameStateData> {
  return invoke<GameStateData>("toggle_loan_list", {
    playerId,
  });
}
