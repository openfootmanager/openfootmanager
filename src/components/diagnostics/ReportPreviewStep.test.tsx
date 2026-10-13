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
});
