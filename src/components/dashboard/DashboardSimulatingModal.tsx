import type { JSX } from "react";
import { useEffect, useRef } from "react";
import {
  AlertTriangle,
  ArrowRightLeft,
  Calendar,
  CheckCircle2,
  Loader2,
  Newspaper,
  Square,
  Swords,
  XCircle,
} from "lucide-react";
import { useTranslation } from "react-i18next";

import type { DigestEntry, DigestStopReason } from "../../hooks/useDigestAdvance";
import type { RecapMatch } from "./advanceRecap";
import { getCategoryIcon } from "../inbox/inboxHelpers";
import { formatVal } from "../../lib/valueFormatting";
import { resolveBackendText } from "../../utils/backendI18n";
import { getBlockerTabLabel } from "../../utils/blockerUtils";
import DashboardModalFrame from "./DashboardModalFrame";

interface DashboardSimulatingModalProps {
  // Digest-mode props (optional — omit for a plain single-advance spinner)
  digestEntries?: DigestEntry[];
  isDigestRunning?: boolean;
  isDigestAborting?: boolean;
  stopReason?: DigestStopReason | null;
  onStop?: () => void;
  onDismiss?: () => void;
  onNavigate?: (tab: string) => void;
  onContinueAfterBlocker?: () => void;
}

function ResultBadge({ result }: { result: RecapMatch["userResult"] }): JSX.Element | null {
  const { t } = useTranslation();
  if (!result) return null;
  const styles = {
    win: "bg-primary-100 text-primary-700 dark:bg-primary-900/40 dark:text-primary-300",
    draw: "bg-gray-100 text-gray-600 dark:bg-navy-700 dark:text-gray-400",
    loss: "bg-red-100 text-red-700 dark:bg-red-900/40 dark:text-red-300",
  } as const;
  const labels = {
    win: t("common.won"),
    draw: t("common.drawn"),
    loss: t("common.lost"),
  } as const;
  return (
    <span className={`shrink-0 text-[10px] font-bold px-1.5 py-0.5 rounded uppercase ${styles[result]}`}>
      {labels[result]}
    </span>
  );
}

function MatchCard({ match, idx }: { match: RecapMatch; idx: number }): JSX.Element {
  return (
    <div
      className="digest-event-item flex items-center gap-2.5 rounded-lg px-3 py-2 bg-primary-50/60 dark:bg-primary-900/10"
      style={{ animationDelay: `${Math.min(idx, 5) * 80}ms` }}
    >
      <div className="flex h-7 w-7 shrink-0 items-center justify-center rounded-full bg-primary-100 text-primary-600 dark:bg-primary-900/40 dark:text-primary-400">
        <Swords className="h-3.5 w-3.5" />
      </div>
      <div className="min-w-0 flex-1">
        <p className="text-xs font-medium text-gray-800 dark:text-gray-200 truncate">
          {match.home_team} {match.home_goals}–{match.away_goals} {match.away_team}
        </p>
        <p className="text-[11px] text-gray-500 dark:text-gray-400 truncate">{match.competition}</p>
      </div>
      {match.involves_user && <ResultBadge result={match.userResult} />}
    </div>
  );
}

function TransferCard({ player, from, to, fee, involvesUser, idx }: {
  player: string;
  from: string;
  to: string;
  fee: number;
  involvesUser?: boolean;
  idx: number;
}): JSX.Element {
  const feeLabel = fee > 0 ? formatVal(fee) : null;
  return (
    <div
      className="digest-event-item flex items-center gap-2.5 rounded-lg px-3 py-2 bg-purple-50/60 dark:bg-purple-900/10"
      style={{ animationDelay: `${Math.min(idx, 5) * 80}ms` }}
    >
      <div className="flex h-7 w-7 shrink-0 items-center justify-center rounded-full bg-purple-100 text-purple-600 dark:bg-purple-900/40 dark:text-purple-400">
        <ArrowRightLeft className="h-3.5 w-3.5" />
      </div>
      <div className="min-w-0 flex-1">
        <p className={`text-xs truncate ${involvesUser ? "font-bold text-primary-600 dark:text-primary-400" : "font-medium text-gray-800 dark:text-gray-200"}`}>
          {player}
        </p>
        <p className="text-[11px] text-gray-500 dark:text-gray-400 truncate">{from} → {to}</p>
      </div>
      {feeLabel && (
        <span className="shrink-0 text-[10px] font-semibold px-1.5 py-0.5 rounded bg-purple-100 text-purple-700 dark:bg-purple-900/40 dark:text-purple-300">
          {feeLabel}
        </span>
      )}
    </div>
  );
}

