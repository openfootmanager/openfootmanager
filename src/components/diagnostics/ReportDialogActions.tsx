import { Send } from "lucide-react";
import { useTranslation } from "react-i18next";

import { Button } from "../ui";

interface ReportDialogActionsProps {
  step: "describe" | "preview" | "done" | "sent";
  canUpload: boolean;
  busy: boolean;
  canSend: boolean;
  onContinue: () => void;
  onCancel: () => void;
  onSend: () => void;
  onSaveInstead: () => void;
  onBack: () => void;
  onFinish: () => void;
}

export function ReportDialogActions({
  step,
  canUpload,
  busy,
  canSend,
  onContinue,
  onCancel,
  onSend,
  onSaveInstead,
  onBack,
  onFinish,
}: ReportDialogActionsProps) {
  const { t } = useTranslation();
  return (
    <div className="flex gap-3 mt-6">
      {step === "describe" && (
        <>
          <Button className="flex-1" onClick={onContinue}>
            {t("reportBug.review")}
          </Button>
          <Button variant="outline" onClick={onCancel}>
            {t("common.cancel")}
          </Button>
        </>
      )}
      {step === "preview" && !canUpload && (
        <>
          <Button className="flex-1" disabled={busy} onClick={onSaveInstead}>
            {busy ? t("reportBug.working") : t("reportBug.saveAndOpen")}
          </Button>
          <Button variant="outline" disabled={busy} onClick={onBack}>
            {t("common.back")}
          </Button>
        </>
      )}
      {step === "preview" && canUpload && (
        <div className="flex flex-col gap-2 w-full">
          <Button
            className="w-full"
            disabled={busy || !canSend}
            icon={<Send className="w-4 h-4" />}
            onClick={onSend}
          >
            {busy ? t("reportBug.working") : t("reportBug.send")}
          </Button>
          <div className="flex gap-3">
            <Button variant="outline" className="flex-1" disabled={busy} onClick={onSaveInstead}>
              {t("reportBug.saveAndOpenInstead")}
            </Button>
            <Button variant="outline" disabled={busy} onClick={onBack}>
              {t("common.back")}
            </Button>
          </div>
        </div>
      )}
      {(step === "done" || step === "sent") && (
        <Button className="flex-1" onClick={onFinish}>
          {t("common.close")}
        </Button>
      )}
    </div>
  );
}
