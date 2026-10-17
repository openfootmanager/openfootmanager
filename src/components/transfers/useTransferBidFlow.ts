import { useEffect, useRef, useState } from "react";
import { useTranslation } from "react-i18next";

import type { GameStateData, PlayerData, TeamData, TransferOfferData } from "../../store/gameStore";
import {
  type BidNegotiationStatusData,
  getBidNegotiationStatus,
  makeTransferBid,
  previewTransferBidFinancialImpact,
  type TransferBidProjectionData,
  type TransferNegotiationFeedbackData,
  type TransferNegotiationResponseData,
} from "../../services/transfersService";
import { getErrorMessage, resolveTranslatedErrorMessage } from "../../utils/errorMessage";
import {
  buildResumedBidFeedback,
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
  /** The day broken-off talks can resume; set means the bid form is replaced by that date. */
  talksResumeOn: string | null;
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
  const myTeam = gameState.teams.find((team) => team.id === gameState.manager.team_id) ?? null;
  // The id, not the player: the player is read from the game state on every render, so the panel
  // sees the round and the offers a bid has just advanced instead of the ones it opened with.
  const [bidTargetId, setBidTargetId] = useState<string | null>(null);
  const bidTarget =
    bidTargetId === null
      ? null
      : (gameState.players.find((player) => player.id === bidTargetId) ?? null);
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

  // What the backend says the next bid continues (or that talks are broken off until a day): the
  // cooldown is its rule, so the panel asks rather than computing it from the offers.
  const [negotiation, setNegotiation] = useState<BidNegotiationStatusData | null>(null);
  useEffect(() => {
    if (bidTargetId === null) {
      setNegotiation(null);
      return;
    }
    let cancelled = false;
    getBidNegotiationStatus(bidTargetId)
      .then((status) => {
        if (!cancelled) setNegotiation(status);
      })
      .catch(() => {
        if (!cancelled) setNegotiation(null);
      });
    return () => {
      cancelled = true;
    };
  }, [bidTargetId, gameState]);

  const activeBidOffer =
    bidTarget && negotiation?.offer_id
      ? (bidTarget.transfer_offers.find((offer) => offer.id === negotiation.offer_id) ?? null)
      : null;
  const talksResumeOn = negotiation?.talks_resume_on ?? null;

  // Resuming a negotiation pre-fills the amount and the feedback once per opening, when the
  // backend's answer first names one; a later answer (after a bid) must not overwrite them.
  const resumedFor = useRef<string | null>(null);
  useEffect(() => {
    if (!bidTarget || !activeBidOffer || resumedFor.current === bidTarget.id) return;
    resumedFor.current = bidTarget.id;
    setBidAmount(
      ((activeBidOffer.suggested_counter_fee ?? activeBidOffer.fee) / 1_000_000).toFixed(2),
    );
    setBidFeedback(buildResumedBidFeedback(activeBidOffer));
  }, [bidTarget, activeBidOffer]);
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
    resumedFor.current = null;
    setBidTargetId(player.id);
    setBidAmount((player.market_value / 1_000_000).toFixed(1));
    setBidResult(null);
    setBidError(null);
    setBidFeedback(null);
    setBidProjection(null);
    setNegotiation(null);
  };

  const closeBidNegotiation = (): void => {
    resumedFor.current = null;
    setBidTargetId(null);
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
    talksResumeOn,
    myTeam,
    hasExistingOffer: activeBidOffer?.status === "Pending",
    bidSubmitDisabled:
      bidLoading ||
      bidResult === "accepted" ||
      bidFee === null ||
      bidFee <= 0 ||
      talksResumeOn !== null ||
      bidProjection === null ||
      bidProjection.exceeds_transfer_budget ||
      bidProjection.exceeds_finance ||
      bidProjection.blocked_by_wage_policy,
    openBidNegotiation,
    closeBidNegotiation,
    handleMakeBid,
  };
}