function NewsCard({ text, idx }: { text: string; idx: number }): JSX.Element {
  return (
    <div
      className="digest-event-item flex items-center gap-2.5 rounded-lg px-3 py-2 bg-gray-50 dark:bg-navy-700/50"
      style={{ animationDelay: `${Math.min(idx, 5) * 80}ms` }}
    >
      <div className="flex h-7 w-7 shrink-0 items-center justify-center rounded-full bg-gray-200 text-gray-500 dark:bg-navy-600 dark:text-gray-400">
        <Newspaper className="h-3.5 w-3.5" />
      </div>
      <p className="min-w-0 flex-1 text-xs text-gray-700 dark:text-gray-300 line-clamp-2">{text}</p>
    </div>
  );
}

function InboxCard({ text, category, idx }: { text: string; category?: string; idx: number }): JSX.Element {
  // Always go through getCategoryIcon so the fallback (System/Info) stays consistent with the inbox
  const icon = getCategoryIcon(category ?? "System");
  return (
    <div
      className="digest-event-item flex items-center gap-2.5 rounded-lg px-3 py-2 bg-amber-50/80 dark:bg-amber-900/15"
      style={{ animationDelay: `${Math.min(idx, 5) * 80}ms` }}
    >
      <div className="flex h-7 w-7 shrink-0 items-center justify-center rounded-full bg-amber-100 text-amber-600 dark:bg-amber-900/40 dark:text-amber-400">
        {icon}
      </div>
      <p className="min-w-0 flex-1 text-xs font-medium text-amber-800 dark:text-amber-300 line-clamp-2">{text}</p>
      <Square className="h-2 w-2 shrink-0 fill-current text-amber-500 dark:text-amber-400" />
    </div>
  );
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

  // Declarative stagger offsets per section to avoid mutable counter in render
  const matchOffset = 0;
  const transferOffset = recap.matches.length;
  const newsOffset = transferOffset + recap.transfers.length;
  const inboxOffset = newsOffset + recap.news.length;

  return (
    <div className="border-b border-gray-100 pb-3 last:border-0 dark:border-navy-700">
      <div className="flex items-center gap-1.5 text-xs font-semibold text-gray-500 dark:text-gray-400 mb-2">
        <Calendar className="h-3 w-3" />
        {formattedDate}
      </div>

      {!hasContent && (
        <p className="text-xs text-gray-400 dark:text-gray-500 italic px-1">
          {t("dashboard.digestQuietDay")}
        </p>
      )}

      <div className="space-y-1">
        {recap.matches.map((match, i) => (
          <MatchCard key={`${match.date}-${match.home_team}-${match.away_team}`} match={match} idx={matchOffset + i} />
        ))}

        {recap.transfers.map((transfer, i) => (
          <TransferCard
            key={`${transfer.date}-${transfer.player}`}
            player={transfer.player}
            from={transfer.from}
            to={transfer.to}
            fee={transfer.fee}
            involvesUser={transfer.involvesUser}
            idx={transferOffset + i}
          />
        ))}

        {recap.news.map((article, i) => (
          <NewsCard
            key={article.id}
            text={article.textKey ? t(article.textKey, article.params ?? {}) : article.text}
            idx={newsOffset + i}
          />
        ))}

        {recap.inbox.map((item, i) => (
          <InboxCard
            key={item.id}
            text={item.textKey ? t(item.textKey, item.params ?? {}) : item.text}
            category={item.category}
            idx={inboxOffset + i}
          />
        ))}
      </div>
    </div>
  );
}

