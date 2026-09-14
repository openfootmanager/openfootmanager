import { useEffect, useState } from "react";
import { useTranslation } from "react-i18next";

import type { GameStateData, PlayerData, TeamData, TransferOfferData } from "../../store/gameStore";
import {
  makeTransferBid,
  previewTransferBidFinancialImpact,
  type TransferBidProjectionData,
  type TransferNegotiationFeedbackData,
  type TransferNegotiationResponseData,
} from "../../services/transfersService";
import { getErrorMessage, resolveTranslatedErrorMessage } from "../../utils/errorMessage";
import {
  buildResumedBidFeedback,
  getOutgoingNegotiationOffer,
  normalizeTransferNegotiationFeedback,
} from "./TransfersTab.helpers";

interface UseTransferBidFlowArgs {
  gameState: GameStateData;
  onGameUpdate?: (game: GameStateData) => void;
}

interface UseTransferBidFlowResult {
  bidTarget: PlayerData | null;
  bidAmount: string;
  setBidAmount: (value: string) => void;
  bidResult: TransferNegotiationResponseData["decision"] | "error" | null;
  /** The translated reason a bid failed. Set only when `bidResult` is `"error"`. */
  bidError: string | null;
  bidLoading: boolean;
  bidFeedback: TransferNegotiationFeedbackData | null;
  bidProjection: TransferBidProjectionData["projection"] | null;
  bidFee: number | null;
  activeBidOffer: TransferOfferData | null;
  myTeam: TeamData | null;
  hasExistingOffer: boolean;
  bidSubmitDisabled: boolean;
  openBidNegotiation: (player: PlayerData) => void;
  closeBidNegotiation: () => void;
  handleMakeBid: () => Promise<void>;
}

export function useTransferBidFlow({
  gameState,
  onGameUpdate,
}: UseTransferBidFlowArgs): UseTransferBidFlowResult {
  const { t } = useTranslation();
  const userTeamId = gameState.manager.team_id;
  const myTeam = gameState.teams.find((team) => team.id === gameState.manager.team_id) ?? null;
  const [bidTarget, setBidTarget] = useState<PlayerData | null>(null);
  const [bidAmount, setBidAmount] = useState("");
  const [bidResult, setBidResult] = useState<
    TransferNegotiationResponseData["decision"] | "error" | null
  >(null);
  const [bidLoading, setBidLoading] = useState(false);
  const [bidFeedback, setBidFeedback] = useState<TransferNegotiationFeedbackData | null>(null);
  // Separate from `bidResult` on purpose. `bidResult` is the backend's *decision* and the UI
  // switches on it; the message explaining a failure is a different thing and needs
  // translating. Before this they shared one variable, which only type-checked because the
  // catch clause was `any`.
  const [bidError, setBidError] = useState<string | null>(null);
  const [bidProjection, setBidProjection] = useState<
    TransferBidProjectionData["projection"] | null
  >(null);

  const activeBidOffer = bidTarget ? getOutgoingNegotiationOffer(bidTarget, userTeamId) : null;
  const bidAmountMillions = Number.parseFloat(bidAmount);
  const bidFee = Number.isFinite(bidAmountMillions)
    ? Math.round(bidAmountMillions * 1_000_000)
    : null;

  useEffect(() => {
    if (!bidTarget || bidFee === null || bidFee <= 0) {
      setBidProjection(null);
      return;
    }

    let cancelled = false;

    const loadProjection = async (): Promise<void> => {
      try {
        const result = await previewTransferBidFinancialImpact(bidTarget.id, bidFee);

        if (!cancelled) {
          setBidProjection(result.projection ?? null);
        }
      } catch {
        if (!cancelled) {
          setBidProjection(null);
        }
      }
    };

    void loadProjection();

    return () => {
      cancelled = true;
    };
  }, [bidFee, bidTarget]);

  const openBidNegotiation = (player: PlayerData): void => {
    const existingOffer = getOutgoingNegotiationOffer(player, userTeamId);

    setBidTarget(player);
    setBidAmount(
      (
        (existingOffer?.suggested_counter_fee ?? existingOffer?.fee ?? player.market_value) /
        1_000_000
      ).toFixed(existingOffer ? 2 : 1),
    );
    setBidResult(null);
    setBidError(null);
    setBidFeedback(buildResumedBidFeedback(existingOffer));
    setBidProjection(null);
  };

  const closeBidNegotiation = (): void => {
    setBidTarget(null);
    setBidAmount("");
    setBidResult(null);
    setBidError(null);
    setBidFeedback(null);
    setBidProjection(null);
  };

  const handleMakeBid = async (): Promise<void> => {
    if (!bidTarget || bidFee === null || bidFee <= 0) {
      return;
    }

    setBidLoading(true);
    setBidResult(null);
    setBidError(null);
    setBidFeedback(null);

    try {
      const response = await makeTransferBid(bidTarget.id, bidFee);
      setBidResult(response.decision);
      setBidError(null);
      setBidFeedback(normalizeTransferNegotiationFeedback(response.feedback));
      onGameUpdate?.(response.game);

      // `suggested_fee` is in raw euros, but this input is denominated in
      // millions (see `bidFee` above and `openBidNegotiation`). Convert
      // before pre-filling, or the next submit sends 1,000,000× the value.
      if (response.suggested_fee !== null) {
        setBidAmount((response.suggested_fee / 1_000_000).toFixed(2));
      }
    } catch (error: unknown) {
      setBidResult("error");
      setBidError(resolveTranslatedErrorMessage(getErrorMessage(error), t));
      setBidFeedback(null);
    } finally {
      setBidLoading(false);
    }
  };

  return {
    bidTarget,
    bidAmount,
    setBidAmount,
    bidResult,
    bidError,
    bidLoading,
    bidFeedback,
    bidProjection,
    bidFee,
    activeBidOffer,
    myTeam,
    hasExistingOffer: activeBidOffer !== null,
    bidSubmitDisabled:
      bidLoading ||
      bidResult === "accepted" ||
      bidFee === null ||
      bidFee <= 0 ||
      bidProjection === null ||
      bidProjection.exceeds_transfer_budget ||
      bidProjection.exceeds_finance,
    openBidNegotiation,
    closeBidNegotiation,
    handleMakeBid,
  };
}
