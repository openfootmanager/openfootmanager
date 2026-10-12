import { useCallback, useEffect, useMemo, useRef, useState } from "react";
import { useTranslation } from "react-i18next";

import { logError } from "../../lib/logger";
import {
  type DiagnosticsReport,
  collectDiagnostics,
  reportUploadAvailable,
} from "../../services/reportService";
import { useGameStore } from "../../store/gameStore";
import {
  EMPTY_DRAFT,
  type ReportDraft,
  composeReportText,
  describeCareer,
  missingRequiredFields,
} from "./ReportBugModal.helpers";
import { ReportDescribeStep } from "./ReportDescribeStep";
import { ReportDialogActions } from "./ReportDialogActions";
import { ReportDialogHeader } from "./ReportDialogHeader";
import { ReportDoneStep } from "./ReportDoneStep";
import { ReportPreviewStep } from "./ReportPreviewStep";
import { ReportSentStep } from "./ReportSentStep";
import { useDialogKeyboard } from "./useDialogKeyboard";
import { useReportSubmission } from "./useReportSubmission";

type Step = "describe" | "preview" | "done" | "sent";

interface ReportBugModalProps {
  onClose: () => void;
  /** Called when a report is accepted, even if the dialog was closed while it was being sent. */
  onUploaded?: () => void;
}

/** Whether OFMHub takes reports, and what this machine is; both fail closed. */
function useReportEnvironment() {
  const [uploadAvailable, setUploadAvailable] = useState<boolean | null>(null);
  const [diagnostics, setDiagnostics] = useState<DiagnosticsReport | null>(null);

  useEffect(() => {
    let cancelled = false;
    void reportUploadAvailable()
      .then((available) => {
        if (!cancelled) setUploadAvailable(available);
      })
      .catch((error: unknown) => {
        logError(`[report] could not tell whether uploads are available: ${String(error)}`);
        if (!cancelled) setUploadAvailable(false);
      });
    void collectDiagnostics()
      .then((report) => {
        if (!cancelled) setDiagnostics(report);
      })
      .catch((error: unknown) => {
        // Not fatal: the report is still worth sending without the machine summary.
        logError(`[report] could not collect diagnostics: ${String(error)}`);
      });
    return () => {
      cancelled = true;
    };
  }, []);

  return { uploadAvailable, diagnostics };
}

/**
 * Report a bug without leaving the game.
 *
 * Describe, then preview: the player sees every file and field that is about to leave their machine,
 * with sizes, before anything is written or sent. The preview is also the consent screen. The
 * prefilled GitHub form needs no server, so it stays as the path for a player who declines and the
 * fallback whenever an upload fails.
 */

