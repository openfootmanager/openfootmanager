import { save } from "@tauri-apps/plugin-dialog";
import { openUrl } from "@tauri-apps/plugin-opener";
import { useRef, useState } from "react";
import { useTranslation } from "react-i18next";

import { formatAppVersion } from "../../lib/appVersion";
import { buildBugReportUrl } from "../../lib/issueUrl";
import { logError } from "../../lib/logger";
import {
  type BundleSummary,
  type DiagnosticsReport,
  type UploadFailureKind,
  type UploadReceipt,
  exportReportBundle,
  redactReportFields,
  suggestedReportFileName,
  uploadReportBundle,
} from "../../services/reportService";
import { resolveBackendError } from "../../utils/backendI18n";
import {
  type ReportDraft,
  describeContext,
  describeMachine,
  describeResolution,
  minutesToWait,
  toUploadFailure,
} from "./ReportBugModal.helpers";

export type Working = "upload" | "export" | null;

interface SubmissionInputs {
  draft: ReportDraft;
  careerLine: string;
  reportText: () => string;
  includeSaveInBundle: boolean;
  consent: boolean;
  diagnostics: DiagnosticsReport | null;
  /** Called once a report is accepted, even if the dialog was closed while it was sent. */
  onUploaded?: () => void;
}

interface SubmissionOutcome {
  onSent: () => void;
  onExported: () => void;
}

/**
 * Through the same redactor the bundle uses. The player's own words are the one part of the report
 * nobody vets, and a path pasted into "what happened" would go into the URL verbatim, which is to
 * say into GitHub and the browser's history. There is no falling back to the raw text here: not
 * opening the form is recoverable, publishing a path is not.
 */
async function openIssueForm(
  inputs: SubmissionInputs,
  language: string,
  frequencyLabel: string,
): Promise<void> {
  const { draft, careerLine, diagnostics } = inputs;
  const redacted = await redactReportFields([
    draft.whatHappened,
    draft.expected,
    draft.steps,
    describeContext(frequencyLabel, careerLine),
  ]);
  const [whatHappened = "", expected = "", steps = "", gameContext = ""] = redacted;
  await openUrl(
    buildBugReportUrl(language, {
      whatHappened,
      expected,
      steps,
      gameContext,
      appVersion: formatAppVersion(),
      os: describeMachine(diagnostics),
      resolution: describeResolution(window.screen),
    }),
  );
}

export function useReportSubmission(inputs: SubmissionInputs, outcome: SubmissionOutcome) {
  const { t, i18n } = useTranslation();
  const [working, setWorking] = useState<Working>(null);
  const [failure, setFailure] = useState<string | null>(null);
  const [failureKind, setFailureKind] = useState<UploadFailureKind | null>(null);
  const [failureDetail, setFailureDetail] = useState<string | null>(null);
  const [receipt, setReceipt] = useState<UploadReceipt | null>(null);
  const [summary, setSummary] = useState<BundleSummary | null>(null);
  const [browserFailed, setBrowserFailed] = useState(false);
  // Read when the upload settles, not when it started: the dialog may have been closed since.
  const onUploaded = useRef(inputs.onUploaded);
  onUploaded.current = inputs.onUploaded;

  const clearFailure = () => {
    setFailure(null);
    setFailureKind(null);
    setFailureDetail(null);
  };

  const upload = async () => {
    if (!inputs.consent) return;
    setWorking("upload");
    clearFailure();
    try {
      const sent = await uploadReportBundle(
        inputs.reportText(),
        {
          what_happened: inputs.draft.whatHappened,
          expected: inputs.draft.expected,
          steps: inputs.draft.steps,
          frequency: inputs.draft.frequency,
          game_context: inputs.careerLine,
          locale: i18n.language,
        },
        inputs.includeSaveInBundle,
        inputs.consent,
      );
      setReceipt(sent);
      onUploaded.current?.();
      outcome.onSent();
    } catch (error: unknown) {
      const rejected = toUploadFailure(error);
      logError(`[report] upload failed: ${rejected.key}`);
      setFailureKind(rejected.kind);
      setFailureDetail(rejected.detail);
      setFailure(
        rejected.kind === "rateLimited" && rejected.retry_after_seconds !== null
          ? t("be.error.report.upload.rateLimitedWait", {
              minutes: minutesToWait(rejected.retry_after_seconds),
            })
          : resolveBackendError(rejected.key),
      );
    } finally {
      setWorking(null);
    }
  };

  const exportAndOpenForm = async () => {
    setWorking("export");
    clearFailure();
    try {
      const chosen = await save({
        defaultPath: await suggestedReportFileName(),
        filters: [{ name: "Zip", extensions: ["zip"] }],
      });
      // The dialog returns null when the player backs out; that is not a failure.
      if (typeof chosen !== "string") return;

      setSummary(await exportReportBundle(chosen, inputs.reportText(), inputs.includeSaveInBundle));
      // Two outcomes, reported separately on purpose: once the bundle exists the player has a
      // file, and telling them only that something failed sends them round again to write a
      // second copy. If the form cannot be opened, the file is still theirs.
      try {
        await openIssueForm(
          inputs,
          i18n.language,
          t(`reportBug.frequency.${inputs.draft.frequency}`),
        );
      } catch (error: unknown) {
        logError(`[report] could not open the issue form: ${String(error)}`);
        setBrowserFailed(true);
      }
      outcome.onExported();
    } catch (error: unknown) {
      logError(`[report] export failed: ${String(error)}`);
      setFailure(resolveBackendError(error));
    } finally {
      setWorking(null);
    }
  };

  return {
    working,
    failure,
    failureKind,
    failureDetail,
    receipt,
    summary,
    browserFailed,
    clearFailure,
    upload,
    exportAndOpenForm,
  };
}
