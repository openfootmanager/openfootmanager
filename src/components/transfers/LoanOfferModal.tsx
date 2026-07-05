import { BadgeEuro, CalendarClock, CalendarDays, Percent } from "lucide-react";
import { useTranslation } from "react-i18next";

import type { PlayerData, TeamData } from "../../store/gameStore";
import { formatVal, getTeamName, positionBadgeVariant } from "../../lib/helpers";
import { formatDate } from "../../lib/dateFormatting";
import { Badge } from "../ui";
import { translatePositionAbbreviation } from "../squad/SquadTab.helpers";
import type {
  LoanPeriodOption,
  LoanPeriodOptionId,
} from "./TransfersTab.helpers";

export interface LoanOfferFormProps {
  loanTarget: PlayerData;
  teams: TeamData[];
  periodId: LoanPeriodOptionId | "";
  periodOptions: LoanPeriodOption[];
  selectedEndDate: string;
  onPeriodChange: (value: LoanPeriodOptionId) => void;
  wageContributionPct: number;
  onWageContributionChange: (value: number) => void;
  buyOptionEnabled: boolean;
  buyOptionFee: string;
  onBuyOptionEnabledChange: (value: boolean) => void;
  onBuyOptionFeeChange: (value: string) => void;
  result: "accepted" | "rejected" | "counter_offer" | "error" | null;
  titleKey?: string;
  submitLabelKey?: string;
  acceptedLabelKey?: string;
  rejectedLabelKey?: string;
  counteredLabelKey?: string;
  suggestedTerms?: {
    wageContributionPct: number;
    endDate: string;
    buyOptionFee?: number | null;
  } | null;
  error: string | null;
  loading: boolean;
  submitDisabled: boolean;
  noticeTitle?: string | null;
  noticeDetail?: string | null;
  acceptedMessage?: string | null;
  showPlayerSummary?: boolean;
  onSubmit: () => void;
  onClose: () => void;
}

type LoanOfferModalProps = LoanOfferFormProps;

