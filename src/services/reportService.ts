import { invoke } from "@tauri-apps/api/core";

/** What the backend knows about this build and this machine. */
export interface DiagnosticsReport {
  app_version: string;
  os: string;
  arch: string;
  webview_version: string;
  log_directory: string;
  crash_on_previous_run: boolean;
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
  includeSave: boolean,
): Promise<BundleSummary> {
  return invoke<BundleSummary>("export_report_bundle", {
    outputPath,
    includeSave,
  });
}
