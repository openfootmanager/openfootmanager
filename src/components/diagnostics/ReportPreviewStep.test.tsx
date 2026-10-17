import { render, screen } from "@testing-library/react";
import { describe, expect, it, vi } from "vitest";

import type { DiagnosticsReport } from "../../services/reportService";
import { ReportPreviewStep } from "./ReportPreviewStep";

vi.mock("react-i18next", () => ({
  useTranslation: () => ({ t: (key: string) => key, i18n: { language: "ru" } }),
}));

const DIAGNOSTICS: DiagnosticsReport = {
  app_version: "0.3.0",
  os: "linux",
  arch: "x86_64",
  webview_version: "2.50.1",
  log_directory: "/logs",
  crash_on_previous_run: false,
  has_active_save: true,
  log_files: [],
  save_bytes: 1_468_006,
};

function renderStep(overrides: Partial<React.ComponentProps<typeof ReportPreviewStep>> = {}) {
  return render(
    <ReportPreviewStep
      diagnostics={DIAGNOSTICS}
      careerLine=""
      logBytes={null}
      canAttachSave
      includeSave={false}
      onIncludeSaveChange={vi.fn()}
      busy={false}
      canUpload={false}
      consent={false}
      canConsent={false}
      onConsentChange={vi.fn()}
      failure={null}
      failureKind={null}
      failureDetail={null}
      {...overrides}
    />,
  );
}

describe("ReportPreviewStep", () => {
  /**
   * Given the UI language is Russian and a save could be attached
   * When the preview offers the save
   * Then its size reads in Russian.
   */
  it("shows the save size in the UI language", () => {
    renderStep();

    expect(screen.getByText("1,4 МБ")).toBeInTheDocument();
  });

  /**
   * Given the UI language is Russian and the logs total two megabytes
   * When the preview lists the logs
   * Then the total reads in Russian.
   */
  it("shows the logs size in the UI language", () => {
    renderStep({ canAttachSave: false, logBytes: 2 * 1024 * 1024 });

    expect(screen.getByText("2,0 МБ")).toBeInTheDocument();
  });

  /**
   * Given an upload refused with a reason from the server
   * When the preview shows the failure
   * Then the reason sits in a collapsed "Technical details" line, verbatim
   */
  it("shows the server's reason under a collapsible technical-details line", () => {
    const detail = 'HTTP 422 · invalid · bundle: Entry "report.md" has a disallowed file type';
    renderStep({
      failure: "The report was refused.",
      failureKind: "rejected",
      failureDetail: detail,
    });

    const summary = screen.getByText("reportBug.technicalDetails");
    expect(summary.closest("details")).not.toHaveAttribute("open");
    expect(screen.getByText(detail)).toBeInTheDocument();
  });

  /**
   * Given a failure the server said nothing about
   * When the preview shows it
   * Then there is no technical-details line
   */
  it("shows no technical details when there are none", () => {
    renderStep({ failure: "No connection.", failureKind: "unreachable" });

    expect(screen.queryByText("reportBug.technicalDetails")).not.toBeInTheDocument();
  });
});