export function LoanOfferForm({
  loanTarget,
  teams,
  periodId,
  periodOptions,
  selectedEndDate,
  onPeriodChange,
  wageContributionPct,
  onWageContributionChange,
  buyOptionEnabled,
  buyOptionFee,
  onBuyOptionEnabledChange,
  onBuyOptionFeeChange,
  result,
  titleKey = "transfers.makeLoanOffer",
  submitLabelKey = "transfers.submitLoanOffer",
  acceptedLabelKey = "transfers.loanOfferAccepted",
  rejectedLabelKey = "transfers.loanOfferRejected",
  counteredLabelKey = "transfers.loanOfferCountered",
  suggestedTerms = null,
  error,
  loading,
  submitDisabled,
  noticeTitle = null,
  noticeDetail = null,
  acceptedMessage = null,
  showPlayerSummary = true,
  onSubmit,
  onClose,
}: LoanOfferFormProps) {
  const { t, i18n } = useTranslation();

  return (
    <>
        <h3
          id="loan-offer-modal-title"
          className="text-sm font-heading font-bold uppercase tracking-wider text-gray-500 dark:text-gray-400 mb-3"
        >
          {t(titleKey)}
        </h3>
        {showPlayerSummary ? (
          <div className="flex items-center gap-3 mb-4">
            <Badge variant={positionBadgeVariant(loanTarget.position)} size="sm">
              {translatePositionAbbreviation(t, loanTarget.position)}
            </Badge>
            <div>
              <p className="font-semibold text-sm text-gray-800 dark:text-gray-200">
                {loanTarget.full_name}
              </p>
              <p className="text-xs text-gray-400">
                {getTeamName(teams, loanTarget.team_id)} •{" "}
                {t("transfers.playerValue", {
                  value: formatVal(loanTarget.market_value),
                })}
              </p>
            </div>
          </div>
        ) : null}

        {noticeTitle ? (
          <div
            role="status"
            className="mb-4 flex gap-2 rounded-lg border border-amber-200 bg-amber-50 px-3 py-2 text-amber-800 dark:border-amber-500/30 dark:bg-amber-500/10 dark:text-amber-100"
          >
            <CalendarClock className="mt-0.5 h-4 w-4 shrink-0" />
            <div className="text-xs">
              <p className="font-heading font-bold uppercase tracking-wider">
                {noticeTitle}
              </p>
              {noticeDetail ? <p className="mt-1">{noticeDetail}</p> : null}
            </div>
          </div>
        ) : null}

        <label
          htmlFor="loan-period"
          className="text-xs font-heading font-bold uppercase tracking-wider text-gray-500 dark:text-gray-400 mb-1 flex items-center gap-1.5"
        >
          <CalendarDays className="w-3.5 h-3.5" />
          {t("transfers.loanPeriod")}
        </label>
        <select
          id="loan-period"
          value={periodId}
          onChange={(event) =>
            onPeriodChange(event.target.value as LoanPeriodOptionId)
          }
          className="w-full px-3 py-2 rounded-lg bg-gray-50 dark:bg-navy-700 border border-gray-200 dark:border-navy-600 text-sm text-gray-800 dark:text-gray-200 mb-3 focus:outline-none focus:ring-2 focus:ring-primary-500/50"
        >
          {periodId ? null : (
            <option value="" disabled>
              {t("transfers.noLoanPeriodAvailable")}
            </option>
          )}
          {periodOptions.map((option) => {
            const label = t(option.labelKey);
            const disabledReason = option.disabledReasonKey
              ? t(option.disabledReasonKey)
              : null;

            return (
              <option
                key={option.id}
                value={option.id}
                disabled={option.disabled}
              >
                {disabledReason ? `${label} (${disabledReason})` : label}
              </option>
            );
          })}
        </select>

        {selectedEndDate ? (
          <p className="text-xs text-gray-500 dark:text-gray-400 mb-3">
            {t("transfers.loanEndsOn", {
              endDate: formatDate(selectedEndDate, i18n.language),
            })}
          </p>
        ) : (
          <p role="alert" className="text-xs text-red-600 dark:text-red-300 mb-3">
            {t("transfers.noLoanPeriodAvailable")}
          </p>
        )}

        <div
          id="loan-wage-contribution-label"
          className="text-xs font-heading font-bold uppercase tracking-wider text-gray-500 dark:text-gray-400 mb-2 flex items-center gap-1.5"
        >
          <Percent className="w-3.5 h-3.5" />
          {t("transfers.loanWageContribution")}
        </div>
        <div className="flex items-center gap-3 mb-3">
          <input
            id="loan-wage-contribution"
            type="range"
            min="0"
            max="100"
            step="5"
            value={wageContributionPct}
            aria-labelledby="loan-wage-contribution-label"
            onChange={(event) => {
              const nextValue = Number(event.target.value);
              onWageContributionChange(
                Number.isFinite(nextValue) ? nextValue : 0,
              );
            }}
            className="h-2 flex-1 cursor-pointer accent-primary-500"
          />
          <div className="relative w-24">
            <input
              type="number"
              min="0"
              max="100"
              step="5"
              value={wageContributionPct}
              aria-label={t("transfers.loanWageContributionManual")}
              onChange={(event) => {
                const nextValue = Number(event.target.value);
                onWageContributionChange(
                  Number.isFinite(nextValue)
                    ? Math.max(0, Math.min(100, nextValue))
                    : 0,
                );
              }}
              className="w-full rounded-lg border border-gray-200 bg-gray-50 px-3 py-2 pr-7 text-sm text-gray-800 focus:outline-none focus:ring-2 focus:ring-primary-500/50 dark:border-navy-600 dark:bg-navy-700 dark:text-gray-200"
            />
            <span className="pointer-events-none absolute right-3 top-1/2 -translate-y-1/2 text-xs text-gray-400">
              %
            </span>
          </div>
        </div>

        <p className="text-xs text-gray-500 dark:text-gray-400 mb-3">
          {t("transfers.loanWageSummary", {
            percent: wageContributionPct,
            wage: formatVal(Math.round((loanTarget.wage * wageContributionPct) / 100)),
          })}
        </p>

        <label className="flex items-start gap-2 rounded-lg border border-gray-200 dark:border-navy-600 bg-gray-50 dark:bg-navy-700/60 p-3 mb-3 cursor-pointer">
          <input
            type="checkbox"
            checked={buyOptionEnabled}
            onChange={(event) => onBuyOptionEnabledChange(event.target.checked)}
            className="mt-0.5 h-4 w-4 rounded border-gray-300 text-primary-500 focus:ring-primary-500"
          />
          <span>
            <span className="block text-xs font-heading font-bold uppercase tracking-wider text-gray-600 dark:text-gray-300">
              {t("transfers.loanToBuyOption")}
            </span>
            <span className="block text-xs text-gray-500 dark:text-gray-400 mt-0.5">
              {t("transfers.loanToBuyOptionDesc")}
            </span>
          </span>
        </label>

        <label
          htmlFor="loan-buy-option-fee"
          className="text-xs font-heading font-bold uppercase tracking-wider text-gray-500 dark:text-gray-400 mb-1 flex items-center gap-1.5"
        >
          <BadgeEuro className="w-3.5 h-3.5" />
          {t("transfers.buyOptionFee")}
        </label>
        <input
          id="loan-buy-option-fee"
          type="number"
          min="0"
          step="50000"
          value={buyOptionFee}
          disabled={!buyOptionEnabled}
          onChange={(event) => onBuyOptionFeeChange(event.target.value)}
          className="w-full px-3 py-2 rounded-lg bg-gray-50 dark:bg-navy-700 border border-gray-200 dark:border-navy-600 text-sm text-gray-800 dark:text-gray-200 mb-3 focus:outline-none focus:ring-2 focus:ring-primary-500/50 disabled:opacity-50"
        />

        {buyOptionEnabled && Number(buyOptionFee) > 0 ? (
          <p className="text-xs text-gray-500 dark:text-gray-400 mb-3">
            {t("transfers.loanBuyOptionSummary", {
              fee: formatVal(Number(buyOptionFee)),
            })}
          </p>
        ) : null}

        {result ? (
          <div
            className={`text-xs font-heading font-bold uppercase tracking-wider mb-3 ${result === "accepted" ? "text-green-500" : result === "counter_offer" ? "text-amber-500" : "text-red-600 dark:text-red-300"}`}
          >
            {result === "accepted"
              ? acceptedMessage ?? t(acceptedLabelKey)
              : result === "rejected"
                ? t(rejectedLabelKey)
                : result === "counter_offer"
                  ? t(counteredLabelKey)
                  : error}
          </div>
        ) : null}

        {suggestedTerms ? (
          <div className="rounded-lg border border-amber-200 bg-amber-50 px-3 py-2 text-xs text-amber-700 dark:border-amber-500/30 dark:bg-amber-500/10 dark:text-amber-200 mb-3">
            <p className="font-heading font-bold uppercase tracking-wider">
              {t("transfers.loanCounterSuggestedTerms", {
                percent: suggestedTerms.wageContributionPct,
                endDate: formatDate(suggestedTerms.endDate, i18n.language),
              })}
            </p>
            {suggestedTerms.buyOptionFee ? (
              <p className="mt-1">
                {t("transfers.loanCounterSuggestedBuyOption", {
                  fee: formatVal(suggestedTerms.buyOptionFee),
                })}
              </p>
            ) : null}
          </div>
        ) : null}

        {error && !result ? (
          <p role="alert" className="text-xs text-red-600 dark:text-red-300 mb-3">
            {error}
          </p>
        ) : null}

        <div className="flex gap-2">
          <button
            type="button"
            onClick={onSubmit}
            disabled={submitDisabled}
            className="flex-1 py-2 bg-primary-700 hover:bg-primary-800 text-white rounded-lg font-heading font-bold text-sm uppercase tracking-wider transition-colors disabled:opacity-50"
          >
            {loading ? t("transfers.submitting") : t(submitLabelKey)}
          </button>
          <button
            type="button"
            onClick={onClose}
            className="px-4 py-2 bg-gray-200 dark:bg-navy-700 text-gray-600 dark:text-gray-300 rounded-lg font-heading font-bold text-sm uppercase tracking-wider hover:bg-gray-300 dark:hover:bg-navy-600 transition-colors"
          >
            {t("transfers.close")}
          </button>
        </div>
    </>
  );
}

export default function LoanOfferModal(props: LoanOfferModalProps) {
  return (
    <div
      role="presentation"
      className="fixed inset-0 bg-black/50 flex items-center justify-center z-50"
      onClick={props.onClose}
    >
      <div
        role="dialog"
        aria-modal="true"
        aria-labelledby="loan-offer-modal-title"
        className="bg-white dark:bg-navy-800 rounded-xl shadow-2xl border border-gray-200 dark:border-navy-600 p-6 w-full max-w-md"
        onClick={(event) => event.stopPropagation()}
      >
        <LoanOfferForm {...props} />
      </div>
    </div>
  );
}
