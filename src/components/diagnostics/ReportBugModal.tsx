import { useCallback, useEffect, useMemo, useRef, useState } from "react";
import { useTranslation } from "react-i18next";
import { save } from "@tauri-apps/plugin-dialog";
import { openUrl } from "@tauri-apps/plugin-opener";
import { AlertTriangle, Bug, Check, Eye, FileText, Package, X } from "lucide-react";

import { formatAppVersion } from "../../lib/appVersion";
import { buildBugReportUrl } from "../../lib/issueUrl";
import { logError } from "../../lib/logger";
import {
  type BundleSummary,
  type DiagnosticsReport,
  type LogFileSummary,
  collectDiagnostics,
  exportReportBundle,
  redactReportFields,
  suggestedReportFileName,
} from "../../services/reportService";
import { resolveBackendError } from "../../utils/backendI18n";
import { Button, Checkbox } from "../ui";
import {
  EMPTY_DRAFT,
  type Frequency,
  type ReportDraft,
  composeReportText,
  describeBundle,
  describeContext,
  describeMachine,
  describeResolution,
  formatBytes,
  missingRequiredFields,
} from "./ReportBugModal.helpers";

const FREQUENCIES: Frequency[] = ["everyTime", "sometimes", "once"];

interface ReportBugModalProps {
  onClose: () => void;
}

/**
 * Report a bug without leaving the game.
 *
 * Three steps, and the middle one is the point: the player sees every file and field that is about
 * to leave their machine, with sizes, before anything is written. Nothing here uploads — the bundle
 * is saved where they choose and the GitHub form opens with the text already filled in. That path
 * needs no server, which is why it is also the permanent fallback once one exists.
 */
