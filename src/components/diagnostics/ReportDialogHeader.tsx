import { Bug, Check, Eye, X } from "lucide-react";
import type { RefObject } from "react";
import { useTranslation } from "react-i18next";

interface ReportDialogHeaderProps {
  step: "describe" | "preview" | "done" | "sent";
  canUpload: boolean;
  closeDisabled: boolean;
  onClose: () => void;
  headingRef: RefObject<HTMLHeadingElement | null>;
}

export function ReportDialogHeader({
  step,
  canUpload,
  closeDisabled,
  onClose,
  headingRef,
}: ReportDialogHeaderProps) {
  const { t } = useTranslation();
  return (
    <div className="flex items-start gap-3 mb-5">
      <span className="text-primary-600 dark:text-primary-400 shrink-0 mt-0.5">
        {step === "done" || step === "sent" ? (
          <Check className="w-5 h-5" />
        ) : step === "preview" ? (
          <Eye className="w-5 h-5" />
        ) : (
          <Bug className="w-5 h-5" />
        )}
      </span>
      <div className="flex-1 min-w-0">
        <h2
          id="report-bug-title"
          ref={headingRef}
          tabIndex={-1}
          className="font-heading font-bold uppercase tracking-wider text-xl text-gray-900 dark:text-gray-100 focus:outline-none"
        >
          {t(`reportBug.${step}Title`)}
        </h2>
        <p className="mt-1 text-xs text-gray-600 dark:text-gray-400">
          {step === "preview" && canUpload
            ? t("reportBug.previewIntroUpload")
            : t(`reportBug.${step}Intro`)}
        </p>
      </div>
      <button
        type="button"
        onClick={onClose}
        disabled={closeDisabled}
        aria-label={t("common.close")}
        className="p-1 rounded-lg text-gray-500 dark:text-gray-400 hover:bg-gray-100 dark:hover:bg-navy-700 disabled:opacity-50 disabled:cursor-not-allowed disabled:hover:bg-transparent dark:disabled:hover:bg-transparent focus:outline-none focus:ring-2 focus:ring-primary-500 focus:ring-offset-2 dark:focus:ring-offset-navy-800"
      >
        <X className="w-4 h-4" />
      </button>
    </div>
  );
}
