import { invoke } from "@tauri-apps/api/core";

/** One log file the report would carry. */
export interface LogFileSummary {
  name: string;
  bytes: number;
}

/** What the backend knows about this build and this machine. */
export interface DiagnosticsReport {
  app_version: string;
  os: string;
  arch: string;
  webview_version: string;
  log_directory: string;
  crash_on_previous_run: boolean;
  has_active_save: boolean;
  /** The logs this report would carry, newest first — chosen by the same code the export uses. */
  log_files: LogFileSummary[];
  /** Size of the save the tick box would attach, when there is one. */
  save_bytes: number | null;
}

/** What actually went into the bundle, for the preview and the confirmation. */
export interface BundleSummary {
  path: string;
  bytes: number;
  log_files: string[];
  included_save: boolean;
  included_crash: boolean;
}

export function collectDiagnostics(): Promise<DiagnosticsReport> {
  return invoke<DiagnosticsReport>("collect_diagnostics");
}

/**
 * Redact free text through the backend, for anything that leaves outside the bundle.
 *
 * The prefilled issue URL is the case that matters: it reaches GitHub and the browser's history,
 * and neither can be taken back.
 */
export function redactReportFields(values: string[]): Promise<string[]> {
  return invoke<string[]>("redact_report_fields", { values });
}

export function suggestedReportFileName(): Promise<string> {
  return invoke<string>("suggested_report_file_name");
}

export function exportReportBundle(
  outputPath: string,
  reportText: string,
  includeSave: boolean,
): Promise<BundleSummary> {
  return invoke<BundleSummary>("export_report_bundle", {
    outputPath,
    reportText,
    includeSave,
  });
}

/** The relay accepted the report. */
export interface UploadReceipt {
  /** Eight Crockford Base32 characters — an identifier to quote, not a password. */
  code: string;
  /** Whether the code also reached the "Your reports" list; if not, the player should note it. */
  recorded: boolean;
}

/** One report this install has sent, from `reports.json`. */
export interface SubmittedReport {
  code: string;
  /** RFC 3339, UTC. */
  submitted_at: string;
  included_save: boolean;
  app_version: string;
}

/** Whether this build has a relay to send to. Without one, the GitHub form is the only path. */
export function reportUploadAvailable(): Promise<boolean> {
  return invoke<boolean>("report_upload_available");
}

/**
 * Build the bundle privately and send it to the relay.
 *
 * `consent` is the player's tick on the preview, passed through rather than assumed — the backend
 * refuses without it. Rejects with a `be.error.report.upload.*` key on any failure.
 */
export function uploadReportBundle(
  reportText: string,
  includeSave: boolean,
  consent: boolean,
): Promise<UploadReceipt> {
  return invoke<UploadReceipt>("upload_report_bundle", { reportText, includeSave, consent });
}

/** The reports this install has sent, newest first. */
export function listSubmittedReports(): Promise<SubmittedReport[]> {
  return invoke<SubmittedReport[]>("list_submitted_reports");
}
