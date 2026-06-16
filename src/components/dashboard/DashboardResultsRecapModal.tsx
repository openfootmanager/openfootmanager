import type { JSX } from "react";
import { useTranslation } from "react-i18next";

import { formatMatchDate } from "../../lib/helpers";
import type { AdvanceMatchResultData } from "../../services/advanceTimeService";
import { Badge } from "../ui";
import DashboardModalFrame from "./DashboardModalFrame";

interface DashboardResultsRecapModalProps {
  results: AdvanceMatchResultData[];
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

/**
 * Post-advance recap of the matches played during a Continue / Skip — the
 * user's competitions and national-team fixtures, grouped day by day.
 */
export default function DashboardResultsRecapModal({
  results,
  onClose,
}: DashboardResultsRecapModalProps): JSX.Element {
  const { t } = useTranslation();
  const days = groupByDate(results);

  return (
    <DashboardModalFrame maxWidthClassName="max-w-lg">
      <div className="flex flex-col gap-4">
        <h3 className="text-lg font-heading font-bold uppercase tracking-wide text-gray-900 dark:text-white">
          {t("dashboard.resultsRecapTitle")}
        </h3>

        <div className="flex max-h-[60vh] flex-col gap-4 overflow-y-auto">
          {days.map((day) => (
            <div key={day.date} className="flex flex-col gap-1.5">
              <p className="font-heading text-xs font-bold uppercase tracking-wider text-gray-500 dark:text-gray-400">
                {formatMatchDate(day.date)}
              </p>
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
                    <span className="font-heading font-bold tabular-nums">
                      {match.home_goals} - {match.away_goals}
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
