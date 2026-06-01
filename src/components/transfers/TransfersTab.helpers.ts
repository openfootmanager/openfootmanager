import type { PlayerData, TransferOfferData } from "../../store/gameStore";
import type { TransferNegotiationFeedbackData } from "../../services/transfersService";
import { formatExactMoney } from "../../lib/helpers";

type Translate = (
  key: string,
  options?: Record<string, string | number>,
) => string;

export function formatTransferFeeInput(fee: number | null | undefined): string {
  if (fee === null || fee === undefined || !Number.isFinite(fee)) {
    return "";
  }

  return String(Math.round(fee));
}

export function parseTransferFeeInput(value: string): number | null {
  const digits = value.replace(/\D/g, "");

  if (!digits) {
    return null;
  }

  const parsed = Number.parseInt(digits, 10);
  return Number.isFinite(parsed) ? parsed : null;
}

export function normalizeTransferNegotiationFeedback(
  feedback: TransferNegotiationFeedbackData | null,
): TransferNegotiationFeedbackData | null {
  if (!feedback?.params?.fee) {
    return feedback;
  }

  const parsedFee = Number.parseInt(feedback.params.fee, 10);
  if (!Number.isFinite(parsedFee)) {
    return feedback;
  }

  return {
    ...feedback,
    params: {
      ...feedback.params,
      fee: formatExactMoney(parsedFee),
    },
  };
}

export function getOutgoingNegotiationOffer(
  player: PlayerData,
  userTeamId: string | null,
): TransferOfferData | null {
  if (!userTeamId) {
    return null;
  }

  return (
    player.transfer_offers.find(
      (offer) =>
        offer.from_team_id === userTeamId && offer.status === "Pending",
    ) ?? null
  );
}

export function buildResumedBidFeedback(
  offer: TransferOfferData | null,
): TransferNegotiationFeedbackData | null {
  if (!offer) {
    return null;
  }

  const round = Math.max(offer.negotiation_round || 1, 1);
  const tension = Math.min(36 + (round - 1) * 16, 84);
  const patience = Math.max(82 - (round - 1) * 16, 30);

  return normalizeTransferNegotiationFeedback({
    mood: round >= 3 ? "tense" : "firm",
    headline_key: "transfers.resumeNegotiationHeadline",
    detail_key: "transfers.resumeNegotiationDetail",
    tension,
    patience,
    round,
    params: {
      fee: String(offer.suggested_counter_fee ?? offer.fee),
    },
  });
}

export function buildResumedCounterFeedback(
  offer: TransferOfferData | null,
): TransferNegotiationFeedbackData | null {
  if (!offer) {
    return null;
  }

  const round = Math.max(offer.negotiation_round || 1, 1);
  const tension = Math.min(40 + (round - 1) * 14, 86);
  const patience = Math.max(78 - (round - 1) * 14, 28);

  return normalizeTransferNegotiationFeedback({
    mood: round >= 3 ? "tense" : "firm",
    headline_key: "transfers.resumeNegotiationHeadline",
    detail_key: "transfers.resumeNegotiationDetail",
    tension,
    patience,
    round,
    params: {
      fee: String(offer.suggested_counter_fee ?? offer.fee),
    },
  });
}

export function getTransferOfferStatusLabel(
  t: Translate,
  status: TransferOfferData["status"],
): string {
  switch (status) {
    case "Pending":
      return t("transfers.offerStatusPending");
    case "Accepted":
      return t("transfers.offerStatusAccepted");
    case "Rejected":
      return t("transfers.offerStatusRejected");
    case "Withdrawn":
      return t("transfers.offerStatusWithdrawn");
    default:
      return status;
  }
}

export function getTransferOfferBadgeVariant(
  status: TransferOfferData["status"],
) {
  switch (status) {
    case "Pending":
      return "accent" as const;
    case "Accepted":
      return "success" as const;
    case "Withdrawn":
      return "neutral" as const;
    case "Rejected":
    default:
      return "danger" as const;
  }
}

export function mapTransferNegotiationError(
  t: Translate,
  error: string,
): string {
  if (error.includes("Offer not found or not pending")) {
    return t("transfers.negotiationExpiredError");
  }

  return error;
}