import { act, fireEvent, render, screen, waitFor } from "@testing-library/react";
import { beforeEach, describe, expect, it, vi } from "vitest";

const crashNotice = vi.fn();
const dismissCrashNotice = vi.fn();
const logError = vi.fn();

vi.mock("../../services/reportService", () => ({
  crashNotice: () => crashNotice(),
  dismissCrashNotice: () => dismissCrashNotice(),
}));
vi.mock("../../lib/logger", () => ({
  logError: (message: string) => logError(message),
}));
vi.mock("react-i18next", () => ({
  useTranslation: () => ({ t: (key: string) => key, i18n: { language: "en" } }),
}));

import { CrashReportPrompt } from "./CrashReportPrompt";

const NOTICE = { occurred_at: "2026-10-02T21:14:00+00:00", app_version: "0.3.0" };

describe("CrashReportPrompt", () => {
  beforeEach(() => {
    crashNotice.mockReset().mockResolvedValue(NOTICE);
    dismissCrashNotice.mockReset().mockResolvedValue(undefined);
    logError.mockReset();
  });

  /**
   * Given the last session ended in a crash,
   * when the game opens on the main menu,
   * then a dialog offers to report it, saying when it happened and in which version.
   */
  it("offers to report a crash from the last session", async () => {
    render(<CrashReportPrompt onReport={vi.fn()} />);

    const dialog = await screen.findByRole("dialog", { name: "crashPrompt.title" });
    expect(dialog).toHaveTextContent("crashPrompt.body");
    expect(dialog).toHaveTextContent("October 2, 2026");
    expect(dialog).toHaveTextContent("0.3.0");
  });

  /**
   * Given the last session ended cleanly,
   * when the game opens,
   * then no prompt appears.
   */
  it("shows nothing after a clean session", async () => {
    crashNotice.mockResolvedValue(null);
    render(<CrashReportPrompt onReport={vi.fn()} />);

    await waitFor(() => expect(crashNotice).toHaveBeenCalled());
    expect(screen.queryByRole("dialog")).toBeNull();
  });

  /**
   * Given the prompt is showing,
   * when the player chooses Not now,
   * then it closes, the backend records the answer, and no report is started.
   */
  it("goes away for this launch when the player declines", async () => {
    const onReport = vi.fn();
    render(<CrashReportPrompt onReport={onReport} />);
    await screen.findByRole("dialog");

    await act(async () => {
      fireEvent.click(screen.getByRole("button", { name: "crashPrompt.notNow" }));
    });

    expect(screen.queryByRole("dialog")).toBeNull();
    expect(dismissCrashNotice).toHaveBeenCalledTimes(1);
    expect(onReport).not.toHaveBeenCalled();
  });

  /**
   * Given the prompt is showing,
   * when the player chooses to report,
   * then it closes, the answer is recorded, and the report dialog is asked for.
   */
  it("opens the report when the player accepts", async () => {
    const onReport = vi.fn();
    render(<CrashReportPrompt onReport={onReport} />);
    await screen.findByRole("dialog");

    await act(async () => {
      fireEvent.click(screen.getByRole("button", { name: "crashPrompt.report" }));
    });

    expect(screen.queryByRole("dialog")).toBeNull();
    expect(dismissCrashNotice).toHaveBeenCalledTimes(1);
    expect(onReport).toHaveBeenCalledTimes(1);
  });

  /**
   * Given the prompt is showing,
   * when the player presses Escape,
   * then it is treated as Not now.
   */
  it("treats Escape as not now", async () => {
    const onReport = vi.fn();
    render(<CrashReportPrompt onReport={onReport} />);
    await screen.findByRole("dialog");

    await act(async () => {
      fireEvent.keyDown(window, { key: "Escape" });
    });

    expect(screen.queryByRole("dialog")).toBeNull();
    expect(dismissCrashNotice).toHaveBeenCalledTimes(1);
    expect(onReport).not.toHaveBeenCalled();
  });

  /**
   * Given a backend that cannot say whether there was a crash,
   * when the game opens,
   * then no prompt appears and the failure goes to the log.
   */
  it("stays out of the way when the backend cannot answer", async () => {
    crashNotice.mockRejectedValue(new Error("no backend"));
    render(<CrashReportPrompt onReport={vi.fn()} />);

    await waitFor(() => expect(logError).toHaveBeenCalled());
    expect(screen.queryByRole("dialog")).toBeNull();
  });
});
