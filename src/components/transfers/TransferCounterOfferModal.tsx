import { useTranslation } from "react-i18next";

import type {
  PlayerData,
  TeamData,
  TransferOfferData,
} from "../../store/gameStore";
import {
  formatExactMoney,
  formatVal,
  getTeamName,
  positionBadgeVariant,
} from "../../lib/helpers";
import type { TransferNegotiationResponseData } from "../../services/transfersService";
import NegotiationFeedbackPanel, {
  type NegotiationFeedbackPanelData,
} from "../NegotiationFeedbackPanel";
import { Badge } from "../ui";
import { translatePositionAbbreviation } from "../squad/SquadTab.helpers";
import TransferNegotiationHistory from "./TransferNegotiationHistory";
import { parseTransferFeeInput } from "./TransfersTab.helpers";

interface TransferCounterTarget {
  player: PlayerData;
  offerId: string;
  fromTeamId: string;
  fee: number;
}

interface TransferCounterOfferModalProps {
  counterTarget: TransferCounterTarget;
  teams: TeamData[];
  counterAmount: string;
  onCounterAmountChange: (value: string) => void;
  counterFeedback: NegotiationFeedbackPanelData | null;
  activeCounterOffer: TransferOfferData | null;
  counterResult: TransferNegotiationResponseData["decision"] | "error" | null;
  counterError: string | null;
  counterLoading: boolean;
  onSubmit: () => void;
  onClose: () => void;
}

export default function TransferCounterOfferModal({
  counterTarget,
  teams,
  counterAmount,
  onCounterAmountChange,
  counterFeedback,
  activeCounterOffer,
  counterResult,
  counterError,
  counterLoading,
  onSubmit,
  onClose,
}: TransferCounterOfferModalProps) {
  const { t } = useTranslation();
  const parsedCounterAmount = parseTransferFeeInput(counterAmount);

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
          {t("transfers.counterOffer")}
        </h3>
        <div className="flex items-center gap-3 mb-4">
          <Badge
            variant={positionBadgeVariant(counterTarget.player.position)}
            size="sm"
          >
            {translatePositionAbbreviation(t, counterTarget.player.position)}
          </Badge>
          <div>
            <p className="font-semibold text-sm text-gray-800 dark:text-gray-200">
              {counterTarget.player.full_name}
            </p>
            <p className="text-xs text-gray-400">
              {getTeamName(teams, counterTarget.fromTeamId)} •
              {t("transfers.currentOffer", {
                fee: formatVal(counterTarget.fee),
              })}
            </p>
          </div>
        </div>
        {counterFeedback ? (
          <p className="text-xs text-gray-500 dark:text-gray-400 mb-3">
            {t("transfers.resumeNegotiationHint")}
          </p>
        ) : null}
        <label
          htmlFor="counter-offer-amount"
          className="text-xs font-heading font-bold uppercase tracking-wider text-gray-500 dark:text-gray-400 mb-1 block"
        >
          {t("transfers.counterAmount")}
        </label>
        <input
          id="counter-offer-amount"
          type="text"
          inputMode="numeric"
          pattern="[0-9]*"
          value={counterAmount}
          onChange={(event) => onCounterAmountChange(event.target.value)}
          className="w-full px-3 py-2 rounded-lg bg-gray-50 dark:bg-navy-700 border border-gray-200 dark:border-navy-600 text-sm text-gray-800 dark:text-gray-200 mb-3 focus:outline-none focus:ring-2 focus:ring-primary-500/50"
        />
        {parsedCounterAmount !== null ? (
          <p className="text-xs text-gray-500 dark:text-gray-400 mb-3">
            {formatExactMoney(parsedCounterAmount)}
          </p>
        ) : null}
        <NegotiationFeedbackPanel
          feedback={counterFeedback}
          titleKey="transfers.negotiationPulse"
          roundKey="transfers.negotiationRound"
          patienceKey="transfers.negotiationPatience"
          tensionKey="transfers.negotiationTension"
          className="mb-3"
        />
        <TransferNegotiationHistory offer={activeCounterOffer} mode="incoming" />
        {counterResult ? (
          <div
            className={`text-xs font-heading font-bold uppercase tracking-wider mb-3 ${counterResult === "accepted" ? "text-green-500" : counterResult === "rejected" ? "text-red-500" : "text-amber-500"}`}
          >
            {counterResult === "accepted"
              ? t("transfers.counterAccepted")
              : counterResult === "rejected"
                ? t("transfers.counterRejected")
                : t("transfers.counterCountered")}
          </div>
        ) : null}
        {counterError ? (
          <div className="text-xs font-heading font-bold uppercase tracking-wider mb-3 text-red-500">
            {counterError}
          </div>
        ) : null}
        <div className="flex gap-2">
          <button
            onClick={onSubmit}
            disabled={counterLoading || counterResult === "accepted"}
            className="flex-1 py-2 bg-primary-500 hover:bg-primary-600 text-white rounded-lg font-heading font-bold text-sm uppercase tracking-wider transition-colors disabled:opacity-50"
          >
            {counterLoading
              ? t("transfers.submitting")
              : t("transfers.submitCounter")}
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