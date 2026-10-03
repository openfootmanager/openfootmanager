import { useCallback, useEffect, useRef, useState } from "react";
import { useTranslation } from "react-i18next";
import { AlertTriangle } from "lucide-react";

import { formatDate } from "../../lib/dateFormatting";
import { logError } from "../../lib/logger";
import { type CrashNotice, crashNotice, dismissCrashNotice } from "../../services/reportService";
import { Button } from "../ui";
import { useDialogKeyboard } from "./useDialogKeyboard";

interface CrashReportPromptProps {
  /** Open the report dialog. Its preview already lists the crash record as part of the bundle. */
  onReport: () => void;
}

/**
 * After a crash, offer to report it on the next launch.
 *
 * A player whose game vanished rarely thinks to dig out a bug-report form afterwards, and the
 * crash record only survives until this session ends. So the main menu asks, once. Either answer
 * is final for this launch — the backend clears the crash file at startup, so the next launch has
 * nothing to ask about unless the game crashes again.
 */
export function CrashReportPrompt({ onReport }: CrashReportPromptProps) {
  const [notice, setNotice] = useState<CrashNotice | null>(null);

  useEffect(() => {
    let cancelled = false;
    void crashNotice()
      .then((pending) => {
        if (!cancelled) setNotice(pending ?? null);
      })
      .catch((error: unknown) => {
        // A prompt is a courtesy. Failing to offer it must not get in front of the menu.
        logError(`[report] could not read the crash notice: ${String(error)}`);
      });
    return () => {
      cancelled = true;
    };
  }, []);

  const answer = useCallback(
    (report: boolean) => {
      setNotice(null);
      void dismissCrashNotice().catch((error: unknown) => {
        logError(`[report] could not record the crash prompt answer: ${String(error)}`);
      });
      if (report) onReport();
    },
    [onReport],
  );

  if (notice === null) return null;
  return <CrashPromptDialog notice={notice} onAnswer={answer} />;
}

/** Split out so its focus handling starts when the dialog appears, not when the menu mounts. */
function CrashPromptDialog({
  notice,
  onAnswer,
}: {
  notice: CrashNotice;
  onAnswer: (report: boolean) => void;
}) {
  const { t, i18n } = useTranslation();
  const dialogRef = useRef<HTMLDivElement>(null);
  const headingRef = useRef<HTMLHeadingElement>(null);
  const decline = useCallback(() => onAnswer(false), [onAnswer]);

  useDialogKeyboard(dialogRef, decline);
  useEffect(() => {
    headingRef.current?.focus();
  }, []);

  return (
    <div
      className="fixed inset-0 bg-black/50 flex items-center justify-center z-50 p-4"
      role="dialog"
      aria-modal="true"
      aria-labelledby="crash-prompt-title"
      aria-describedby="crash-prompt-body"
    >
      <div
        ref={dialogRef}
        className="bg-white dark:bg-navy-800 rounded-xl shadow-2xl border border-gray-200 dark:border-navy-600 p-6 w-full max-w-md"
      >
        <div className="flex items-start gap-3">
          <span className="text-accent-600 dark:text-accent-400 shrink-0 mt-0.5">
            <AlertTriangle className="w-5 h-5" aria-hidden="true" />
          </span>
          <div className="flex-1 min-w-0">
            <h2
              id="crash-prompt-title"
              ref={headingRef}
              tabIndex={-1}
              className="font-heading font-bold uppercase tracking-wider text-xl text-gray-900 dark:text-gray-100 focus:outline-none"
            >
              {t("crashPrompt.title")}
            </h2>
            <p id="crash-prompt-body" className="mt-2 text-sm text-gray-700 dark:text-gray-300">
              {t("crashPrompt.body")}
            </p>
            {/* Data, not prose: when, and in which build — the two things a player can check. */}
            <p className="mt-2 text-xs font-mono text-gray-600 dark:text-gray-400">
              <time dateTime={notice.occurred_at}>
                {formatDate(notice.occurred_at, i18n.language)}
              </time>
              {notice.app_version !== "" && ` · v${notice.app_version}`}
            </p>
          </div>
        </div>
        <div className="flex gap-3 mt-6">
          <Button className="flex-1" onClick={() => onAnswer(true)}>
            {t("crashPrompt.report")}
          </Button>
          <Button variant="outline" onClick={decline}>
            {t("crashPrompt.notNow")}
          </Button>
        </div>
      </div>
    </div>
  );
}
