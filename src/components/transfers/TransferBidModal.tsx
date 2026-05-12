import { useTranslation } from "react-i18next";

import type {
  PlayerData,
  TeamData,
  TransferOfferData,
} from "../../store/gameStore";
import {
  formatVal,
  getTeamName,
  positionBadgeVariant,
} from "../../lib/helpers";
import type {
  TransferBidProjectionData,
  TransferNegotiationResponseData,
} from "../../services/transfersService";
import NegotiationFeedbackPanel, {
  type NegotiationFeedbackPanelData,
} from "../NegotiationFeedbackPanel";
import { Badge } from "../ui";
import { translatePositionAbbreviation } from "../squad/SquadTab.helpers";
import TransferNegotiationHistory from "./TransferNegotiationHistory";

interface TransferBidModalProps {
  bidTarget: PlayerData;
  teams: TeamData[];
  bidAmount: string;
  onBidAmountChange: (value: string) => void;
  myTeam: TeamData | null;
  bidFee: number | null;
  bidProjection: TransferBidProjectionData["projection"] | null;
  bidFeedback: NegotiationFeedbackPanelData | null;
  activeBidOffer: TransferOfferData | null;
  hasExistingOffer: boolean;
  bidResult: TransferNegotiationResponseData["decision"] | "error" | null;
  bidLoading: boolean;
  bidSubmitDisabled: boolean;
  onSubmit: () => void;
  onClose: () => void;
}

export default function TransferBidModal({
  bidTarget,
  teams,
  bidAmount,
  onBidAmountChange,
  myTeam,
  bidFee,
  bidProjection,
  bidFeedback,
  activeBidOffer,
  hasExistingOffer,
  bidResult,
  bidLoading,
  bidSubmitDisabled,
  onSubmit,
  onClose,
}: TransferBidModalProps) {
  const { t } = useTranslation();

  return (
    <div
      className="fixed inset-0 bg-black/50 flex items-center justify-center z-50"
      onClick={onClose}
    >
      <div
        className="bg-white dark:bg-navy-800 rounded-xl shadow-2xl border border-gray-200 dark:border-navy-600 p-6 w-full max-w-sm"
        onClick={(event) => event.stopPropagation()}
      >
        <h3 className="text-sm font-heading font-bold uppercase tracking-wider text-gray-500 dark:text-gray-400 mb-3">
          {t("transfers.makeBid")}
        </h3>
        <div className="flex items-center gap-3 mb-4">
          <Badge variant={positionBadgeVariant(bidTarget.position)} size="sm">
            {translatePositionAbbreviation(t, bidTarget.position)}
          </Badge>
          <div>
            <p className="font-semibold text-sm text-gray-800 dark:text-gray-200">
              {bidTarget.full_name}
            </p>
            <p className="text-xs text-gray-400">
              {getTeamName(teams, bidTarget.team_id)} •{" "}
              {t("transfers.playerValue", {
                value: formatVal(bidTarget.market_value),
              })}
            </p>
          </div>
        </div>
        {hasExistingOffer ? (
          <p className="text-xs text-gray-500 dark:text-gray-400 mb-3">
            {t("transfers.resumeNegotiationHint")}
          </p>
        ) : null}
        <label
          htmlFor="bid-amount"
          className="text-xs font-heading font-bold uppercase tracking-wider text-gray-500 dark:text-gray-400 mb-1 block"
        >
          {t("transfers.bidAmount")}
        </label>
        <input
          id="bid-amount"
          type="number"
          step="0.1"
          min="0"
          value={bidAmount}
          onChange={(event) => onBidAmountChange(event.target.value)}
          className="w-full px-3 py-2 rounded-lg bg-gray-50 dark:bg-navy-700 border border-gray-200 dark:border-navy-600 text-sm text-gray-800 dark:text-gray-200 mb-3 focus:outline-none focus:ring-2 focus:ring-primary-500/50"
        />
        {myTeam && bidFee !== null && bidProjection ? (
          <div className="rounded-lg border border-gray-200 dark:border-navy-700 bg-white/70 dark:bg-navy-900/40 p-3 mb-3 space-y-2">
            <p className="text-[11px] font-heading font-bold uppercase tracking-wider text-gray-500 dark:text-gray-400">
              {t("transfers.bidImpactTitle")}
            </p>
            <p className="text-xs text-gray-600 dark:text-gray-300">
              {t("transfers.bidImpactTransferBudget", {
                before: formatVal(bidProjection.transfer_budget_before),
                after: formatVal(bidProjection.transfer_budget_after),
              })}
            </p>
            <p className="text-xs text-gray-600 dark:text-gray-300">
              {t("transfers.bidImpactBalance", {
                before: formatVal(bidProjection.finance_before),
                after: formatVal(bidProjection.finance_after),
              })}
            </p>
            <p className="text-xs text-gray-600 dark:text-gray-300">
              {t("transfers.bidImpactWagePressure", {
                percent: bidProjection.projected_wage_budget_usage_pct,
              })}
            </p>
            {bidProjection.exceeds_transfer_budget ? (
              <p className="text-xs text-red-500">
                {t("transfers.bidImpactOverTransferBudget")}
              </p>
            ) : null}
            {bidProjection.exceeds_finance ? (
              <p className="text-xs text-red-500">
                {t("transfers.bidImpactOverBalance")}
              </p>
            ) : null}
          </div>
        ) : null}
        <NegotiationFeedbackPanel
          feedback={bidFeedback}
          titleKey="transfers.negotiationPulse"
          roundKey="transfers.negotiationRound"
          patienceKey="transfers.negotiationPatience"
          tensionKey="transfers.negotiationTension"
          className="mb-3"
        />
        <TransferNegotiationHistory offer={activeBidOffer} mode="outgoing" />
        {bidResult ? (
          <div
            className={`text-xs font-heading font-bold uppercase tracking-wider mb-3 ${bidResult === "accepted" ? "text-green-500" : bidResult === "rejected" ? "text-red-500" : "text-amber-500"}`}
          >
            {bidResult === "accepted"
              ? t("transfers.bidAccepted")
              : bidResult === "rejected"
                ? t("transfers.bidRejected")
                : bidResult === "counter_offer"
                  ? t("transfers.bidCountered")
                  : bidResult}
          </div>
        ) : null}
        <div className="flex gap-2">
          <button
            onClick={onSubmit}
            disabled={bidSubmitDisabled}
            className="flex-1 py-2 bg-primary-500 hover:bg-primary-600 text-white rounded-lg font-heading font-bold text-sm uppercase tracking-wider transition-colors disabled:opacity-50"
          >
            {bidLoading ? t("transfers.submitting") : t("transfers.submitBid")}
          </button>
          <button
            onClick={onClose}
            className="px-4 py-2 bg-gray-200 dark:bg-navy-700 text-gray-600 dark:text-gray-300 rounded-lg font-heading font-bold text-sm uppercase tracking-wider hover:bg-gray-300 dark:hover:bg-navy-600 transition-colors"
          >
            {t("transfers.close")}
          </button>
        </div>
      </div>
    </div>
  );
}
