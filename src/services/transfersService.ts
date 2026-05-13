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
  };
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