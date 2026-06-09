import type { JSX } from "react";
import { useTranslation } from "react-i18next";

import { formatExactMoney } from "../../lib/helpers";
import type { MessageData } from "../../store/gameStore";
import { resolveBackendText } from "../../utils/backendI18n";
import { renderMessageBodyLine } from "./inboxHelpers";

interface InboxDelegatedRenewalReportProps {
  message: MessageData;
  onPlayerClick?: (playerId: string) => void;
}

export default function InboxDelegatedRenewalReport({
  message,
  onPlayerClick,
}: InboxDelegatedRenewalReportProps): JSX.Element | null {
  const { t } = useTranslation();
  const report = message.context?.delegated_renewal_report;

  const formatMoneyParam = (value?: string | number | null): string => {
    const amount = Number(value ?? 0);

    if (!Number.isFinite(amount)) {
      return String(value ?? 0);
    }

    return formatExactMoney(amount);
  };

  if (!report || report.cases.length === 0) {
    return null;
  }

  return (
    <div
      className="mt-6 rounded-xl border border-gray-100 bg-gray-50 p-4 dark:border-navy-600 dark:bg-navy-700"
      data-testid="delegated-renewal-report"
    >
      <div className="space-y-2">
        {report.cases.map((renewalCase, index) => {
          const detail = resolveBackendText(
            renewalCase.note_key,
            "",
            renewalCase.note_params,
          );
          const formattedWage = formatMoneyParam(renewalCase.agreed_wage);

          const line =
            renewalCase.status === "successful"
              ? resolveBackendText(
                "be.msg.delegatedRenewals.case.successful",
                `Completed: ${renewalCase.player_name} agreed to ${String(renewalCase.agreed_years ?? 0)} year(s) on ${formattedWage}/wk.`,
                {
                  player: renewalCase.player_name,
                  years: String(renewalCase.agreed_years ?? 0),
                  wage: String(renewalCase.agreed_wage ?? 0),
                },
              )
              : renewalCase.status === "stalled"
                ? resolveBackendText(
                  "be.msg.delegatedRenewals.case.stalled",
                  `Still difficult: ${renewalCase.player_name} - ${detail}`,
                  {
                    player: renewalCase.player_name,
                    detail,
                  },
                )
                : resolveBackendText(
                  "be.msg.delegatedRenewals.case.failed",
                  `Failed: ${renewalCase.player_name} - ${detail}`,
                  {
                    player: renewalCase.player_name,
                    detail,
                  },
                );

          return (
            <div
              key={`${renewalCase.player_id}-${index}`}
              className="flex flex-wrap items-start justify-between gap-2"
            >
              <div className="min-w-0 flex-1">
                {renderMessageBodyLine(`• ${line}`, index)}
              </div>
              {onPlayerClick ? (
                <>
                  <span
                    id={`delegated-renewal-profile-${renewalCase.player_id}`}
                    className="sr-only"
                  >
                    {renewalCase.player_name}
                  </span>
                  <button
                    type="button"
                    aria-describedby={`delegated-renewal-profile-${renewalCase.player_id}`}
                    className="text-xs font-heading font-bold uppercase tracking-wider text-primary-500 hover:text-primary-600 dark:text-primary-400 dark:hover:text-primary-300"
                    onClick={() => onPlayerClick(renewalCase.player_id)}
                  >
                    {t("squad.viewProfile")}
                  </button>
                </>
              ) : null}
            </div>
          );
        })}
      </div>
    </div>
  );
}