export default function DashboardSimulatingModal({
  digestEntries,
  isDigestRunning,
  isDigestAborting,
  stopReason,
  onStop,
  onDismiss,
  onNavigate,
  onContinueAfterBlocker,
}: DashboardSimulatingModalProps): JSX.Element {
  const { t } = useTranslation();
  const listEndRef = useRef<HTMLDivElement>(null);
  const isDigestMode =
    digestEntries !== undefined ||
    isDigestRunning === true ||
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
    <DashboardModalFrame maxWidthClassName="max-w-2xl">
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
          <h3 className="flex-1 text-base font-heading font-bold uppercase tracking-wide text-gray-900 dark:text-white">
            {isRunning
              ? t("dashboard.digestAdvancing")
              : t("dashboard.digestDone")}
          </h3>
          {isRunning && onStop && (
            <button
              type="button"
              onClick={onStop}
              disabled={isDigestAborting}
              className="flex items-center gap-1.5 rounded-lg border border-gray-300 px-3 py-1.5 text-xs font-medium text-gray-700 hover:bg-gray-50 disabled:opacity-60 disabled:cursor-not-allowed dark:border-navy-600 dark:text-gray-300 dark:hover:bg-navy-700"
            >
              {isDigestAborting ? (
                <Loader2 className="h-3.5 w-3.5 animate-spin" />
              ) : (
                <XCircle className="h-3.5 w-3.5" />
              )}
              {isDigestAborting ? t("dashboard.digestStopping") : t("dashboard.digestStop")}
            </button>
          )}
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

        {/* Close button when digest finished with no specific stop reason (natural end or user-aborted) */}
        {!isRunning && !stopReason && digestEntries && digestEntries.length > 0 && (
          <div className="border-t border-gray-200 dark:border-navy-700 pt-4 shrink-0">
            <button
              type="button"
              onClick={onDismiss}
              className="w-full rounded-lg border border-gray-300 px-4 py-2 text-sm font-medium text-gray-700 hover:bg-gray-50 dark:border-navy-600 dark:text-gray-300 dark:hover:bg-navy-700"
            >
              {t("common.close")}
            </button>
          </div>
        )}

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
                  type="button"
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
                {stopReason.blockers.length > 0 && (
                  <div className="mb-3 flex flex-col gap-1.5">
                    {stopReason.blockers.map((blocker) => (
                      <button
                        key={blocker.id}
                        type="button"
                        onClick={() => { onDismiss?.(); onNavigate?.(blocker.tab); }}
                        className="w-full rounded-lg border border-amber-500/30 bg-amber-500/5 px-3 py-2 text-left hover:bg-amber-500/10 transition-colors"
                      >
                        <p className="text-xs font-medium text-amber-700 dark:text-amber-300">
                          {resolveBackendText(blocker.text_key, blocker.text, blocker.text_params)}
                        </p>
                        <p className="mt-0.5 text-[10px] font-heading uppercase tracking-widest text-amber-600/70 dark:text-amber-400/70">
                          {t("notifications.goTo")} {getBlockerTabLabel(t, blocker.tab)} →
                        </p>
                      </button>
                    ))}
                  </div>
                )}
                <div className="flex flex-col gap-2">
                  <button
                    type="button"
                    onClick={onContinueAfterBlocker}
                    className="w-full rounded-lg bg-amber-600 px-3 py-2 text-xs font-semibold text-white hover:bg-amber-700"
                  >
                    {t("dashboard.digestContinueAnyway")}
                  </button>
                  <button
                    type="button"
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
                  type="button"
                  onClick={onDismiss}
                  className="w-full rounded-lg border border-gray-300 px-3 py-2 text-xs font-medium text-gray-700 hover:bg-gray-50 dark:border-navy-600 dark:text-gray-300 dark:hover:bg-navy-700"
                >
                  {t("dashboard.digestClose")}
                </button>
              </div>
            )}

            {stopReason.kind === "stopped" && (
              <div className="bg-gray-50 rounded-lg px-4 py-3 dark:bg-navy-700/60">
                <p className="text-sm font-semibold text-gray-700 dark:text-gray-300 mb-3">
                  {t("dashboard.digestStopped")}
                </p>
                <button
                  type="button"
                  onClick={onDismiss}
                  className="w-full rounded-lg border border-gray-300 px-3 py-2 text-xs font-medium text-gray-700 hover:bg-gray-50 dark:border-navy-600 dark:text-gray-300 dark:hover:bg-navy-700"
                >
                  {t("dashboard.digestClose")}
                </button>
              </div>
            )}

            {stopReason.kind === "error" && (
              <div className="bg-gray-50 rounded-lg px-4 py-3 dark:bg-navy-700/60">
                <p className="text-sm font-semibold text-gray-700 dark:text-gray-300 mb-3">
                  {t("dashboard.digestError")}
                </p>
                <button
                  type="button"
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
