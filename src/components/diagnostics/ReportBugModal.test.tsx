import { fireEvent, render, screen, waitFor } from "@testing-library/react";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";

const saveDialog = vi.fn();
const openUrl = vi.fn();
const collectDiagnostics = vi.fn();
const exportReportBundle = vi.fn();
const suggestedReportFileName = vi.fn();
const logError = vi.fn();

vi.mock("@tauri-apps/plugin-dialog", () => ({
  save: (...args: unknown[]) => saveDialog(...args),
}));
vi.mock("@tauri-apps/plugin-opener", () => ({
  openUrl: (...args: unknown[]) => openUrl(...args),
}));
vi.mock("../../services/reportService", () => ({
  collectDiagnostics: () => collectDiagnostics(),
  exportReportBundle: (path: string, reportText: string, includeSave: boolean) =>
    exportReportBundle(path, reportText, includeSave),
  suggestedReportFileName: () => suggestedReportFileName(),
}));
vi.mock("../../lib/logger", () => ({
  logError: (message: string) => logError(message),
}));
vi.mock("../../utils/backendI18n", () => ({
  resolveBackendError: (error: unknown) => String(error),
}));
vi.mock("react-i18next", () => ({
  useTranslation: () => ({ t: (key: string) => key, i18n: { language: "en" } }),
}));

import { ReportBugModal } from "./ReportBugModal";

const DIAGNOSTICS = {
  app_version: "0.3.0",
  has_active_save: true,
  os: "linux",
  arch: "x86_64",
  webview_version: "2.50.1",
  log_directory: "~/.local/share/ofm/logs",
  crash_on_previous_run: false,
};

const SUMMARY = {
  path: "/home/x/ofm-report.zip",
  bytes: 1_468_006,
  log_files: ["app.log"],
  included_save: false,
  included_crash: false,
};

function fillRequired() {
  fireEvent.change(screen.getByLabelText(/reportBug\.whatHappened/), {
    target: { value: "It froze" },
  });
  fireEvent.change(screen.getByLabelText(/reportBug\.expected/), {
    target: { value: "It should start" },
  });
}

