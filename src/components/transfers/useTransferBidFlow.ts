import { useEffect, useState } from "react";

import type {
    GameStateData,
    PlayerData,
    TeamData,
    TransferOfferData,
} from "../../store/gameStore";
import {
    makeTransferBid,
    previewTransferBidFinancialImpact,
    type TransferBidProjectionData,
    type TransferNegotiationFeedbackData,
    type TransferNegotiationResponseData,
} from "../../services/transfersService";
import {
    buildResumedBidFeedback,
    formatTransferFeeInput,
    getOutgoingNegotiationOffer,
    normalizeTransferNegotiationFeedback,
} from "./TransfersTab.helpers";

interface UseTransferBidFlowArgs {
    gameState: GameStateData;
    onGameUpdate?: (game: GameStateData) => void;
    /**
     * Called when a bid agrees the fee and the offer enters the personal-terms
     * phase, so the UI can immediately launch the club-to-player negotiation
     * instead of leaving a dangling `PersonalTermsPending` offer.
     */
    onFeeAgreed?: (
        player: PlayerData,
        offerId: string,
        buyerTeamId: string,
    ) => void;
}

interface UseTransferBidFlowResult {
    bidTarget: PlayerData | null;
    bidAmount: string;
    setBidAmount: (value: string) => void;
    bidResult: TransferNegotiationResponseData["decision"] | "error" | null;
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
    onFeeAgreed,
}: UseTransferBidFlowArgs): UseTransferBidFlowResult {
    const userTeamId = gameState.manager.team_id;
    const myTeam = gameState.teams.find(
        (team) => team.id === gameState.manager.team_id,
    ) ?? null;
    const [bidTarget, setBidTarget] = useState<PlayerData | null>(null);
    const [bidAmount, setBidAmount] = useState("");
    const [bidResult, setBidResult] = useState<
        TransferNegotiationResponseData["decision"] | "error" | null
    >(null);
    const [bidLoading, setBidLoading] = useState(false);
    const [bidFeedback, setBidFeedback] =
        useState<TransferNegotiationFeedbackData | null>(null);
    const [bidProjection, setBidProjection] =
        useState<TransferBidProjectionData["projection"] | null>(null);

    const activeBidOffer = bidTarget
        ? getOutgoingNegotiationOffer(bidTarget, userTeamId)
        : null;
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
                const result = await previewTransferBidFinancialImpact(
                    bidTarget.id,
                    bidFee,
                );

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
                (existingOffer?.suggested_counter_fee ??
                    existingOffer?.fee ??
                    player.market_value) /
                1_000_000
            ).toFixed(existingOffer ? 2 : 1),
        );
        setBidResult(null);
        setBidFeedback(buildResumedBidFeedback(existingOffer));
        setBidProjection(null);
    };

    const closeBidNegotiation = (): void => {
        setBidTarget(null);
        setBidAmount("");
        setBidResult(null);
        setBidFeedback(null);
        setBidProjection(null);
    };

    const handleMakeBid = async (): Promise<void> => {
        if (!bidTarget || bidFee === null || bidFee <= 0) {
            return;
        }

        setBidLoading(true);
        setBidResult(null);
        setBidFeedback(null);

        try {
            const response = await makeTransferBid(bidTarget.id, bidFee);
            setBidResult(response.decision);
            setBidFeedback(normalizeTransferNegotiationFeedback(response.feedback));
            onGameUpdate?.(response.game);

            if (response.suggested_fee !== null) {
                setBidAmount(formatTransferFeeInput(response.suggested_fee));
            }

            // Fee agreed → the offer is now in personal terms. Hand straight over
            // to the club-to-player negotiation so the user never has to hunt for
            // it, and close this bid dialog.
            if (response.decision === "accepted" && userTeamId) {
                const target = bidTarget;
                const feeAgreedOffer = response.game.players
                    .find((player) => player.id === target.id)
                    ?.transfer_offers.find(
                        (offer) =>
                            offer.from_team_id === userTeamId &&
                            offer.status === "PersonalTermsPending",
                    );

                if (feeAgreedOffer && onFeeAgreed) {
                    closeBidNegotiation();
                    onFeeAgreed(target, feeAgreedOffer.id, userTeamId);
                    return;
                }
            }
        } catch (error: any) {
            setBidResult(error?.toString() || "error");
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
