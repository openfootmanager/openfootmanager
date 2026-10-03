import { useEffect, useState } from "react";
import { useTranslation } from "react-i18next";

import { formatDate } from "../../lib/dateFormatting";
import { logError } from "../../lib/logger";
import { type SubmittedReport, listSubmittedReports } from "../../services/reportService";

interface SubmittedReportsProps {
  /** Changed by the parent after a report may have been sent, so a new code shows up at once. */
  refreshKey: number;
}

/**
 * "Your reports" in Settings → Help: the reference codes this install has been given.
 *
 * The codes come from `reports.json` and nothing else. There is deliberately no status beside them
 * — the relay has no lookup endpoint, because a code is an identifier to quote to the maintainer,
 * not a credential — so the list is a record of what was sent, newest first, and when.
 */
export function SubmittedReports({ refreshKey }: SubmittedReportsProps) {
  const { t, i18n } = useTranslation();
  // `null` while loading, so the empty-state sentence is not flashed at a player who has reports.
  const [reports, setReports] = useState<SubmittedReport[] | null>(null);

  useEffect(() => {
    let cancelled = false;
    void listSubmittedReports()
      .then((listed) => {
        if (!cancelled) setReports(listed);
      })
      .catch((error: unknown) => {
        logError(`[report] could not list submitted reports: ${String(error)}`);
        if (!cancelled) setReports([]);
      });
    return () => {
      cancelled = true;
    };
  }, [refreshKey]);

  return (
    <div>
      <p className="text-sm font-medium text-gray-800 dark:text-gray-200">
        {t("settings.yourReports")}
      </p>
      <p className="text-xs text-gray-500 dark:text-gray-400 mt-0.5">
        {t("settings.yourReportsDesc")}
      </p>
      {reports !== null && reports.length === 0 && (
        <p className="mt-2 text-xs italic text-gray-500 dark:text-gray-400">
          {t("settings.noReports")}
        </p>
      )}
      {reports !== null && reports.length > 0 && (
        <ul className="mt-2 flex flex-col gap-1">
          {reports.map((report) => (
            <li
              key={report.code}
              className="flex items-center gap-3 px-3 py-2 rounded-lg bg-gray-50 dark:bg-navy-700 border border-gray-200 dark:border-navy-600"
            >
              {/* `select-all`: the code exists to be quoted, so one click should take all of it. */}
              <code className="select-all font-mono text-sm tracking-widest text-gray-900 dark:text-gray-100">
                {report.code}
              </code>
              <span className="text-xs text-gray-600 dark:text-gray-400">
                {formatDate(report.submitted_at, i18n.language)}
              </span>
              {report.included_save && (
                <span className="ml-auto text-[10px] font-heading font-bold uppercase tracking-wider text-accent-700 dark:text-accent-400">
                  {t("settings.reportIncludedSave")}
                </span>
              )}
            </li>
          ))}
        </ul>
      )}
    </div>
  );
}