describe("ReportBugModal", () => {
  beforeEach(() => {
    collectDiagnostics.mockReset().mockResolvedValue(DIAGNOSTICS);
    exportReportBundle.mockReset().mockResolvedValue(SUMMARY);
    suggestedReportFileName.mockReset().mockResolvedValue("ofm-report.zip");
    saveDialog.mockReset().mockResolvedValue("/home/x/ofm-report.zip");
    openUrl.mockReset().mockResolvedValue(undefined);
    logError.mockReset();
  });

  afterEach(() => {
    vi.restoreAllMocks();
  });

  it("opens on the describe step", () => {
    render(<ReportBugModal onClose={vi.fn()} />);

    expect(screen.getByRole("heading", { name: "reportBug.describeTitle" })).toBeInTheDocument();
  });

  it("will not advance while a required field is blank", () => {
    render(<ReportBugModal onClose={vi.fn()} />);

    fireEvent.click(screen.getByRole("button", { name: "reportBug.review" }));

    expect(screen.getByRole("alert")).toHaveTextContent("reportBug.fillRequired");
    expect(
      screen.queryByRole("heading", { name: "reportBug.previewTitle" }),
    ).not.toBeInTheDocument();
  });

  it("shows what will be sent before anything is written", async () => {
    render(<ReportBugModal onClose={vi.fn()} />);
    fillRequired();

    fireEvent.click(screen.getByRole("button", { name: "reportBug.review" }));

    expect(
      await screen.findByRole("heading", { name: "reportBug.previewTitle" }),
    ).toBeInTheDocument();
    expect(screen.getByText("reportBug.itemLogs")).toBeInTheDocument();
    // Nothing has touched the disk at this point — that is the promise the screen makes.
    expect(exportReportBundle).not.toHaveBeenCalled();
    expect(saveDialog).not.toHaveBeenCalled();
  });

  it("leaves the save unchecked until the player asks for it", async () => {
    render(<ReportBugModal onClose={vi.fn()} />);
    fillRequired();
    fireEvent.click(screen.getByRole("button", { name: "reportBug.review" }));
    await screen.findByRole("heading", { name: "reportBug.previewTitle" });

    expect(screen.getByRole("checkbox")).not.toBeChecked();

    fireEvent.click(screen.getByRole("button", { name: "reportBug.saveAndOpen" }));
    await waitFor(() => expect(exportReportBundle).toHaveBeenCalled());
    expect(exportReportBundle).toHaveBeenCalledWith(
      "/home/x/ofm-report.zip",
      expect.stringContaining("It froze"),
      false,
    );
  });

  it("disables the save box, and says so, when no career is open", async () => {
    // The preview is the consent step. Claiming there is nothing to attach while the backend
    // would attach the open career is the one failure this screen must not have.
    collectDiagnostics.mockResolvedValue({ ...DIAGNOSTICS, has_active_save: false });
    render(<ReportBugModal onClose={vi.fn()} />);
    fillRequired();
    fireEvent.click(screen.getByRole("button", { name: "reportBug.review" }));
    await screen.findByRole("heading", { name: "reportBug.previewTitle" });

    expect(screen.getByRole("checkbox")).toBeDisabled();
    expect(screen.getByText("reportBug.includeSaveNoCareer")).toBeInTheDocument();
  });

  it("offers the save when a career is open, without claiming there is none", async () => {
    collectDiagnostics.mockResolvedValue({ ...DIAGNOSTICS, has_active_save: true });
    render(<ReportBugModal onClose={vi.fn()} />);
    fillRequired();
    fireEvent.click(screen.getByRole("button", { name: "reportBug.review" }));
    await screen.findByRole("heading", { name: "reportBug.previewTitle" });

    expect(screen.getByRole("checkbox")).toBeEnabled();
    expect(screen.queryByText("reportBug.includeSaveNoCareer")).not.toBeInTheDocument();
  });

  it("keeps the save box disabled when diagnostics could not be read", async () => {
    // Unknown must fail closed: offering to attach a save we cannot confirm exists is the same
    // mistake in the other direction.
    collectDiagnostics.mockRejectedValue(new Error("no backend"));
    render(<ReportBugModal onClose={vi.fn()} />);
    fillRequired();
    fireEvent.click(screen.getByRole("button", { name: "reportBug.review" }));
    await screen.findByRole("heading", { name: "reportBug.previewTitle" });

    expect(screen.getByRole("checkbox")).toBeDisabled();
  });

  it("includes the save when the box is ticked", async () => {
    render(<ReportBugModal onClose={vi.fn()} />);
    fillRequired();
    fireEvent.click(screen.getByRole("button", { name: "reportBug.review" }));
    await screen.findByRole("heading", { name: "reportBug.previewTitle" });

    fireEvent.click(screen.getByRole("checkbox"));
    fireEvent.click(screen.getByRole("button", { name: "reportBug.saveAndOpen" }));

    await waitFor(() =>
      expect(exportReportBundle).toHaveBeenCalledWith(
        "/home/x/ofm-report.zip",
        expect.stringContaining("It froze"),
        true,
      ),
    );
  });

  it("writes the bundle and opens the prefilled form", async () => {
    render(<ReportBugModal onClose={vi.fn()} />);
    fillRequired();
    fireEvent.click(screen.getByRole("button", { name: "reportBug.review" }));
    await screen.findByRole("heading", { name: "reportBug.previewTitle" });

    fireEvent.click(screen.getByRole("button", { name: "reportBug.saveAndOpen" }));

    await waitFor(() => expect(openUrl).toHaveBeenCalled());
    const url = openUrl.mock.calls[0][0] as string;
    expect(url).toContain("issues/new");
    expect(url).toContain("template=bug_report.yml");
    expect(new URL(url).searchParams.get("what-happened")).toBe("It froze");
    expect(await screen.findByRole("heading", { name: "reportBug.doneTitle" })).toBeInTheDocument();
  });

  it("does nothing when the player cancels the save dialog", async () => {
    saveDialog.mockResolvedValue(null);
    render(<ReportBugModal onClose={vi.fn()} />);
    fillRequired();
    fireEvent.click(screen.getByRole("button", { name: "reportBug.review" }));
    await screen.findByRole("heading", { name: "reportBug.previewTitle" });

    fireEvent.click(screen.getByRole("button", { name: "reportBug.saveAndOpen" }));

    await waitFor(() => expect(saveDialog).toHaveBeenCalled());
    expect(exportReportBundle).not.toHaveBeenCalled();
    expect(openUrl).not.toHaveBeenCalled();
    // Backing out of the file picker is not a failure and must not be reported as one.
    expect(screen.queryByRole("alert")).not.toBeInTheDocument();
  });

  it("keeps the player on the preview when writing fails", async () => {
    exportReportBundle.mockRejectedValue("be.error.report.bundleFailed");
    render(<ReportBugModal onClose={vi.fn()} />);
    fillRequired();
    fireEvent.click(screen.getByRole("button", { name: "reportBug.review" }));
    await screen.findByRole("heading", { name: "reportBug.previewTitle" });

    fireEvent.click(screen.getByRole("button", { name: "reportBug.saveAndOpen" }));

    expect(await screen.findByRole("alert")).toHaveTextContent("be.error.report.bundleFailed");
    // Still on the preview, with everything they typed intact, so they can retry.
    expect(screen.getByRole("heading", { name: "reportBug.previewTitle" })).toBeInTheDocument();
    expect(openUrl).not.toHaveBeenCalled();
  });

  it("stays usable when diagnostics cannot be collected", async () => {
    collectDiagnostics.mockRejectedValue(new Error("no backend"));
    render(<ReportBugModal onClose={vi.fn()} />);
    fillRequired();

    fireEvent.click(screen.getByRole("button", { name: "reportBug.review" }));

    expect(
      await screen.findByRole("heading", { name: "reportBug.previewTitle" }),
    ).toBeInTheDocument();
    await waitFor(() => expect(logError).toHaveBeenCalled());
  });

  it("closes on Escape", () => {
    const onClose = vi.fn();
    render(<ReportBugModal onClose={onClose} />);

    fireEvent.keyDown(window, { key: "Escape" });

    expect(onClose).toHaveBeenCalled();
  });
});
