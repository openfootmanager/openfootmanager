import { useState } from "react";
import type { GameStateData, PlayerData, TransferOfferData } from "../../store/gameStore";
import type { NegotiationFeedbackPanelData } from "../NegotiationFeedbackPanel";
import {
  counterOffer,
  type TransferNegotiationResponseData,
} from "../../services/transfersService";
import {
  buildResumedCounterFeedback,
  formatTransferFeeInput,
  mapTransferNegotiationError,
  normalizeTransferNegotiationFeedback,
  parseTransferFeeInput,
} from "./TransfersTab.helpers";
import type { TFunction } from "i18next";
import type { CounterTarget } from "./TransfersTab.types";
interface Input {
  onGameUpdate: ((game: GameStateData) => void) | undefined;
  t: TFunction<"translation", undefined>;
}
export function useTransferCounterFlow({ onGameUpdate, t }: Input) {
  const [counterTarget, setCounterTarget] = useState<CounterTarget | null>(null);
  const [counterAmount, setCounterAmount] = useState("");
  const [counterLoading, setCounterLoading] = useState(false);
  const [counterError, setCounterError] = useState<string | null>(null);
  const [counterResult, setCounterResult] = useState<
    TransferNegotiationResponseData["decision"] | "error" | null
  >(null);
  const [counterFeedback, setCounterFeedback] = useState<NegotiationFeedbackPanelData | null>(null);
  const openCounterNegotiation = (player: PlayerData, offer: TransferOfferData) => {
    setCounterTarget({
      player,
      offerId: offer.id,
      fromTeamId: offer.from_team_id,
      fee: offer.fee,
    });
    setCounterAmount(formatTransferFeeInput(offer.suggested_counter_fee ?? offer.fee));
    setCounterError(null);
    setCounterResult(null);
    setCounterFeedback(buildResumedCounterFeedback(offer));
  };
  const handleCounterOffer = async () => {
    const requestedFee = parseTransferFeeInput(counterAmount);

    if (!counterTarget || requestedFee === null || requestedFee <= 0) return;

    setCounterLoading(true);
    setCounterError(null);
    setCounterResult(null);
    setCounterFeedback(null);

    try {
      const response = await counterOffer(
        counterTarget.player.id,
        counterTarget.offerId,
        requestedFee,
      );

      if (onGameUpdate) onGameUpdate(response.game);
      setCounterResult(response.decision);
      setCounterFeedback(normalizeTransferNegotiationFeedback(response.feedback));
      if (response.suggested_fee !== null) {
        setCounterAmount(formatTransferFeeInput(response.suggested_fee));
      }
    } catch (err: unknown) {
      setCounterError(mapTransferNegotiationError(t, String(err) || "error"));
    } finally {
      setCounterLoading(false);
    }
  };
  const activeCounterOffer = counterTarget
    ? (counterTarget.player.transfer_offers.find((offer) => offer.id === counterTarget.offerId) ??
      null)
    : null;
  return {
    counterTarget,
    setCounterTarget,
    counterAmount,
    setCounterAmount,
    counterLoading,
    counterError,
    setCounterError,
    counterResult,
    setCounterResult,
    counterFeedback,
    setCounterFeedback,
    openCounterNegotiation,
    handleCounterOffer,
    activeCounterOffer,
  };
}