export function ReportBugModal({ onClose, onUploaded }: ReportBugModalProps) {
  const { t } = useTranslation();
  const [step, setStep] = useState<Step>("describe");
  const [draft, setDraft] = useState<ReportDraft>(EMPTY_DRAFT);
  const [showErrors, setShowErrors] = useState(false);
  const [includeSave, setIncludeSave] = useState(false);
  const [consent, setConsent] = useState(false);
  const headingRef = useRef<HTMLHeadingElement>(null);
  const dialogRef = useRef<HTMLDivElement>(null);
  const { uploadAvailable, diagnostics } = useReportEnvironment();

  // Where the player was when it broke. The backend deliberately does not duplicate this: the
  // career lives here, and a second copy across the IPC boundary can disagree.
  const gameState = useGameStore((state) => state.gameState);
  const careerLine = useMemo(() => describeCareer(gameState), [gameState]);
  const missing = useMemo(() => missingRequiredFields(draft), [draft]);
  const logBytes = useMemo(
    () =>
      diagnostics === null
        ? null
        : diagnostics.log_files.reduce((total, file) => total + file.bytes, 0),
    [diagnostics],
  );

  // Fails closed: offering to attach a save we cannot confirm exists is the same mistake as
  // denying one that does. The export reads the same `get_save_id()` behind this flag.
  const canAttachSave = diagnostics?.has_active_save === true;

  const reportText = () =>
    composeReportText(
      draft,
      {
        whatHappened: t("reportBug.whatHappened"),
        expected: t("reportBug.expected"),
        steps: t("reportBug.steps"),
        frequencyLabel: t("reportBug.frequencyLabel"),
        frequency: t(`reportBug.frequency.${draft.frequency}`),
        career: t("reportBug.gameContext"),
      },
      careerLine,
    );

  const submission = useReportSubmission(
    {
      draft,
      careerLine,
      reportText,
      includeSaveInBundle: canAttachSave && includeSave,
      consent,
      diagnostics,
      onUploaded,
    },
    { onSent: () => setStep("sent"), onExported: () => setStep("done") },
  );
  const { working } = submission;
  const busy = working !== null;
  const canUpload = uploadAvailable === true;
  // Consent is to a list of files, so it cannot be given before that list has been read and shown.
  const canConsent = diagnostics !== null && !busy;

  // Leaving during an export would let it finish and open the browser on a report the player had
  // just dismissed. An upload is safe to leave: the backend finishes it and records the code.
  const requestClose = useCallback(() => {
    if (working === "export") return;
    onClose();
  }, [working, onClose]);
  useDialogKeyboard(dialogRef, requestClose);

  // On every step, so the new title is announced and the next Tab starts inside the dialog.
  useEffect(() => {
    headingRef.current?.focus();
  }, [step]);

  // Agreeing to send one bundle is not agreeing to send another: anything that changes what would
  // be sent takes the tick away.
  const changeIncludeSave = (checked: boolean) => {
    setIncludeSave(checked);
    setConsent(false);
  };
  useEffect(() => {
    setConsent(false);
  }, [diagnostics]);

  const handleContinue = () => {
    setShowErrors(missing.length > 0);
    if (missing.length === 0) setStep("preview");
  };

  const handleBack = () => {
    setConsent(false);
    submission.clearFailure();
    setStep("describe");
  };

  return (
    // `aria-busy` because the export refuses Escape and the close button: without it the refusal
    // is silent to a screen reader, and the dialog just appears to stop responding.
    <div
      className="fixed inset-0 bg-black/50 flex items-center justify-center z-50 p-4"
      role="dialog"
      aria-modal="true"
      aria-labelledby="report-bug-title"
      aria-busy={busy}
    >
      <div
        ref={dialogRef}
        className="bg-white dark:bg-navy-800 rounded-xl shadow-2xl border border-gray-200 dark:border-navy-600 p-6 w-full max-w-lg max-h-[90vh] overflow-y-auto"
      >
        <ReportDialogHeader
          step={step}
          canUpload={canUpload}
          closeDisabled={working === "export"}
          onClose={requestClose}
          headingRef={headingRef}
        />

        {step === "describe" && (
          <ReportDescribeStep
            draft={draft}
            onDraftChange={(change) => setDraft((prev) => ({ ...prev, ...change }))}
            showErrors={showErrors}
            missing={missing}
          />
        )}
        {step === "preview" && (
          <ReportPreviewStep
            diagnostics={diagnostics}
            careerLine={careerLine}
            logBytes={logBytes}
            canAttachSave={canAttachSave}
            includeSave={includeSave}
            onIncludeSaveChange={changeIncludeSave}
            busy={busy}
            canUpload={canUpload}
            consent={consent}
            canConsent={canConsent}
            onConsentChange={setConsent}
            failure={submission.failure}
            failureKind={submission.failureKind}
          />
        )}
        {step === "sent" && submission.receipt && <ReportSentStep receipt={submission.receipt} />}
        {step === "done" && submission.summary && (
          <ReportDoneStep summary={submission.summary} browserFailed={submission.browserFailed} />
        )}

        <ReportDialogActions
          step={step}
          canUpload={canUpload}
          busy={busy}
          canSend={consent}
          onContinue={handleContinue}
          onCancel={requestClose}
          onSend={() => void submission.upload()}
          onSaveInstead={() => void submission.exportAndOpenForm()}
          onBack={handleBack}
          onFinish={onClose}
        />
      </div>
    </div>
  );
}
