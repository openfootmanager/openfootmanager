import { fireEvent, render, screen, waitFor } from "@testing-library/react";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";

const saveDialog = vi.fn();
const openUrl = vi.fn();
const collectDiagnostics = vi.fn();
const exportReportBundle = vi.fn();
const suggestedReportFileName = vi.fn();
const redactReportFields = vi.fn();
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
  redactReportFields: (values: string[]) => redactReportFields(values),
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
  log_files: [
    { name: "app.log", bytes: 1_468_006 },
    { name: "app.2026-09-28.log", bytes: 1024 * 1024 },
  ],
  save_bytes: 12 * 1024 * 1024,
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
    // The real command replaces the home directory with `~`; the modal must send what comes back.
    redactReportFields
      .mockReset()
      .mockImplementation((values: string[]) =>
        Promise.resolve(values.map((value) => value.replace("/home/alice", "~"))),
      );
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

  it("still names the saved file when the browser will not open", async () => {
    // The bundle exists at this point. Reporting only a failure, without the path, sends the
    // player round again to write a second copy of it.
    openUrl.mockRejectedValue(new Error("no handler"));
    render(<ReportBugModal onClose={vi.fn()} />);
    fillRequired();
    fireEvent.click(screen.getByRole("button", { name: "reportBug.review" }));
    await screen.findByRole("heading", { name: "reportBug.previewTitle" });

    fireEvent.click(screen.getByRole("button", { name: "reportBug.saveAndOpen" }));

    expect(await screen.findByRole("heading", { name: "reportBug.doneTitle" })).toBeInTheDocument();
    expect(screen.getByText(/ofm-report\.zip/)).toBeInTheDocument();
    expect(screen.getByText("reportBug.browserDidNotOpen")).toBeInTheDocument();
  });

  it("refuses to close while the export is running", async () => {
    // Leaving mid-export used to let it finish and open the browser afterwards, on a report the
    // player had already dismissed.
    let release: (value: unknown) => void = () => {};
    exportReportBundle.mockReturnValue(
      new Promise((resolve) => {
        release = resolve;
      }),
    );
    const onClose = vi.fn();
    render(<ReportBugModal onClose={onClose} />);
    fillRequired();
    fireEvent.click(screen.getByRole("button", { name: "reportBug.review" }));
    await screen.findByRole("heading", { name: "reportBug.previewTitle" });

    fireEvent.click(screen.getByRole("button", { name: "reportBug.saveAndOpen" }));
    await waitFor(() => expect(exportReportBundle).toHaveBeenCalled());

    fireEvent.keyDown(window, { key: "Escape" });
    fireEvent.click(screen.getByRole("button", { name: "common.close" }));

    expect(onClose).not.toHaveBeenCalled();
    release(SUMMARY);
  });

  it("closes on Escape", () => {
    const onClose = vi.fn();
    render(<ReportBugModal onClose={onClose} />);

    fireEvent.keyDown(window, { key: "Escape" });

    expect(onClose).toHaveBeenCalled();
  });

  it("redacts what the player typed before it reaches the URL", async () => {
    // The bundle's copy of these words was redacted while the URL's was not, so a path pasted
    // into the description went to GitHub and into the browser's history — neither of which gives
    // it back.
    render(<ReportBugModal onClose={vi.fn()} />);
    fireEvent.change(screen.getByLabelText(/reportBug\.whatHappened/), {
      target: { value: "it died loading /home/alice/private/save.db" },
    });
    fireEvent.change(screen.getByLabelText(/reportBug\.expected/), {
      target: { value: "it should open" },
    });
    fireEvent.click(screen.getByRole("button", { name: "reportBug.review" }));
    await screen.findByRole("heading", { name: "reportBug.previewTitle" });

    fireEvent.click(screen.getByRole("button", { name: "reportBug.saveAndOpen" }));
    await waitFor(() => expect(openUrl).toHaveBeenCalled());

    const url = String(openUrl.mock.calls[0]?.[0]);
    expect(url).not.toContain("alice");
    expect(decodeURIComponent(url)).toContain("~/private/save.db");
  });

  it("does not fall back to the raw text when redaction fails", async () => {
    // Not opening the form is recoverable — the file is written and the done screen says where.
    // Publishing an unredacted path is not, so a failure here must not become a plain URL.
    redactReportFields.mockRejectedValue(new Error("no"));
    render(<ReportBugModal onClose={vi.fn()} />);
    fillRequired();
    fireEvent.click(screen.getByRole("button", { name: "reportBug.review" }));
    await screen.findByRole("heading", { name: "reportBug.previewTitle" });

    fireEvent.click(screen.getByRole("button", { name: "reportBug.saveAndOpen" }));
    await screen.findByRole("heading", { name: "reportBug.doneTitle" });

    expect(openUrl).not.toHaveBeenCalled();
    expect(screen.getByText("reportBug.browserDidNotOpen")).toBeInTheDocument();
  });

  it("names every file it is about to pack, with its size", async () => {
    // The preview is the consent screen, and "your logs" is not consent to something whose size
    // the player cannot see — a career database is tens of megabytes and it is the one part they
    // choose. The names come from the backend, which picks them with the same code the export
    // uses, so the screen cannot list one set of files and the zip hold another.
    render(<ReportBugModal onClose={vi.fn()} />);
    fillRequired();
    fireEvent.click(screen.getByRole("button", { name: "reportBug.review" }));
    await screen.findByRole("heading", { name: "reportBug.previewTitle" });

    expect(screen.getByText("app.log")).toBeInTheDocument();
    expect(screen.getByText("app.2026-09-28.log")).toBeInTheDocument();
    expect(screen.getByText("1.4 MB")).toBeInTheDocument();
    expect(screen.getByText("1.0 MB")).toBeInTheDocument();
    // And the two of them together, against the row that holds the list.
    expect(screen.getByText("2.4 MB")).toBeInTheDocument();
    // And the save, which is only offered because a career is open.
    expect(screen.getByText("12 MB")).toBeInTheDocument();
  });

  it("shows no save size when there is no career to attach", async () => {
    collectDiagnostics.mockResolvedValue({
      ...DIAGNOSTICS,
      has_active_save: false,
      save_bytes: null,
    });
    render(<ReportBugModal onClose={vi.fn()} />);
    fillRequired();
    fireEvent.click(screen.getByRole("button", { name: "reportBug.review" }));
    await screen.findByRole("heading", { name: "reportBug.previewTitle" });

    expect(screen.queryByText("12 MB")).not.toBeInTheDocument();
  });

  it("gives the save checkbox exactly one label", async () => {
    // `Checkbox` renders its own `<label>` around the real input. Wrapping that in a second label
    // is invalid, and the browser then associates the input with only one of them — so the row
    // the player clicks may not be the one that toggles anything.
    const { container } = render(<ReportBugModal onClose={vi.fn()} />);
    fillRequired();
    fireEvent.click(screen.getByRole("button", { name: "reportBug.review" }));
    await screen.findByRole("heading", { name: "reportBug.previewTitle" });

    expect(container.querySelector("label label")).toBeNull();

    // And the visible title is what names it, now that the checkbox carries no `aria-label`.
    const checkbox = screen.getByLabelText("reportBug.includeSave");
    expect(checkbox).toBe(screen.getByRole("checkbox"));

    fireEvent.click(checkbox);
    expect(checkbox).toBeChecked();
  });

  it("keeps Tab inside the dialog", async () => {
    // `aria-modal` is a promise to assistive technology, not something the browser enforces: the
    // page behind the overlay stays tabbable, so without a trap the player tabs into controls
    // they cannot see while the dialog is still open.
    const { container } = render(<ReportBugModal onClose={vi.fn()} />);
    await waitFor(() => expect(collectDiagnostics).toHaveBeenCalled());

    const focusable = Array.from(
      container.querySelectorAll<HTMLElement>("button:not([disabled]), textarea, input"),
    );
    const first = focusable[0];
    const last = focusable[focusable.length - 1];

    last.focus();
    fireEvent.keyDown(last, { key: "Tab" });
    expect(document.activeElement).toBe(first);

    fireEvent.keyDown(first, { key: "Tab", shiftKey: true });
    expect(document.activeElement).toBe(last);
  });
});
