import { AlertCircle } from "lucide-react";
import type { JSX } from "react";
import { useTranslation } from "react-i18next";

import type { BlockerModal } from "../../hooks/useAdvanceTime.helpers";
import DashboardModalFrame from "./DashboardModalFrame";

interface DashboardBlockerModalProps {
  blockerModal: BlockerModal;
  onClose: () => void;
  onContinueAnyway: (() => void) | null;
  onNavigate: (tab: string) => void;
}

function getBlockerButtonClassName(severity: string): string {
  const baseClassName =
    "w-full rounded-xl border p-3 text-left transition-all hover:shadow-sm";

  if (severity === "warn") {
    return `${baseClassName} border-amber-500/30 bg-amber-500/5 hover:bg-amber-500/10`;
  }

  return `${baseClassName} border-blue-500/30 bg-blue-500/5 hover:bg-blue-500/10`;
}

function getBlockerTextClassName(severity: string): string {
  if (severity === "warn") {
    return "text-sm font-medium text-amber-600 dark:text-amber-400";
  }

  return "text-sm font-medium text-blue-600 dark:text-blue-400";
}

export default function DashboardBlockerModal({
  blockerModal,
  onClose,
  onContinueAnyway,
  onNavigate,
}: DashboardBlockerModalProps): JSX.Element {
  const { t } = useTranslation();

  return (
    <DashboardModalFrame maxWidthClassName="max-w-md">
      <div className="mb-4 flex items-center gap-3">
        <div className="flex h-10 w-10 items-center justify-center rounded-xl bg-amber-500/20">
          <AlertCircle className="h-5 w-5 text-amber-500" />
        </div>
        <div>
          <h3 className="text-lg font-heading font-bold uppercase tracking-wide text-gray-900 dark:text-white">
            {t("notifications.attentionRequired", "Attention Required")}
          </h3>
          <p className="text-xs text-gray-500 dark:text-gray-400">
            {t(
              "notifications.resolveBeforeContinuing",
              "Resolve these issues before continuing",
            )}
          </p>
        </div>
      </div>
      <div className="mb-5 flex flex-col gap-2">
        {blockerModal.blockers.map((blocker) => (
          <button
            key={blocker.id}
            onClick={() => onNavigate(blocker.tab)}
            className={getBlockerButtonClassName(blocker.severity)}
          >
            <p className={getBlockerTextClassName(blocker.severity)}>
              {blocker.text}
            </p>
            <p className="mt-1 text-[10px] font-heading uppercase tracking-widest text-gray-400">
              {t("notifications.goTo", "Go to")} {blocker.tab} →
            </p>
          </button>
        ))}
      </div>
      <div className="flex gap-3">
        <button
          onClick={onClose}
          className="flex-1 rounded-lg bg-gray-100 px-4 py-2.5 text-sm font-heading font-bold uppercase tracking-wider text-gray-700 transition-colors hover:bg-gray-200 dark:bg-navy-700 dark:text-gray-300 dark:hover:bg-navy-600"
        >
          {t("notifications.reviewIssues", "Review Issues")}
        </button>
        {onContinueAnyway && (
          <button
            onClick={onContinueAnyway}
            className="flex-1 rounded-lg bg-amber-500 px-4 py-2.5 text-sm font-heading font-bold uppercase tracking-wider text-white transition-colors hover:bg-amber-600"
          >
            {t("notifications.continueAnyway", "Continue Anyway")}
          </button>
        )}
      </div>
    </DashboardModalFrame>
  );
}
