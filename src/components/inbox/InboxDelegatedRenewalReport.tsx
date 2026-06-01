import type { JSX } from "react";

import { formatExactMoney } from "../../lib/helpers";
import type { MessageData } from "../../store/gameStore";
import { resolveBackendText } from "../../utils/backendI18n";
import { renderMessageBodyLine } from "./inboxHelpers";

interface InboxDelegatedRenewalReportProps {
  message: MessageData;
}

export default function InboxDelegatedRenewalReport({
  message,
}: InboxDelegatedRenewalReportProps): JSX.Element | null {
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
                  `Still difficult: ${renewalCase.player_name} — ${detail}`,
                  {
                    player: renewalCase.player_name,
                    detail,
                  },
                )
                : resolveBackendText(
                  "be.msg.delegatedRenewals.case.failed",
                  `Failed: ${renewalCase.player_name} — ${detail}`,
                  {
                    player: renewalCase.player_name,
                    detail,
                  },
                );

          return renderMessageBodyLine(`• ${line}`, index);
        })}
      </div>
    </div>
  );
}
