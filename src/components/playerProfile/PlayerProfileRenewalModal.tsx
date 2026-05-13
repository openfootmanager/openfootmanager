import DashboardModalFrame from "../dashboard/DashboardModalFrame";
import NegotiationFeedbackPanel from "../NegotiationFeedbackPanel";
import { Button } from "../ui";
import { formatPlayerWage } from "./PlayerProfile.helpers";
import type {
  NegotiationFeedbackData,
  RenewalProjection,
} from "./PlayerProfile.renewal";

type TranslateFn = (
  key: string,
  options?: Record<string, string | number>,
) => string;

interface PlayerProfileRenewalModalProps {
  show: boolean;
  playerName: string;
  t: TranslateFn;
  weeklySuffix: string;
  renewalWage: string;
  renewalLength: string;
  renewalIsTerminal: boolean;
  isRenewalWageValid: boolean;
  renewalViolatesSoftCap: boolean;
  renewalProjection: RenewalProjection | null;
  renewalStatusMessage: string | null;
  renewalStatusClassName: string;
  renewalCooledOff: boolean;
  renewalFeedback: NegotiationFeedbackData | null;
  renewalSubmitting: boolean;
  renewalSubmitDisabled: boolean;
  onWageChange: (value: string) => void;
  onLengthChange: (value: string) => void;
  onClose: () => void;
  onDelegate: () => void;
  onSubmit: () => void;
}

