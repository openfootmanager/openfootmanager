import { useTranslation } from "react-i18next";

import type { Frequency, ReportDraft } from "./ReportBugModal.helpers";
import { REQUIRED_ERROR_ID, ReportField, type ReportFieldKey } from "./ReportField";

const FREQUENCIES: Frequency[] = ["everyTime", "sometimes", "once"];

interface ReportDescribeStepProps {
  draft: ReportDraft;
  onDraftChange: (change: Partial<ReportDraft>) => void;
  showErrors: boolean;
  missing: Array<keyof ReportDraft>;
}

export function ReportDescribeStep({
  draft,
  onDraftChange,
  showErrors,
  missing,
}: ReportDescribeStepProps) {
  const { t } = useTranslation();
  const field = (fieldKey: ReportFieldKey, rows: number, required: boolean) => (
    <ReportField
      fieldKey={fieldKey}
      rows={rows}
      required={required}
      value={draft[fieldKey]}
      invalid={required && showErrors && missing.includes(fieldKey)}
      onChange={(value) => onDraftChange({ [fieldKey]: value })}
    />
  );

  return (
    <div className="flex flex-col gap-4">
      {field("whatHappened", 3, true)}
      {field("expected", 2, true)}
      {field("steps", 3, false)}
      <fieldset className="border-0 p-0 m-0 min-w-0">
        <legend className="block text-[11px] font-heading font-bold uppercase tracking-wider text-gray-500 dark:text-gray-400 mb-1.5">
          {t("reportBug.frequencyLabel")}
        </legend>
        <div className="flex rounded-lg bg-gray-100 dark:bg-navy-700 p-0.5 border border-gray-200 dark:border-navy-600">
          {FREQUENCIES.map((value) => (
            <button
              key={value}
              type="button"
              aria-pressed={draft.frequency === value}
              onClick={() => onDraftChange({ frequency: value })}
              className={`flex-1 px-3 py-1.5 rounded-md text-xs font-heading font-bold uppercase tracking-wider transition-all focus:outline-none focus:ring-2 focus:ring-primary-500 ${
                draft.frequency === value
                  ? "bg-white dark:bg-navy-500 text-primary-600 dark:text-primary-400 shadow-sm"
                  : "text-gray-600 dark:text-gray-400"
              }`}
            >
              {t(`reportBug.frequency.${value}`)}
            </button>
          ))}
        </div>
      </fieldset>
      {showErrors && missing.length > 0 && (
        <p id={REQUIRED_ERROR_ID} role="alert" className="text-xs text-red-600 dark:text-red-400">
          {t("reportBug.fillRequired")}
        </p>
      )}
    </div>
  );
}
