import { useState } from "react";
import { useTranslation } from "react-i18next";

import type {
  GameStateData,
  PlayerData,
  TransferOfferData,
} from "../../store/gameStore";
import {
  counterOffer,
  type TransferNegotiationResponseData,
} from "../../services/transfersService";
import { type NegotiationFeedbackPanelData } from "../NegotiationFeedbackPanel";
import {
  buildResumedCounterFeedback,
  formatTransferFeeInput,
  mapTransferNegotiationError,
  normalizeTransferNegotiationFeedback,
  parseTransferFeeInput,
} from "./TransfersTab.helpers";

interface CounterTarget {
  player: PlayerData;
  offerId: string;
  fromTeamId: string;
  fee: number;
}

interface UseTransferCounterOfferFlowArgs {
  onGameUpdate?: (game: GameStateData) => void;
}

interface UseTransferCounterOfferFlowResult {
  counterTarget: CounterTarget | null;
  counterAmount: string;
  setCounterAmount: (value: string) => void;
  counterLoading: boolean;
  counterError: string | null;
  counterResult: TransferNegotiationResponseData["decision"] | "error" | null;
  counterFeedback: NegotiationFeedbackPanelData | null;
  activeCounterOffer: TransferOfferData | null;
  openCounterNegotiation: (player: PlayerData, offer: TransferOfferData) => void;
  closeCounterNegotiation: () => void;
  handleCounterOffer: () => Promise<void>;
}

export function useTransferCounterOfferFlow({
  onGameUpdate,
}: UseTransferCounterOfferFlowArgs): UseTransferCounterOfferFlowResult {
  const { t } = useTranslation();
  const [counterTarget, setCounterTarget] = useState<CounterTarget | null>(
    null,
  );
  const [counterAmount, setCounterAmount] = useState("");
  const [counterLoading, setCounterLoading] = useState(false);
  const [counterError, setCounterError] = useState<string | null>(null);
  const [counterResult, setCounterResult] = useState<
    TransferNegotiationResponseData["decision"] | "error" | null
  >(null);
  const [counterFeedback, setCounterFeedback] =
    useState<NegotiationFeedbackPanelData | null>(null);

  const activeCounterOffer = counterTarget
    ? (counterTarget.player.transfer_offers.find(
        (offer) => offer.id === counterTarget.offerId,
      ) ?? null)
    : null;

  const openCounterNegotiation = (
    player: PlayerData,
    offer: TransferOfferData,
  ): void => {
    setCounterTarget({
      player,
      offerId: offer.id,
      fromTeamId: offer.from_team_id,
      fee: offer.fee,
    });
    setCounterAmount(
      formatTransferFeeInput(offer.suggested_counter_fee ?? offer.fee),
    );
    setCounterError(null);
    setCounterResult(null);
    setCounterFeedback(buildResumedCounterFeedback(offer));
  };

  const closeCounterNegotiation = (): void => {
    setCounterTarget(null);
    setCounterAmount("");
    setCounterError(null);
    setCounterResult(null);
    setCounterFeedback(null);
  };

  const handleCounterOffer = async (): Promise<void> => {
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
      setCounterFeedback(
        normalizeTransferNegotiationFeedback(response.feedback),
      );
      if (response.suggested_fee !== null) {
        setCounterAmount(formatTransferFeeInput(response.suggested_fee));
      }
    } catch (err: any) {
      setCounterError(
        mapTransferNegotiationError(t, err?.toString() || "error"),
      );
    } finally {
      setCounterLoading(false);
    }
  };

  return {
    counterTarget,
    counterAmount,
    setCounterAmount,
    counterLoading,
    counterError,
    counterResult,
    counterFeedback,
    activeCounterOffer,
    openCounterNegotiation,
    closeCounterNegotiation,
    handleCounterOffer,
  };
}
