import { fireEvent, render, screen, waitFor } from "@testing-library/react";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";

const saveDialog = vi.fn();
const openUrl = vi.fn();
const collectDiagnostics = vi.fn();
const exportReportBundle = vi.fn();
const suggestedReportFileName = vi.fn();
const redactReportFields = vi.fn();
const reportUploadAvailable = vi.fn();
const uploadReportBundle = vi.fn();
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
  reportUploadAvailable: () => reportUploadAvailable(),
  uploadReportBundle: (reportText: string, includeSave: boolean, consent: boolean) =>
    uploadReportBundle(reportText, includeSave, consent),
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

import { useGameStore } from "../../store/gameStore";
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
    // The tests in this block cover a build with no relay, where the GitHub form is the only path.
    reportUploadAvailable.mockReset().mockResolvedValue(false);
    uploadReportBundle.mockReset();
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
    useGameStore.setState({ gameState: null });
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

  it("says where the player was when it broke", async () => {
    // The backend deliberately does not duplicate the career, so composing it is this screen's
    // job — and it was passing an empty string, which meant nothing described the world the bug
    // happened in. The packages in particular are what make it reproducible.
    useGameStore.setState({
      gameState: {
        clock: { current_date: "2026-03-14", start_date: "2025-07-01" },
        manager: { team_id: "t1" },
        teams: [{ id: "t1", name: "Boca Juniors" }],
        league: { id: "l1", name: "Primera División", season: 2026 },
        package_lockfile: [{ id: "argentina-1962", version: "1.2.0", hash: "abc" }],
      } as never,
    });
    render(<ReportBugModal onClose={vi.fn()} />);
    fillRequired();
    fireEvent.click(screen.getByRole("button", { name: "reportBug.review" }));
    await screen.findByRole("heading", { name: "reportBug.previewTitle" });

    // On the preview first: the screen may not promise what it has not shown.
    expect(screen.getByText(/Boca Juniors/)).toBeInTheDocument();

    fireEvent.click(screen.getByRole("button", { name: "reportBug.saveAndOpen" }));
    await waitFor(() => expect(openUrl).toHaveBeenCalled());

    expect(exportReportBundle).toHaveBeenCalledWith(
      "/home/x/ofm-report.zip",
      expect.stringContaining("argentina-1962@1.2.0"),
      false,
    );
    // `+` for a space is form encoding, which `decodeURIComponent` does not undo.
    const url = String(openUrl.mock.calls[0]?.[0]).replace(/\+/g, " ");
    expect(decodeURIComponent(url)).toContain("Primera División");
  });

  it("moves focus to the title of every step, not just the first", async () => {
    // Each step replaces the body of the dialog, including the button just pressed. Without this
    // focus falls to <body>: the new title is never announced, and the next Tab starts at the top
    // of the page behind the overlay rather than inside the dialog.
    render(<ReportBugModal onClose={vi.fn()} />);
    expect(screen.getByRole("heading", { name: "reportBug.describeTitle" })).toHaveFocus();

    fillRequired();
    // Focused explicitly because `fireEvent.click` does not move focus the way a real activation
    // does — and the button holding focus when it unmounts is the whole failure being guarded.
    const review = screen.getByRole("button", { name: "reportBug.review" });
    review.focus();
    fireEvent.click(review);

    // `waitFor`, not a bare assertion after `findBy`: the heading's text changes when React
    // commits, and the effect that moves focus runs after that — so a query resolving on the text
    // alone can read focus a beat too early. This failed that way once in ten runs.
    await waitFor(() =>
      expect(screen.getByRole("heading", { name: "reportBug.previewTitle" })).toHaveFocus(),
    );

    const submit = screen.getByRole("button", { name: "reportBug.saveAndOpen" });
    submit.focus();
    fireEvent.click(submit);

    await waitFor(() =>
      expect(screen.getByRole("heading", { name: "reportBug.doneTitle" })).toHaveFocus(),
    );
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

  it("locks the save box and says it is busy while exporting", async () => {
    // The refusal to close was invisible: no `aria-busy`, a close button with no disabled styling,
    // and a tick box still accepting clicks after the export had already read it.
    let release: (value: unknown) => void = () => {};
    exportReportBundle.mockReturnValue(
      new Promise((resolve) => {
        release = resolve;
      }),
    );
    const { container } = render(<ReportBugModal onClose={vi.fn()} />);
    fillRequired();
    fireEvent.click(screen.getByRole("button", { name: "reportBug.review" }));
    await screen.findByRole("heading", { name: "reportBug.previewTitle" });

    fireEvent.click(screen.getByRole("button", { name: "reportBug.saveAndOpen" }));
    await waitFor(() => expect(exportReportBundle).toHaveBeenCalled());

    expect(container.querySelector('[role="dialog"]')).toHaveAttribute("aria-busy", "true");
    expect(screen.getByRole("checkbox")).toBeDisabled();
    expect(screen.getByRole("button", { name: "common.close" })).toBeDisabled();

    release(SUMMARY);
  });

  it("says on the done screen whether the save went in", async () => {
    // The preview asked the player to decide about their career. Never saying afterwards whether
    // it was attached leaves the one question they were asked to answer unanswered.
    exportReportBundle.mockResolvedValue({ ...SUMMARY, included_save: true });
    render(<ReportBugModal onClose={vi.fn()} />);
    fillRequired();
    fireEvent.click(screen.getByRole("button", { name: "reportBug.review" }));
    await screen.findByRole("heading", { name: "reportBug.previewTitle" });

    fireEvent.click(screen.getByRole("button", { name: "reportBug.saveAndOpen" }));
    await screen.findByRole("heading", { name: "reportBug.doneTitle" });

    expect(screen.getByText(/reportBug\.includeSave/)).toBeInTheDocument();
  });

  it("does not claim a save that was left out", async () => {
    render(<ReportBugModal onClose={vi.fn()} />);
    fillRequired();
    fireEvent.click(screen.getByRole("button", { name: "reportBug.review" }));
    await screen.findByRole("heading", { name: "reportBug.previewTitle" });

    fireEvent.click(screen.getByRole("button", { name: "reportBug.saveAndOpen" }));
    await screen.findByRole("heading", { name: "reportBug.doneTitle" });

    expect(screen.queryByText(/reportBug\.includeSave/)).not.toBeInTheDocument();
    expect(screen.getByText(/reportBug\.itemLogs/)).toBeInTheDocument();
  });

  it("tells a screen reader which field is missing", async () => {
    render(<ReportBugModal onClose={vi.fn()} />);

    const whatHappened = screen.getByLabelText(/reportBug\.whatHappened/);
    expect(whatHappened).toHaveAttribute("aria-required", "true");
    expect(whatHappened).not.toHaveAttribute("aria-describedby");

    fireEvent.click(screen.getByRole("button", { name: "reportBug.review" }));

    const error = screen.getByRole("alert");
    expect(whatHappened).toHaveAttribute("aria-describedby", error.id);
    expect(error.id).not.toBe("");
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

describe("ReportBugModal with a relay to send to", () => {
  beforeEach(() => {
    collectDiagnostics.mockReset().mockResolvedValue(DIAGNOSTICS);
    reportUploadAvailable.mockReset().mockResolvedValue(true);
    uploadReportBundle.mockReset().mockResolvedValue({ code: "7K2M9Q4R", recorded: true });
    exportReportBundle.mockReset().mockResolvedValue(SUMMARY);
    suggestedReportFileName.mockReset().mockResolvedValue("ofm-report.zip");
    saveDialog.mockReset().mockResolvedValue("/home/x/ofm-report.zip");
    redactReportFields
      .mockReset()
      .mockImplementation((values: string[]) => Promise.resolve(values));
    openUrl.mockReset().mockResolvedValue(undefined);
    logError.mockReset();
  });

  afterEach(() => {
    vi.restoreAllMocks();
    useGameStore.setState({ gameState: null });
  });

  async function reachPreview() {
    render(<ReportBugModal onClose={vi.fn()} />);
    fillRequired();
    fireEvent.click(screen.getByRole("button", { name: "reportBug.review" }));
    await screen.findByRole("heading", { name: "reportBug.previewTitle" });
    // The consent box waits for the file list; wait for it to be offered.
    await waitFor(() =>
      expect(screen.getByRole("checkbox", { name: /reportBug\.consentLabel/ })).toBeEnabled(),
    );
  }

  const consentBox = () => screen.getByRole("checkbox", { name: /reportBug\.consentLabel/ });
  const saveBox = () => screen.getByRole("checkbox", { name: "reportBug.includeSave" });
  const sendButton = () => screen.getByRole("button", { name: "reportBug.send" });

  /**
   * Given a build with a relay and a player on the preview,
   * when they have not ticked the consent box,
   * then Send is disabled and nothing has been uploaded.
   */
  it("will not send until the player consents", async () => {
    await reachPreview();

    expect(consentBox()).not.toBeChecked();
    // What ticking it means is read out with it, not only shown beside it.
    expect(consentBox()).toHaveAccessibleDescription("reportBug.consentDesc");
    expect(sendButton()).toBeDisabled();
    expect(uploadReportBundle).not.toHaveBeenCalled();
  });

  /**
   * Given a player who ticked consent and left the save unticked,
   * when they press Send,
   * then the report is uploaded with consent and without the save, no file dialog or browser
   * opens, and the reference code is shown.
   */
  it("sends with consent and shows the reference code", async () => {
    await reachPreview();

    fireEvent.click(consentBox());
    fireEvent.click(sendButton());

    expect(await screen.findByRole("heading", { name: "reportBug.sentTitle" })).toBeInTheDocument();
    expect(uploadReportBundle).toHaveBeenCalledWith(
      expect.stringContaining("It froze"),
      false,
      true,
    );
    expect(screen.getByText("7K2M9Q4R")).toBeInTheDocument();
    expect(screen.getByText("reportBug.referenceCodeSaved")).toBeInTheDocument();
    expect(saveDialog).not.toHaveBeenCalled();
    expect(openUrl).not.toHaveBeenCalled();
  });

  /**
   * Given a report the relay accepted but the game could not add to the list,
   * when the code is shown,
   * then the player is told to write it down.
   */
  it("asks the player to note a code it could not record", async () => {
    uploadReportBundle.mockResolvedValue({ code: "7K2M9Q4R", recorded: false });
    await reachPreview();

    fireEvent.click(consentBox());
    fireEvent.click(sendButton());

    expect(await screen.findByRole("alert")).toHaveTextContent("reportBug.notRecorded");
    expect(screen.queryByText("reportBug.referenceCodeSaved")).not.toBeInTheDocument();
  });

  /**
   * Given a player who consented and then ticked their save,
   * when the bundle changes,
   * then consent is withdrawn — agreeing to one bundle is not agreeing to another.
   */
  it("withdraws consent when the save choice changes", async () => {
    await reachPreview();

    fireEvent.click(consentBox());
    fireEvent.click(saveBox());

    expect(consentBox()).not.toBeChecked();
    expect(sendButton()).toBeDisabled();
  });

  /**
   * Given a player who consented and then went back to edit their description,
   * when they return to the preview,
   * then consent has to be given again.
   */
  it("withdraws consent when the player goes back to edit", async () => {
    await reachPreview();
    fireEvent.click(consentBox());

    fireEvent.click(screen.getByRole("button", { name: "common.back" }));
    fireEvent.click(screen.getByRole("button", { name: "reportBug.review" }));
    await screen.findByRole("heading", { name: "reportBug.previewTitle" });

    expect(consentBox()).not.toBeChecked();
  });

  /**
   * Given a preview whose file list has not been read,
   * when the player looks for the consent box,
   * then it is disabled: consent to a list nobody has seen is not consent.
   */
  it("does not take consent before the file list is known", async () => {
    collectDiagnostics.mockRejectedValue(new Error("no backend"));
    render(<ReportBugModal onClose={vi.fn()} />);
    fillRequired();
    fireEvent.click(screen.getByRole("button", { name: "reportBug.review" }));
    await screen.findByRole("heading", { name: "reportBug.previewTitle" });

    expect(consentBox()).toBeDisabled();
  });

  /**
   * Given a relay that refuses the report,
   * when the player presses Send,
   * then they stay on the preview with the reason and the GitHub fallback, and that fallback
   * still saves the file and opens the form.
   */
  it("falls back to the GitHub form when the upload fails", async () => {
    uploadReportBundle.mockRejectedValue("be.error.report.upload.rateLimited");
    await reachPreview();

    fireEvent.click(consentBox());
    fireEvent.click(sendButton());

    const alert = await screen.findByRole("alert");
    expect(alert).toHaveTextContent("be.error.report.upload.rateLimited");
    expect(alert).toHaveTextContent("reportBug.uploadFailedFallback");
    expect(screen.getByRole("heading", { name: "reportBug.previewTitle" })).toBeInTheDocument();

    fireEvent.click(screen.getByRole("button", { name: "reportBug.saveAndOpenInstead" }));
    await screen.findByRole("heading", { name: "reportBug.doneTitle" });
    expect(exportReportBundle).toHaveBeenCalled();
    expect(openUrl).toHaveBeenCalled();
  });

  /**
   * Given a report with the save attached that is too large for the relay,
   * when the upload fails,
   * then the player is told that unticking the save makes it smaller — and the save is not
   * dropped for them.
   */
  it("suggests unticking the save when the report is too large", async () => {
    uploadReportBundle.mockRejectedValue("be.error.report.upload.tooLarge");
    await reachPreview();

    fireEvent.click(saveBox());
    fireEvent.click(consentBox());
    fireEvent.click(sendButton());

    expect(await screen.findByRole("alert")).toHaveTextContent("reportBug.tooLargeUntickSave");
    expect(uploadReportBundle).toHaveBeenCalledTimes(1);
    expect(uploadReportBundle).toHaveBeenCalledWith(expect.any(String), true, true);
    expect(saveBox()).toBeChecked();
  });

  /**
   * Given a build with no relay,
   * when the player reaches the preview,
   * then there is no consent box and no Send button — only the GitHub path.
   */
  it("offers only the GitHub path when there is no relay", async () => {
    reportUploadAvailable.mockResolvedValue(false);
    render(<ReportBugModal onClose={vi.fn()} />);
    fillRequired();
    fireEvent.click(screen.getByRole("button", { name: "reportBug.review" }));
    await screen.findByRole("heading", { name: "reportBug.previewTitle" });

    expect(screen.queryByRole("checkbox", { name: /reportBug\.consentLabel/ })).toBeNull();
    expect(screen.queryByRole("button", { name: "reportBug.send" })).toBeNull();
    expect(screen.getByRole("button", { name: "reportBug.saveAndOpen" })).toBeInTheDocument();
  });

  /**
   * Given a backend that has not yet said whether this build can upload,
   * when the player reaches the preview,
   * then only the GitHub path is offered until it does — never a Send that may not work.
   */
  it("offers no Send button while it is unknown whether uploads work", async () => {
    reportUploadAvailable.mockReturnValue(new Promise<boolean>(() => {}));
    render(<ReportBugModal onClose={vi.fn()} />);
    fillRequired();
    fireEvent.click(screen.getByRole("button", { name: "reportBug.review" }));
    await screen.findByRole("heading", { name: "reportBug.previewTitle" });

    expect(screen.queryByRole("button", { name: "reportBug.send" })).toBeNull();
    expect(screen.getByRole("button", { name: "reportBug.saveAndOpen" })).toBeInTheDocument();
  });
});
