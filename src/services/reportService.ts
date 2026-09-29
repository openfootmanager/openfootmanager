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
