import { AlertTriangle } from "lucide-react";
import { useTranslation } from "react-i18next";

import type { UploadReceipt } from "../../services/reportService";
import { ReferenceCode } from "./ReferenceCode";

export function ReportSentStep({ receipt }: { receipt: UploadReceipt }) {
  const { t } = useTranslation();
  return (
    <div className="flex flex-col gap-3">
      <p className="text-[11px] font-heading font-bold uppercase tracking-wider text-gray-500 dark:text-gray-400">
        {t("reportBug.referenceCode")}
      </p>
      <div className="px-3 py-2 rounded-lg bg-gray-50 dark:bg-navy-700 border border-gray-200 dark:border-navy-600">
        <ReferenceCode code={receipt.code} size="lg" />
      </div>
      <p className="text-xs text-gray-600 dark:text-gray-400">{t("reportBug.referenceCodeHint")}</p>
      {receipt.recorded ? (
        <p className="text-xs text-gray-600 dark:text-gray-400">
          {t("reportBug.referenceCodeSaved")}
        </p>
      ) : (
        <p
          role="alert"
          className="flex items-start gap-1.5 text-xs text-accent-700 dark:text-accent-400"
        >
          <AlertTriangle className="w-4 h-4 shrink-0" aria-hidden="true" />
          {t("reportBug.notRecorded")}
        </p>
      )}
    </div>
  );
}
