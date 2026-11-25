import { useId } from "react";

import NegotiationFeedbackPanel from "../NegotiationFeedbackPanel";
import { Button, Dialog } from "../ui";
import { formatPlayerWage } from "./PlayerProfile.helpers";
import type { NegotiationFeedbackData, RenewalProjection } from "./PlayerProfile.renewal";

type TranslateFn = (key: string, options?: Record<string, string | number>) => string;

interface PlayerProfileRenewalModalProps {
  show: boolean;
  playerName: string;
  t: TranslateFn;
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
  renewalSubmitDisabled: boolean;
  delegateRenewalDisabled: boolean;
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
  renewalSubmitDisabled,
  delegateRenewalDisabled,
  onWageChange,
  onLengthChange,
  onClose,
  onDelegate,
  onSubmit,
}: PlayerProfileRenewalModalProps) {
  const titleId = useId();
  if (!show) {
    return null;
  }

  return (
    <Dialog labelledBy={titleId} onClose={onClose} className="w-full max-w-md rounded-2xl p-6">
      <div className="space-y-4">
        <div>
          <h3
            id={titleId}
            className="text-heading font-heading font-bold uppercase tracking-wider text-fg"
          >
            {t("playerProfile.renewalTitle")}
          </h3>
          <p className="text-body text-fg-muted mt-1">{playerName}</p>
        </div>

        <div className="space-y-3">
          <div>
            <label
              htmlFor="renewal-wage"
              className="text-caption font-heading font-bold uppercase tracking-wider text-fg-muted block mb-1"
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
              className="w-full px-3 py-2 rounded-lg bg-sunken border border-line text-body text-fg focus:outline-none focus-visible:ring-2 focus-visible:ring-focus focus-visible:ring-offset-2 focus-visible:ring-offset-panel disabled:opacity-50"
            />
          </div>

          <div>
            <label
              htmlFor="renewal-length"
              className="text-caption font-heading font-bold uppercase tracking-wider text-fg-muted block mb-1"
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
              className="w-full px-3 py-2 rounded-lg bg-sunken border border-line text-body text-fg focus:outline-none focus-visible:ring-2 focus-visible:ring-focus focus-visible:ring-offset-2 focus-visible:ring-offset-panel disabled:opacity-50"
            />
          </div>
        </div>

        {!isRenewalWageValid && renewalWage !== "" ? (
          <p className="text-body text-danger">{t("playerProfile.renewalInvalidWage")}</p>
        ) : null}

        {renewalViolatesSoftCap ? (
          <p className="text-body text-danger">{t("playerProfile.renewalBudgetWarning")}</p>
        ) : null}

        {renewalProjection ? (
          <div className="rounded-lg border border-line bg-sunken p-3 space-y-2">
            <p className="text-caption font-heading font-bold uppercase tracking-wider text-fg-muted">
              {t("playerProfile.renewalProjectionTitle")}
            </p>
            <p className="text-caption text-fg-muted">
              {t("playerProfile.renewalProjectionWageBill", {
                before: formatPlayerWage(renewalProjection.current_weekly_wage_spend),
                after: formatPlayerWage(renewalProjection.projected_weekly_wage_spend),
              })}
            </p>
            <p className="text-caption text-fg-muted">
              {t("playerProfile.renewalProjectionBudgetUsage", {
                before:
                  renewalProjection.annual_wage_budget > 0
                    ? Math.round(
                        (renewalProjection.current_weekly_wage_spend /
                          renewalProjection.annual_wage_budget) *
                          100,
                      )
                    : 0,
                after:
                  renewalProjection.annual_wage_budget > 0
                    ? Math.round(
                        (renewalProjection.projected_weekly_wage_spend /
                          renewalProjection.annual_wage_budget) *
                          100,
                      )
                    : 0,
              })}
            </p>
            <p className="text-caption text-fg-muted">
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
          <p className={`text-body font-medium ${renewalStatusClassName}`}>
            {renewalStatusMessage}
          </p>
        ) : null}

        {renewalCooledOff ? (
          <p className="text-body text-warning">{t("playerProfile.renewalCooledOff")}</p>
        ) : null}

        <NegotiationFeedbackPanel
          feedback={renewalFeedback}
          titleKey="playerProfile.renewalConversationTitle"
          roundKey="playerProfile.renewalRound"
          patienceKey="playerProfile.renewalPatience"
          tensionKey="playerProfile.renewalTension"
        />

        <div className="flex gap-3">
          {renewalIsTerminal ? (
            <Button className="flex-1" onClick={onClose}>
              {t("common.done")}
            </Button>
          ) : (
            <>
              <Button className="flex-1" variant="ghost" onClick={onClose}>
                {t("common.cancel")}
              </Button>
              <Button
                className="flex-1"
                variant="outline"
                onClick={onDelegate}
                disabled={delegateRenewalDisabled}
              >
                {t("playerProfile.delegateRenewal")}
              </Button>
              <Button className="flex-1" onClick={onSubmit} disabled={renewalSubmitDisabled}>
                {t("playerProfile.renewalSubmit")}
              </Button>
            </>
          )}
        </div>
      </div>
    </Dialog>
  );
}
