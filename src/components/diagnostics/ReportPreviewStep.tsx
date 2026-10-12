import { AlertTriangle, FileText, Package } from "lucide-react";
import type { ChangeEvent } from "react";
import { useTranslation } from "react-i18next";

import type { DiagnosticsReport, UploadFailureKind } from "../../services/reportService";
import { CheckboxRow } from "./CheckboxRow";
import { PreviewRow } from "./PreviewRow";
import { formatAppVersion } from "../../lib/appVersion";
import { describeMachine, formatBytes } from "./ReportBugModal.helpers";

interface ReportPreviewStepProps {
  diagnostics: DiagnosticsReport | null;
  careerLine: string;
  logBytes: number | null;
  canAttachSave: boolean;
  includeSave: boolean;
  onIncludeSaveChange: (checked: boolean) => void;
  busy: boolean;
  canUpload: boolean;
  consent: boolean;
  canConsent: boolean;
  onConsentChange: (checked: boolean) => void;
  failure: string | null;
  failureKind: UploadFailureKind | null;
}

export function ReportPreviewStep({
  diagnostics,
  careerLine,
  logBytes,
  canAttachSave,
  includeSave,
  onIncludeSaveChange,
  busy,
  canUpload,
  consent,
  canConsent,
  onConsentChange,
  failure,
  failureKind,
}: ReportPreviewStepProps) {
  const { t } = useTranslation();
  const saveSize =
    canAttachSave && diagnostics !== null && diagnostics.save_bytes !== null ? (
      <span className="text-[11px] font-mono text-gray-600 dark:text-gray-400 shrink-0">
        {formatBytes(diagnostics.save_bytes)}
      </span>
    ) : undefined;

  return (
    <div className="flex flex-col gap-2">
      <PreviewRow
        icon={<FileText className="w-4 h-4" />}
        name={t("reportBug.itemWhatYouWrote")}
        detail={t("reportBug.itemWhatYouWroteDesc")}
      />
      <PreviewRow
        icon={<Package className="w-4 h-4" />}
        name={t("reportBug.itemSetup")}
        detail={[formatAppVersion(), describeMachine(diagnostics), careerLine]
          .filter((part) => part !== "")
          .join(" · ")}
      />
      <PreviewRow
        icon={<FileText className="w-4 h-4" />}
        name={t("reportBug.itemLogs")}
        detail={t("reportBug.itemLogsDesc")}
        size={logBytes === null ? undefined : formatBytes(logBytes)}
        files={diagnostics?.log_files}
      />
      {diagnostics?.crash_on_previous_run && (
        <PreviewRow
          icon={<AlertTriangle className="w-4 h-4" />}
          name={t("reportBug.itemCrash")}
          detail={t("reportBug.itemCrashDesc")}
        />
      )}

      <div className="h-px bg-gray-200 dark:bg-navy-600 my-1.5" />
      <p className="text-[11px] font-heading font-bold uppercase tracking-wider text-gray-500 dark:text-gray-400">
        {t("reportBug.optionalHeading")}
      </p>
      <CheckboxRow
        id="report-include-save"
        checked={canAttachSave && includeSave}
        disabled={!canAttachSave || busy}
        label={t("reportBug.includeSave")}
        description={
          canAttachSave ? t("reportBug.includeSaveDesc") : t("reportBug.includeSaveNoCareer")
        }
        onChange={(event: ChangeEvent<HTMLInputElement>) =>
          onIncludeSaveChange(event.target.checked)
        }
        trailing={saveSize}
      />

      {canUpload && (
        <>
          <div className="h-px bg-gray-200 dark:bg-navy-600 my-1.5" />
          <CheckboxRow
            id="report-consent"
            checked={consent}
            disabled={!canConsent}
            label={t("reportBug.consentLabel")}
            description={t("reportBug.consentDesc")}
            onChange={(event: ChangeEvent<HTMLInputElement>) =>
              onConsentChange(event.target.checked)
            }
          />
        </>
      )}

      <p className="text-[11px] text-gray-600 dark:text-gray-400 mt-2">
        {canUpload ? t("reportBug.nothingSentYetUpload") : t("reportBug.nothingSentYet")}
      </p>
      {failure && (
        <div role="alert" className="flex flex-col gap-1 text-xs text-red-600 dark:text-red-400">
          <p>{failure}</p>
          {failureKind === "tooLarge" && canAttachSave && includeSave && (
            <p>{t("reportBug.tooLargeUntickSave")}</p>
          )}
          {failureKind !== null && <p>{t("reportBug.uploadFailedFallback")}</p>}
        </div>
      )}
    </div>
  );
}