export function ReportBugModal({ onClose }: ReportBugModalProps) {
  const { t, i18n } = useTranslation();
  const [step, setStep] = useState<"describe" | "preview" | "done">("describe");
  const [draft, setDraft] = useState<ReportDraft>(EMPTY_DRAFT);
  const [showErrors, setShowErrors] = useState(false);
  const [includeSave, setIncludeSave] = useState(false);
  const [diagnostics, setDiagnostics] = useState<DiagnosticsReport | null>(null);
  const [summary, setSummary] = useState<BundleSummary | null>(null);
  const [failure, setFailure] = useState<string | null>(null);
  const [busy, setBusy] = useState(false);
  const [browserFailed, setBrowserFailed] = useState(false);
  const headingRef = useRef<HTMLHeadingElement>(null);
  const dialogRef = useRef<HTMLDivElement>(null);

  useEffect(() => {
    let cancelled = false;
    void collectDiagnostics()
      .then((report) => {
        if (!cancelled) setDiagnostics(report);
      })
      .catch((error: unknown) => {
        // Not fatal: the report is still worth sending without the machine summary, so the modal
        // stays usable and the failure goes to the log rather than in front of the player.
        logError(`[report] could not collect diagnostics: ${String(error)}`);
      });
    return () => {
      cancelled = true;
    };
  }, []);

  // One guard for every way out of the dialog. Leaving while an export is running used to let
  // the export finish and open the browser afterwards, on a report the player had just dismissed —
  // and the done screen naming the file they now had was never shown.
  const requestClose = useCallback(() => {
    if (busy) return;
    onClose();
  }, [busy, onClose]);

  useEffect(() => {
    const onKeyDown = (event: KeyboardEvent) => {
      if (event.key === "Escape") {
        requestClose();
        return;
      }
      if (event.key !== "Tab") return;
      const dialog = dialogRef.current;
      if (dialog === null) return;

      // `aria-modal` is a promise to assistive technology, not something the browser enforces: the
      // page behind the overlay stays fully tabbable. Without this, Tab walks out of the dialog
      // into controls the player cannot see, and the export they started is still running.
      const focusable = dialog.querySelectorAll<HTMLElement>(
        'a[href], button:not([disabled]), textarea:not([disabled]), input:not([disabled]), select:not([disabled]), [tabindex]:not([tabindex="-1"])',
      );
      if (focusable.length === 0) return;
      const first = focusable[0];
      const last = focusable[focusable.length - 1];
      const active = document.activeElement;
      const outside = !dialog.contains(active);

      if (!event.shiftKey && (active === last || outside)) {
        event.preventDefault();
        first.focus();
      } else if (event.shiftKey && (active === first || outside)) {
        event.preventDefault();
        last.focus();
      }
    };
    window.addEventListener("keydown", onKeyDown);
    return () => window.removeEventListener("keydown", onKeyDown);
  }, [requestClose]);

  // A dialog that never takes focus is one a keyboard user cannot reach: `aria-modal` alone leaves
  // focus on the button behind the overlay. Whatever had focus when the dialog opened gets it back
  // when it closes.
  useEffect(() => {
    const previouslyFocused = document.activeElement as HTMLElement | null;
    return () => previouslyFocused?.focus?.();
  }, []);

  // On every step, not only the first. Each step replaces the whole body of the dialog, including
  // the button that was just pressed, so focus fell to `<body>`: the new title was never announced,
  // and the next Tab started from the top of the page — outside the dialog entirely.
  useEffect(() => {
    headingRef.current?.focus();
  }, [step]);

  const missing = useMemo(() => missingRequiredFields(draft), [draft]);

  // What the logs add up to, so the preview can put one number against the list. `null` while the
  // backend summary has not arrived — a total of zero would read as "there are no logs".
  const logBytes = useMemo(
    () =>
      diagnostics === null
        ? null
        : diagnostics.log_files.reduce((total, file) => total + file.bytes, 0),
    [diagnostics],
  );

  // Fails closed. `diagnostics` is null until the backend answers and stays null if it never does,
  // and offering to attach a save we cannot confirm exists is the same mistake as denying one that
  // does. The export reads `get_save_id()` — the value behind this flag — so the two agree.
  const canAttachSave = diagnostics?.has_active_save === true;

  const handleContinue = () => {
    if (missing.length > 0) {
      setShowErrors(true);
      return;
    }
    setShowErrors(false);
    setStep("preview");
  };

  const handleSubmit = async () => {
    setBusy(true);
    setFailure(null);
    try {
      const defaultPath = await suggestedReportFileName();
      const chosen = await save({
        defaultPath,
        filters: [{ name: "Zip", extensions: ["zip"] }],
      });
      // The dialog returns null when the player backs out; that is not a failure.
      if (typeof chosen !== "string") return;

      const written = await exportReportBundle(
        chosen,
        composeReportText(draft, {
          whatHappened: t("reportBug.whatHappened"),
          expected: t("reportBug.expected"),
          steps: t("reportBug.steps"),
          frequency: t(`reportBug.frequency.${draft.frequency}`),
        }),
        canAttachSave && includeSave,
      );
      setSummary(written);

      // Two outcomes, reported separately on purpose. Once the bundle exists the player has a
      // file; telling them only that something failed, without saying where it is, sends them
      // round again to write a second copy of it. The redaction belongs inside this block for the
      // same reason: if it fails, the file is still theirs and the browser step is what is lost.
      try {
        // Through the same redactor the bundle uses. The player's own words are the one part of
        // the report nobody vets, and a path pasted into "what happened" went into the URL
        // verbatim — which is to say into GitHub and into their browser history, neither of which
        // can be undone, while the copy in the zip beside it was clean. There is no falling back
        // to the raw text here: not opening the form is recoverable, publishing a path is not.
        const redacted = await redactReportFields([
          draft.whatHappened,
          draft.expected,
          draft.steps,
          describeContext(t(`reportBug.frequency.${draft.frequency}`), ""),
        ]);
        const [whatHappened = "", expected = "", steps = "", gameContext = ""] = redacted;

        await openUrl(
          buildBugReportUrl(i18n.language, {
            whatHappened,
            expected,
            steps,
            gameContext,
            appVersion: formatAppVersion(),
            os: describeMachine(diagnostics),
            resolution: describeResolution(window.screen),
          }),
        );
      } catch (error: unknown) {
        logError(`[report] could not open the issue form: ${String(error)}`);
        setBrowserFailed(true);
      }
      setStep("done");
    } catch (error: unknown) {
      logError(`[report] export failed: ${String(error)}`);
      setFailure(resolveBackendError(error));
    } finally {
      setBusy(false);
    }
  };

  const field = (key: "whatHappened" | "expected" | "steps", rows: number, required: boolean) => {
    const invalid = required && showErrors && missing.includes(key);
    return (
      <div>
        <label
          htmlFor={`report-${key}`}
          className="block text-[11px] font-heading font-bold uppercase tracking-wider text-gray-500 dark:text-gray-400 mb-1.5"
        >
          {t(`reportBug.${key}`)}
          {required && (
            <span className="ml-2 text-accent-600 dark:text-accent-400 tracking-normal">
              {t("reportBug.required")}
            </span>
          )}
        </label>
        <textarea
          id={`report-${key}`}
          rows={rows}
          value={draft[key]}
          aria-invalid={invalid}
          onChange={(event) => setDraft((prev) => ({ ...prev, [key]: event.target.value }))}
          placeholder={t(`reportBug.${key}Placeholder`)}
          className={`w-full px-3 py-2 rounded-lg bg-gray-50 dark:bg-navy-700 border text-sm text-gray-800 dark:text-gray-200 focus:outline-none focus:ring-2 focus:ring-primary-500/50 ${
            invalid ? "border-red-500 dark:border-red-500" : "border-gray-200 dark:border-navy-600"
          }`}
        />
      </div>
    );
  };

  const previewRow = (
    icon: React.ReactNode,
    name: string,
    detail: string,
    size?: string,
    files?: LogFileSummary[],
  ) => (
    <div className="flex items-start gap-2.5 px-3 py-2.5 rounded-lg bg-gray-50 dark:bg-navy-700 border border-gray-200 dark:border-navy-600">
      <span className="text-primary-600 dark:text-primary-400 shrink-0 mt-0.5">{icon}</span>
      <div className="flex-1 min-w-0">
        <p className="text-[13px] font-medium text-gray-800 dark:text-gray-200">{name}</p>
        <p className="text-[11px] text-gray-600 dark:text-gray-400 mt-0.5">{detail}</p>
        {files && files.length > 0 && (
          <ul className="mt-1.5 flex flex-col gap-0.5">
            {files.map((file) => (
              <li
                key={file.name}
                className="flex items-baseline justify-between gap-3 text-[11px] font-mono text-gray-500 dark:text-gray-400"
              >
                <span className="truncate">{file.name}</span>
                <span className="shrink-0">{formatBytes(file.bytes)}</span>
              </li>
            ))}
          </ul>
        )}
      </div>
      {size && (
        <span className="text-[11px] font-mono text-gray-600 dark:text-gray-400 shrink-0">
          {size}
        </span>
      )}
    </div>
  );

  return (
    <div
      className="fixed inset-0 bg-black/50 flex items-center justify-center z-50 p-4"
      role="dialog"
      aria-modal="true"
      aria-labelledby="report-bug-title"
    >
      <div
        ref={dialogRef}
        className="bg-white dark:bg-navy-800 rounded-xl shadow-2xl border border-gray-200 dark:border-navy-600 p-6 w-full max-w-lg max-h-[90vh] overflow-y-auto"
      >
        <div className="flex items-start gap-3 mb-5">
          <span className="text-primary-600 dark:text-primary-400 shrink-0 mt-0.5">
            {step === "done" ? (
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
              {t(`reportBug.${step}Intro`)}
            </p>
          </div>
          <button
            type="button"
            onClick={requestClose}
            disabled={busy}
            aria-label={t("common.close")}
            className="p-1 rounded-lg text-gray-500 dark:text-gray-400 hover:bg-gray-100 dark:hover:bg-navy-700 focus:outline-none focus:ring-2 focus:ring-primary-500 focus:ring-offset-2 dark:focus:ring-offset-navy-800"
          >
            <X className="w-4 h-4" />
          </button>
        </div>

        {step === "describe" && (
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
                    onClick={() => setDraft((prev) => ({ ...prev, frequency: value }))}
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
              <p role="alert" className="text-xs text-red-600 dark:text-red-400">
                {t("reportBug.fillRequired")}
              </p>
            )}
          </div>
        )}

        {step === "preview" && (
          <div className="flex flex-col gap-2">
            {previewRow(
              <FileText className="w-4 h-4" />,
              t("reportBug.itemWhatYouWrote"),
              t("reportBug.itemWhatYouWroteDesc"),
            )}
            {previewRow(
              <Package className="w-4 h-4" />,
              t("reportBug.itemSetup"),
              `${formatAppVersion()} · ${describeMachine(diagnostics)}`,
            )}
            {previewRow(
              <FileText className="w-4 h-4" />,
              t("reportBug.itemLogs"),
              t("reportBug.itemLogsDesc"),
              logBytes === null ? undefined : formatBytes(logBytes),
              diagnostics?.log_files,
            )}
            {diagnostics?.crash_on_previous_run &&
              previewRow(
                <AlertTriangle className="w-4 h-4" />,
                t("reportBug.itemCrash"),
                t("reportBug.itemCrashDesc"),
              )}

            <div className="h-px bg-gray-200 dark:bg-navy-600 my-1.5" />
            <p className="text-[11px] font-heading font-bold uppercase tracking-wider text-gray-500 dark:text-gray-400">
              {t("reportBug.optionalHeading")}
            </p>
            {/* A plain row, not a `<label>`: `Checkbox` renders its own label around the real
                input, and a label nested inside another label is invalid — the browser associates
                the input with one of them and the other stops toggling anything. The visible title
                below is the single label that owns this input, and it names it for a screen
                reader, so the checkbox carries no `aria-label` of its own to override it. */}
            <div className="flex items-center gap-2.5 px-3 py-2.5 rounded-lg bg-gray-50 dark:bg-navy-700 border border-gray-200 dark:border-navy-600">
              <Checkbox
                id="report-include-save"
                checked={canAttachSave && includeSave}
                disabled={!canAttachSave}
                onChange={(event) => setIncludeSave(event.target.checked)}
              />
              <div className="flex-1 min-w-0">
                <label
                  htmlFor="report-include-save"
                  className={`block text-[13px] font-medium text-gray-800 dark:text-gray-200 ${
                    canAttachSave ? "cursor-pointer" : "cursor-not-allowed"
                  }`}
                >
                  {t("reportBug.includeSave")}
                </label>
                <p className="text-[11px] text-gray-600 dark:text-gray-400 mt-0.5">
                  {canAttachSave
                    ? t("reportBug.includeSaveDesc")
                    : t("reportBug.includeSaveNoCareer")}
                </p>
              </div>
              {canAttachSave && diagnostics !== null && diagnostics.save_bytes !== null && (
                <span className="text-[11px] font-mono text-gray-600 dark:text-gray-400 shrink-0">
                  {formatBytes(diagnostics.save_bytes)}
                </span>
              )}
            </div>

            <p className="text-[11px] text-gray-600 dark:text-gray-400 mt-2">
              {t("reportBug.nothingSentYet")}
            </p>
            {failure && (
              <p role="alert" className="text-xs text-red-600 dark:text-red-400">
                {failure}
              </p>
            )}
          </div>
        )}

        {step === "done" && summary && (
          <div className="flex flex-col gap-3">
            <p className="text-sm text-gray-800 dark:text-gray-200">{t("reportBug.savedTo")}</p>
            <code className="block px-3 py-2 rounded-lg bg-gray-50 dark:bg-navy-700 border border-gray-200 dark:border-navy-600 text-[11px] font-mono text-gray-700 dark:text-gray-300 break-all">
              {describeBundle(summary)}
            </code>
            <p className="text-xs text-gray-600 dark:text-gray-400">
              {browserFailed ? t("reportBug.browserDidNotOpen") : t("reportBug.dragItIn")}
            </p>
          </div>
        )}

        <div className="flex gap-3 mt-6">
          {step === "describe" && (
            <>
              <Button className="flex-1" onClick={handleContinue}>
                {t("reportBug.review")}
              </Button>
              <Button variant="outline" onClick={requestClose}>
                {t("common.cancel")}
              </Button>
            </>
          )}
          {step === "preview" && (
            <>
              <Button
                className="flex-1"
                disabled={busy}
                onClick={() => {
                  void handleSubmit();
                }}
              >
                {busy ? t("reportBug.working") : t("reportBug.saveAndOpen")}
              </Button>
              <Button variant="outline" disabled={busy} onClick={() => setStep("describe")}>
                {t("common.back")}
              </Button>
            </>
          )}
          {step === "done" && (
            <Button className="flex-1" onClick={onClose}>
              {t("common.close")}
            </Button>
          )}
        </div>
      </div>
    </div>
  );
}
