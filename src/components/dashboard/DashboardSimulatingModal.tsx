import type { JSX } from "react";
import { useEffect, useRef } from "react";
import {
  AlertTriangle,
  Calendar,
  CheckCircle2,
  Loader2,
  Swords,
} from "lucide-react";
import { useTranslation } from "react-i18next";

import type { DigestEntry, DigestStopReason } from "../../hooks/useDigestAdvance";
import type { BlockerData } from "../../services/advanceTimeService";
import DashboardModalFrame from "./DashboardModalFrame";

interface DashboardSimulatingModalProps {
  // Digest-mode props (optional — omit for a plain single-advance spinner)
  digestEntries?: DigestEntry[];
  isDigestRunning?: boolean;
  stopReason?: DigestStopReason | null;
  onDismiss?: () => void;
  onViewBlockers?: (blockers: BlockerData[]) => void;
  onContinueAfterBlocker?: () => void;
}

function DigestDayRow({ entry }: { entry: DigestEntry }): JSX.Element {
  const { t } = useTranslation();
  const { recap } = entry;
  const hasContent =
    recap.matches.length > 0 ||
    recap.transfers.length > 0 ||
    recap.news.length > 0 ||
    recap.inbox.length > 0;

  const formattedDate = new Intl.DateTimeFormat(undefined, {
    day: "numeric",
    month: "short",
  }).format(new Date(`${entry.date}T00:00:00Z`));

  return (
    <div className="border-b border-gray-100 pb-3 last:border-0 dark:border-navy-700">
      <div className="flex items-center gap-1.5 text-xs font-semibold text-gray-500 dark:text-gray-400 mb-1.5">
        <Calendar className="h-3 w-3" />
        {formattedDate}
      </div>

      {!hasContent && (
        <p className="text-xs text-gray-400 dark:text-gray-500 italic">
          {t("dashboard.digestQuietDay")}
        </p>
      )}

      {recap.matches.map((match) => (
        <p
          key={`${match.date}-${match.home_team}-${match.away_team}`}
          className="text-xs text-gray-700 dark:text-gray-300"
        >
          {match.home_team} {match.home_goals}–{match.away_goals} {match.away_team}
        </p>
      ))}

      {recap.transfers.map((transfer) => (
        <p
          key={`${transfer.date}-${transfer.player}`}
          className="text-xs text-gray-700 dark:text-gray-300"
        >
          {transfer.player}: {transfer.from} → {transfer.to}
        </p>
      ))}

      {recap.news.map((article) => (
        <p key={article.id} className="text-xs text-gray-700 dark:text-gray-300">
          {article.textKey ? t(article.textKey, article.params ?? {}) : article.text}
        </p>
      ))}

      {recap.inbox.map((item) => (
        <p
          key={item.id}
          className="text-xs font-medium text-amber-700 dark:text-amber-400"
        >
          {item.textKey ? t(item.textKey, item.params ?? {}) : item.text}
        </p>
      ))}
    </div>
  );
}