export default function PlayerProfileRenewalModal({
  show,
  playerName,
  t,
  weeklySuffix,
  renewalWage,
  renewalLength,
  renewalIsTerminal,
  isRenewalWageValid,
  renewalViolatesSoftCap,
  renewalProjection,
  renewalStatusMessage,
  renewalStatusClassName,
  renewalCooledOff,
  renewalFeedback,
  renewalSubmitting,
  renewalSubmitDisabled,
  onWageChange,
  onLengthChange,
  onClose,
  onDelegate,
  onSubmit,
}: PlayerProfileRenewalModalProps) {
  if (!show) {
    return null;
  }

  return (
    <DashboardModalFrame maxWidthClassName="max-w-md">
      <div className="space-y-4">
        <div>
          <h3 className="text-lg font-heading font-bold uppercase tracking-wider text-gray-900 dark:text-gray-100">
            {t("playerProfile.renewalTitle")}
          </h3>
          <p className="text-sm text-gray-500 dark:text-gray-400 mt-1">
            {playerName}
          </p>
        </div>

        <div className="space-y-3">
          <div>
            <label
              htmlFor="renewal-wage"
              className="text-xs font-heading font-bold uppercase tracking-wider text-gray-500 dark:text-gray-400 block mb-1"
            >
              {t("playerProfile.renewalWage")}
            </label>
            <input
              id="renewal-wage"
              type="number"
              min="1"
              step="1"
              value={renewalWage}
              onChange={(event) => onWageChange(event.target.value)}
              disabled={renewalIsTerminal}
              className="w-full px-3 py-2 rounded-lg bg-gray-50 dark:bg-navy-700 border border-gray-200 dark:border-navy-600 text-sm text-gray-800 dark:text-gray-200 focus:outline-none focus:ring-2 focus:ring-primary-500/50"
            />
          </div>

          <div>
            <label
              htmlFor="renewal-length"
              className="text-xs font-heading font-bold uppercase tracking-wider text-gray-500 dark:text-gray-400 block mb-1"
            >
              {t("playerProfile.renewalLength")}
            </label>
            <input
              id="renewal-length"
              type="number"
              min="1"
              max="5"
              step="1"
              value={renewalLength}
              onChange={(event) => onLengthChange(event.target.value)}
              disabled={renewalIsTerminal}
              className="w-full px-3 py-2 rounded-lg bg-gray-50 dark:bg-navy-700 border border-gray-200 dark:border-navy-600 text-sm text-gray-800 dark:text-gray-200 focus:outline-none focus:ring-2 focus:ring-primary-500/50"
            />
          </div>
        </div>

        {!isRenewalWageValid && renewalWage !== "" ? (
          <p className="text-sm text-red-500">
            {t("playerProfile.renewalInvalidWage")}
          </p>
        ) : null}

        {renewalViolatesSoftCap ? (
          <p className="text-sm text-red-500">
            {t("playerProfile.renewalBudgetWarning")}
          </p>
        ) : null}

        {renewalProjection ? (
          <div className="rounded-lg border border-gray-200 dark:border-navy-600 bg-gray-50 dark:bg-navy-700/40 p-3 space-y-2">
            <p className="text-xs font-heading font-bold uppercase tracking-wider text-gray-500 dark:text-gray-400">
              {t("playerProfile.renewalProjectionTitle")}
            </p>
            <p className="text-xs text-gray-600 dark:text-gray-300">
              {t("playerProfile.renewalProjectionWageBill", {
                before: formatPlayerWage(
                  renewalProjection.current_annual_wage_bill,
                  weeklySuffix,
                ),
                after: formatPlayerWage(
                  renewalProjection.projected_annual_wage_bill,
                  weeklySuffix,
                ),
              })}
            </p>
            <p className="text-xs text-gray-600 dark:text-gray-300">
              {t("playerProfile.renewalProjectionBudgetUsage", {
                before:
                  renewalProjection.annual_wage_budget > 0
                    ? Math.round(
                      (renewalProjection.current_annual_wage_bill /
                        renewalProjection.annual_wage_budget) *
                      100,
                    )
                    : 0,
                after:
                  renewalProjection.annual_wage_budget > 0
                    ? Math.round(
                      (renewalProjection.projected_annual_wage_bill /
                        renewalProjection.annual_wage_budget) *
                      100,
                    )
                    : 0,
              })}
            </p>
            <p className="text-xs text-gray-600 dark:text-gray-300">
              {t("playerProfile.renewalProjectionRunway", {
                before:
                  renewalProjection.current_cash_runway_weeks === null
                    ? t("finances.runwayStable")
                    : t("finances.runwayWeeks", {
                      count: renewalProjection.current_cash_runway_weeks,
                    }),
                after:
                  renewalProjection.projected_cash_runway_weeks === null
                    ? t("finances.runwayStable")
                    : t("finances.runwayWeeks", {
                      count: renewalProjection.projected_cash_runway_weeks,
                    }),
              })}
            </p>
          </div>
        ) : null}

        {renewalStatusMessage ? (
          <p className={`text-sm font-medium ${renewalStatusClassName}`}>
            {renewalStatusMessage}
          </p>
        ) : null}

        {renewalCooledOff ? (
          <p className="text-sm text-amber-600 dark:text-amber-300">
            {t("playerProfile.renewalCooledOff")}
          </p>
        ) : null}

        <NegotiationFeedbackPanel
          feedback={renewalFeedback}
          titleKey="playerProfile.renewalConversationTitle"
          roundKey="playerProfile.renewalRound"
          patienceKey="playerProfile.renewalPatience"
          tensionKey="playerProfile.renewalTension"
        />

        <div className="flex gap-2 justify-end">
          {renewalIsTerminal ? (
            <Button variant="ghost" onClick={onClose}>
              {t("common.done")}
            </Button>
          ) : (
            <>
              <Button variant="ghost" onClick={onClose}>
                {t("common.cancel")}
              </Button>
              <Button
                variant="outline"
                onClick={onDelegate}
                disabled={renewalSubmitting}
              >
                {t("playerProfile.delegateRenewal")}
              </Button>
              <Button onClick={onSubmit} disabled={renewalSubmitDisabled}>
                {t("playerProfile.renewalSubmit")}
              </Button>
            </>
          )}
        </div>
      </div>
    </DashboardModalFrame>
  );
}
