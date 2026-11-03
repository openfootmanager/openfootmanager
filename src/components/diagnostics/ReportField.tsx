import { useTranslation } from "react-i18next";

/** Ties an invalid field to the one error message the step shows. */
export const REQUIRED_ERROR_ID = "report-required-error";

export type ReportFieldKey = "whatHappened" | "expected" | "steps";

interface ReportFieldProps {
  fieldKey: ReportFieldKey;
  rows: number;
  required: boolean;
  value: string;
  invalid: boolean;
  onChange: (value: string) => void;
}

export function ReportField({
  fieldKey,
  rows,
  required,
  value,
  invalid,
  onChange,
}: ReportFieldProps) {
  const { t } = useTranslation();
  return (
    <div>
      <label
        htmlFor={`report-${fieldKey}`}
        className="block text-[11px] font-heading font-bold uppercase tracking-wider text-gray-500 dark:text-gray-400 mb-1.5"
      >
        {t(`reportBug.${fieldKey}`)}
        {required && (
          <span className="ml-2 text-accent-600 dark:text-accent-400 tracking-normal">
            {t("reportBug.required")}
          </span>
        )}
      </label>
      <textarea
        id={`report-${fieldKey}`}
        rows={rows}
        value={value}
        aria-required={required}
        aria-invalid={invalid}
        aria-describedby={invalid ? REQUIRED_ERROR_ID : undefined}
        onChange={(event) => onChange(event.target.value)}
        placeholder={t(`reportBug.${fieldKey}Placeholder`)}
        className={`w-full px-3 py-2 rounded-lg bg-gray-50 dark:bg-navy-700 border text-sm text-gray-800 dark:text-gray-200 focus:outline-none focus:ring-2 focus:ring-primary-500/50 ${
          invalid ? "border-red-500 dark:border-red-500" : "border-gray-200 dark:border-navy-600"
        }`}
      />
    </div>
  );
}
