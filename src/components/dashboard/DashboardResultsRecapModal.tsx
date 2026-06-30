import type { JSX } from "react";
import { useTranslation } from "react-i18next";

import { formatMatchDate } from "../../lib/helpers";
import { formatVal } from "../../lib/valueFormatting";
import type { AdvanceMatchResultData } from "../../services/advanceTimeService";
import { Badge } from "../ui";
import type { AdvanceRecap, RecapHeadline, RecapTransfer } from "./advanceRecap";
import DashboardModalFrame from "./DashboardModalFrame";

interface DashboardResultsRecapModalProps {
  recap: AdvanceRecap;
  onClose: () => void;
}

interface DayGroup {
  date: string;
  matches: AdvanceMatchResultData[];
}

/** Group consecutive results by date (the list arrives already date-sorted). */
function groupByDate(results: AdvanceMatchResultData[]): DayGroup[] {
  const groups: DayGroup[] = [];
  for (const match of results) {
    const last = groups[groups.length - 1];
    if (last && last.date === match.date) {
      last.matches.push(match);
    } else {
      groups.push({ date: match.date, matches: [match] });
    }
  }
  return groups;
}

function SectionHeading({ children }: { children: string }): JSX.Element {
  return (
    <p className="font-heading text-xs font-bold uppercase tracking-wider text-gray-500 dark:text-gray-400">
      {children}
    </p>
  );
}

/**
 * Post-advance recap of what happened during a Continue / Skip: the new date,
 * the user's match results day by day, and the notable events (transfers, key
 * news, high-priority inbox items) — so even quiet, no-match days have context.
 */
export default function DashboardResultsRecapModal({
  recap,
  onClose,
}: DashboardResultsRecapModalProps): JSX.Element {
  const { t } = useTranslation();
  const days = groupByDate(recap.matches);

  const resolveHeadline = (headline: RecapHeadline): string =>
    headline.textKey
      ? t(headline.textKey, { defaultValue: headline.text, ...headline.params })
      : headline.text;

  const renderTransfer = (transfer: RecapTransfer, index: number): JSX.Element => (
    <div
      key={`${transfer.date}-${index}`}
      className={`flex items-center justify-between gap-3 py-1.5 text-sm ${
        transfer.involvesUser
          ? "font-bold text-primary-600 dark:text-primary-400"
          : "text-gray-700 dark:text-gray-300"
      }`}
    >
      <span className="truncate">
        {t("dashboard.recapTransferLine", {
          player: transfer.player,
          from: transfer.from,
          to: transfer.to,
        })}
      </span>
      <span className="shrink-0 font-heading font-bold tabular-nums">
        {formatVal(transfer.fee)}
      </span>
    </div>
  );

  const renderHeadline = (headline: RecapHeadline): JSX.Element => (
    <p
      key={headline.id}
      className="truncate py-1.5 text-sm text-gray-700 dark:text-gray-300"
    >
      {resolveHeadline(headline)}
    </p>
  );

  return (
    <DashboardModalFrame maxWidthClassName="max-w-lg">
      <div className="flex flex-col gap-4">
        <div className="flex flex-col gap-0.5">
          <h3 className="text-lg font-heading font-bold uppercase tracking-wide text-gray-900 dark:text-white">
            {t("dashboard.resultsRecapTitle")}
          </h3>
          {recap.advancedTo && (
            <p className="text-sm text-gray-500 dark:text-gray-400">
              {t("dashboard.recapAdvancedTo", {
                date: formatMatchDate(recap.advancedTo),
              })}
            </p>
          )}
        </div>

        <div className="flex max-h-[60vh] flex-col gap-4 overflow-y-auto">
          {days.map((day) => (
            <div key={day.date} className="flex flex-col gap-1.5">
              <SectionHeading>{formatMatchDate(day.date)}</SectionHeading>
              <div className="flex flex-col divide-y divide-gray-100 dark:divide-navy-700">
                {day.matches.map((match, index) => (
                  <div
                    key={`${day.date}-${index}`}
                    className={`flex items-center gap-3 py-1.5 text-sm ${
                      match.involves_user
                        ? "font-bold text-primary-600 dark:text-primary-400"
                        : "text-gray-700 dark:text-gray-300"
                    }`}
                  >
                    <span className="flex-1 truncate text-right">
                      {match.home_team}
                    </span>
                    <span className="flex flex-col items-center font-heading font-bold tabular-nums">
                      <span>
                        {match.home_goals} - {match.away_goals}
                      </span>
                      {match.home_penalties != null &&
                        match.away_penalties != null && (
                          <span className="text-[10px] font-semibold text-gray-500 dark:text-gray-400">
                            {t("match.shootout.shootoutScore", {
                              h: match.home_penalties,
                              a: match.away_penalties,
                            })}
                          </span>
                        )}
                    </span>
                    <span className="flex flex-1 items-center gap-1.5 truncate text-left">
                      {match.away_team}
                      {match.international && (
                        <Badge variant="neutral" size="sm">
                          {t("schedule.international")}
                        </Badge>
                      )}
                    </span>
                  </div>
                ))}
              </div>
            </div>
          ))}

          {recap.transfers.length > 0 && (
            <div className="flex flex-col gap-1.5">
              <SectionHeading>{t("dashboard.recapSectionTransfers")}</SectionHeading>
              <div className="flex flex-col divide-y divide-gray-100 dark:divide-navy-700">
                {recap.transfers.map(renderTransfer)}
              </div>
            </div>
          )}

          {recap.news.length > 0 && (
            <div className="flex flex-col gap-1.5">
              <SectionHeading>{t("dashboard.recapSectionNews")}</SectionHeading>
              <div className="flex flex-col divide-y divide-gray-100 dark:divide-navy-700">
                {recap.news.map(renderHeadline)}
              </div>
            </div>
          )}

          {recap.inbox.length > 0 && (
            <div className="flex flex-col gap-1.5">
              <SectionHeading>{t("dashboard.recapSectionInbox")}</SectionHeading>
              <div className="flex flex-col divide-y divide-gray-100 dark:divide-navy-700">
                {recap.inbox.map(renderHeadline)}
              </div>
            </div>
          )}

          {!recap.hasEvents && (
            <p className="py-4 text-center text-sm text-gray-500 dark:text-gray-400">
              {t("dashboard.recapNothingNotable")}
            </p>
          )}
        </div>

        <button
          onClick={onClose}
          className="mt-1 w-full rounded-lg bg-primary-500 px-4 py-2 font-heading text-sm font-bold uppercase tracking-wider text-white transition-colors hover:bg-primary-600"
        >
          {t("dashboard.resultsRecapDismiss")}
        </button>
      </div>
    </DashboardModalFrame>
  );
}
