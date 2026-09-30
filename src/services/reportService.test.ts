import { beforeEach, describe, expect, it, vi } from "vitest";
import { invoke } from "@tauri-apps/api/core";

import {
  type BundleSummary,
  type DiagnosticsReport,
  collectDiagnostics,
  exportReportBundle,
  suggestedReportFileName,
} from "./reportService";

vi.mock("@tauri-apps/api/core", () => ({
  invoke: vi.fn(),
}));

const mockedInvoke = vi.mocked(invoke);

const DIAGNOSTICS: DiagnosticsReport = {
  app_version: "0.3.0",
  os: "linux",
  arch: "x86_64",
  webview_version: "2.50.1",
  log_directory: "~/.local/share/ofm/logs",
  crash_on_previous_run: false,
  has_active_save: true,
  log_files: [{ name: "app.log", bytes: 1_468_006 }],
  save_bytes: 12 * 1024 * 1024,
};

const SUMMARY: BundleSummary = {
  path: "/home/x/ofm-report.zip",
  bytes: 1_468_006,
  log_files: ["app.log"],
  included_save: true,
  included_crash: false,
};

describe("reportService", () => {
  beforeEach(() => {
    mockedInvoke.mockReset();
  });

  it("asks the backend what this machine is", async () => {
    mockedInvoke.mockResolvedValueOnce(DIAGNOSTICS);

    await expect(collectDiagnostics()).resolves.toBe(DIAGNOSTICS);

    expect(mockedInvoke).toHaveBeenCalledWith("collect_diagnostics");
  });

  it("asks the backend for the dated file name", async () => {
    mockedInvoke.mockResolvedValueOnce("ofm-report-2026-09-29.zip");

    await expect(suggestedReportFileName()).resolves.toBe("ofm-report-2026-09-29.zip");

    expect(mockedInvoke).toHaveBeenCalledWith("suggested_report_file_name");
  });

  it("passes the export arguments under the names the command expects", async () => {
    // Tauri matches arguments by name, and a mismatch is not a compile error on either side: the
    // command simply receives a default. `include_save` defaulting to false would write a bundle
    // without the save the player ticked, and `report_text` defaulting to empty would drop what
    // they wrote — both silently. The exact object is the assertion, so a renamed key fails here
    // rather than in front of a player.
    mockedInvoke.mockResolvedValueOnce(SUMMARY);

    await expect(
      exportReportBundle("/home/x/ofm-report.zip", "It froze on the team screen", true),
    ).resolves.toBe(SUMMARY);

    expect(mockedInvoke).toHaveBeenCalledWith("export_report_bundle", {
      outputPath: "/home/x/ofm-report.zip",
      reportText: "It froze on the team screen",
      includeSave: true,
    });
  });

  it("does not turn an unticked save into a ticked one", async () => {
    mockedInvoke.mockResolvedValueOnce({ ...SUMMARY, included_save: false });

    await exportReportBundle("/home/x/ofm-report.zip", "", false);

    expect(mockedInvoke).toHaveBeenCalledWith("export_report_bundle", {
      outputPath: "/home/x/ofm-report.zip",
      reportText: "",
      includeSave: false,
    });
  });

  it("lets a failed export reach the caller", async () => {
    // The modal keeps the player on the preview with everything they typed when this rejects, so
    // swallowing it here would look like a report that was written and was not.
    mockedInvoke.mockRejectedValueOnce("be.error.report.bundleFailed");

    await expect(exportReportBundle("/home/x/ofm-report.zip", "", false)).rejects.toBe(
      "be.error.report.bundleFailed",
    );
  });
});
