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

export interface UploadReceipt {
  /** `OFM-` and eight Crockford Base32 characters: an identifier to quote, not a password. */
  code: string;
  /** When false the code is not in the "Your reports" list, so the player should note it. */
  recorded: boolean;
}

export interface ReportFields {
  what_happened: string;
  expected: string;
  steps: string;
  /** The frequency choice's key, not its translated label. */
  frequency: string;
  game_context: string;
  locale: string;
}

export interface UploadFailure {
  key: string;
  retry_after_seconds: number | null;
}

export interface SubmittedReport {
  code: string;
  /** RFC 3339, UTC. */
  submitted_at: string;
  included_save: boolean;
  app_version: string;
}

/** Whether OFMHub takes reports right now. Without it, the GitHub form is the only path. */
export function reportUploadAvailable(): Promise<boolean> {
  return invoke<boolean>("report_upload_available");
}

/** `consent` is the player's tick, passed through rather than assumed: the backend refuses without it. Rejects with an `UploadFailure`. */
export function uploadReportBundle(
  reportText: string,
  fields: ReportFields,
  includeSave: boolean,
  consent: boolean,
): Promise<UploadReceipt> {
  return invoke<UploadReceipt>("upload_report_bundle", {
    reportText,
    fields,
    includeSave,
    consent,
    screenshotPaths: [],
    packagePaths: [],
  });
}

export function listSubmittedReports(): Promise<SubmittedReport[]> {
  return invoke<SubmittedReport[]>("list_submitted_reports");
}

/** That the last session crashed, and when — no message or trace, which carry paths. */
export interface CrashNotice {
  occurred_at: string;
  app_version: string;
}

/** The crash to offer a report for, or `null` when there is none or it was already answered. */
export function crashNotice(): Promise<CrashNotice | null> {
  return invoke<CrashNotice | null>("crash_notice");
}

/** The player answered the crash prompt, either way; it is not offered again this launch. */
export function dismissCrashNotice(): Promise<void> {
  return invoke<void>("dismiss_crash_notice");
}