export default function DashboardSimulatingModal({
  digestEntries,
  isDigestRunning,
  stopReason,
  onDismiss,
  onViewBlockers,
  onContinueAfterBlocker,
}: DashboardSimulatingModalProps): JSX.Element {
  const { t } = useTranslation();
  const listEndRef = useRef<HTMLDivElement>(null);
  const isDigestMode =
    digestEntries !== undefined && digestEntries.length > 0 ||
    isDigestRunning ||
    stopReason != null;
  const isRunning = isDigestRunning ?? false;

  useEffect(() => {
    listEndRef.current?.scrollIntoView({ behavior: "smooth" });
  }, [digestEntries?.length]);

  // Plain single-advance spinner (no digest data yet)
  if (!isDigestMode) {
    return (
      <DashboardModalFrame maxWidthClassName="max-w-sm">
        <div className="flex flex-col items-center text-center">
          <div className="flex h-14 w-14 items-center justify-center rounded-full bg-primary-100 text-primary-600 dark:bg-primary-500/15 dark:text-primary-300">
            <Loader2 className="h-7 w-7 animate-spin" />
          </div>
          <h3 className="mt-4 text-lg font-heading font-bold uppercase tracking-wide text-gray-900 dark:text-white">
            {t("dashboard.simulating")}
          </h3>
          <p className="mt-2 text-sm text-gray-500 dark:text-gray-400">
            {t("dashboard.simulatingMessage")}
          </p>
        </div>
      </DashboardModalFrame>
    );
  }

  // Digest mode: scrollable feed with action buttons at the bottom
  return (
    <DashboardModalFrame maxWidthClassName="max-w-lg">
      <div className="flex flex-col" style={{ maxHeight: "80vh" }}>
        {/* Header */}
        <div className="flex items-center gap-3 pb-4 border-b border-gray-200 dark:border-navy-700 shrink-0">
          <div className="flex h-10 w-10 items-center justify-center rounded-full bg-primary-100 text-primary-600 dark:bg-primary-500/15 dark:text-primary-300">
            {isRunning ? (
              <Loader2 className="h-5 w-5 animate-spin" />
            ) : (
              <CheckCircle2 className="h-5 w-5" />
            )}
          </div>
          <h3 className="text-base font-heading font-bold uppercase tracking-wide text-gray-900 dark:text-white">
            {isRunning
              ? t("dashboard.digestAdvancing")
              : t("dashboard.digestDone")}
          </h3>
        </div>

        {/* Scrollable event feed */}
        <div className="flex-1 overflow-y-auto py-3 space-y-3 min-h-0">
          {digestEntries && digestEntries.length === 0 && !isRunning && !stopReason && (
            <p className="text-xs text-gray-400 dark:text-gray-500 italic text-center py-4">
              {t("dashboard.digestEmpty")}
            </p>
          )}

          {digestEntries?.map((entry) => (
            <DigestDayRow key={entry.date} entry={entry} />
          ))}

          {isRunning && (
            <div className="flex items-center gap-2 text-xs text-gray-400 dark:text-gray-500 py-1">
              <Loader2 className="h-3 w-3 animate-spin" />
              {t("dashboard.digestSimulating")}
            </div>
          )}

          <div ref={listEndRef} />
        </div>

        {/* Action buttons pinned at the bottom */}
        {stopReason && (
          <div className="border-t border-gray-200 dark:border-navy-700 pt-4 shrink-0">
            {stopReason.kind === "match_day" && (
              <div className="bg-primary-50 rounded-lg px-4 py-3 dark:bg-primary-900/20">
                <div className="flex items-center gap-2 mb-2">
                  <Swords className="h-4 w-4 text-primary-600 dark:text-primary-400" />
                  <span className="text-sm font-semibold text-primary-800 dark:text-primary-300">
                    {t("dashboard.digestMatchDay")}
                  </span>
                </div>
                <p className="text-xs text-primary-700 dark:text-primary-400 mb-3">
                  {t("dashboard.digestMatchDayDesc")}
                </p>
                <button
                  onClick={onDismiss}
                  className="w-full rounded-lg bg-primary-600 px-4 py-2.5 text-sm font-semibold text-white hover:bg-primary-700"
                >
                  {t("dashboard.digestReturnHome")}
                </button>
              </div>
            )}

            {stopReason.kind === "blocked" && (
              <div className="bg-amber-50 rounded-lg px-4 py-3 dark:bg-amber-900/20">
                <div className="flex items-center gap-2 mb-2">
                  <AlertTriangle className="h-4 w-4 text-amber-600 dark:text-amber-400" />
                  <span className="text-sm font-semibold text-amber-800 dark:text-amber-300">
                    {t("dashboard.digestBlocked")}
                  </span>
                </div>
                <p className="text-xs text-amber-700 dark:text-amber-400 mb-3">
                  {t("dashboard.digestBlockedDesc")}
                </p>
                <div className="flex flex-col gap-2">
                  <button
                    onClick={() => onViewBlockers?.(stopReason.blockers)}
                    className="w-full rounded-lg bg-amber-600 px-3 py-2 text-xs font-semibold text-white hover:bg-amber-700"
                  >
                    {t("dashboard.digestViewIssues")}
                  </button>
                  <button
                    onClick={onContinueAfterBlocker}
                    className="w-full rounded-lg border border-amber-400 px-3 py-2 text-xs font-medium text-amber-800 hover:bg-amber-100 dark:border-amber-600 dark:text-amber-300 dark:hover:bg-amber-900/40"
                  >
                    {t("dashboard.digestContinueAnyway")}
                  </button>
                  <button
                    onClick={onDismiss}
                    className="rounded-lg border border-gray-300 px-3 py-2 text-xs font-medium text-gray-700 hover:bg-gray-50 dark:border-navy-600 dark:text-gray-300 dark:hover:bg-navy-700"
                  >
                    {t("dashboard.digestClose")}
                  </button>
                </div>
              </div>
            )}

            {stopReason.kind === "fired" && (
              <div className="bg-red-50 rounded-lg px-4 py-3 dark:bg-red-900/20">
                <p className="text-sm font-semibold text-red-800 dark:text-red-300 mb-3">
                  {t("dashboard.digestFired")}
                </p>
                <button
                  onClick={onDismiss}
                  className="w-full rounded-lg border border-gray-300 px-3 py-2 text-xs font-medium text-gray-700 hover:bg-gray-50 dark:border-navy-600 dark:text-gray-300 dark:hover:bg-navy-700"
                >
                  {t("dashboard.digestClose")}
                </button>
              </div>
            )}
          </div>
        )}
      </div>
    </DashboardModalFrame>
  );
}
