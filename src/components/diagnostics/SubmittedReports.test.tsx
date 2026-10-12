import { render, screen } from "@testing-library/react";
import { beforeEach, describe, expect, it, vi } from "vitest";

const listSubmittedReports = vi.fn();
const logError = vi.fn();

vi.mock("../../services/reportService", () => ({
  listSubmittedReports: () => listSubmittedReports(),
}));
vi.mock("../../lib/logger", () => ({
  logError: (message: string) => logError(message),
}));
vi.mock("react-i18next", () => ({
  useTranslation: () => ({ t: (key: string) => key, i18n: { language: "en" } }),
}));

import { SubmittedReports } from "./SubmittedReports";

const REPORTS = [
  {
    code: "BBBBBBBB",
    submitted_at: "2026-10-02T09:30:00+00:00",
    included_save: true,
    app_version: "0.3.0",
    id: "e1",
    unconfirmed: false,
  },
  {
    code: "AAAAAAAA",
    submitted_at: "2026-09-14T18:00:00+00:00",
    included_save: false,
    app_version: "0.3.0",
    id: "e2",
    unconfirmed: false,
  },
];

describe("SubmittedReports", () => {
  beforeEach(() => {
    listSubmittedReports.mockReset().mockResolvedValue(REPORTS);
    logError.mockReset();
  });

  /**
   * Given an install that has sent two reports,
   * when Settings → Help is open,
   * then both codes are listed in the order the backend gives (newest first), each with its date.
   */
  it("lists every code this install has sent, newest first, with its date", async () => {
    render(<SubmittedReports refreshKey={0} />);

    const items = await screen.findAllByRole("listitem");
    expect(items).toHaveLength(2);
    expect(items[0]).toHaveTextContent("BBBBBBBB");
    expect(items[0]).toHaveTextContent("October 2, 2026");
    expect(items[1]).toHaveTextContent("AAAAAAAA");
  });

  /**
   * Given a report sent with the player's save attached,
   * when it is listed,
   * then it says so — the save is the part a player is most likely to want removed.
   */
  it("marks the reports that carried a save", async () => {
    render(<SubmittedReports refreshKey={0} />);

    const items = await screen.findAllByRole("listitem");
    expect(items[0]).toHaveTextContent("settings.reportIncludedSave");
    expect(items[1]).not.toHaveTextContent("settings.reportIncludedSave");
  });

  /**
   * Given an install that has never sent a report,
   * when Settings → Help is open,
   * then the row says there are none rather than showing an empty list.
   */
  it("says so when no report has been sent", async () => {
    listSubmittedReports.mockResolvedValue([]);
    render(<SubmittedReports refreshKey={0} />);

    expect(await screen.findByText("settings.noReports")).toBeInTheDocument();
    expect(screen.queryByRole("list")).toBeNull();
  });

  /**
   * Given a backend that cannot read the list,
   * when Settings → Help is open,
   * then the row shows the empty state and the failure goes to the log.
   */
  it("shows the empty state and logs when the list cannot be read", async () => {
    listSubmittedReports.mockRejectedValue(new Error("no backend"));
    render(<SubmittedReports refreshKey={0} />);

    expect(await screen.findByText("settings.noReports")).toBeInTheDocument();
    expect(logError).toHaveBeenCalled();
  });

  /**
   * Given the row is already showing,
   * when a report is sent and the refresh key changes,
   * then the list is read again, so the new code appears without leaving the page.
   */
  it("reads the list again when asked to refresh", async () => {
    listSubmittedReports.mockResolvedValueOnce([]);
    const { rerender } = render(<SubmittedReports refreshKey={0} />);
    await screen.findByText("settings.noReports");

    rerender(<SubmittedReports refreshKey={1} />);

    expect(await screen.findAllByRole("listitem")).toHaveLength(2);
    expect(listSubmittedReports).toHaveBeenCalledTimes(2);
  });

  /**
   * Given an attempt whose outcome the game could not learn (no code was saved)
   * When the list is shown
   * Then that entry says the outcome is unknown, with its date, instead of an empty code
   */
  it("shows an unknown outcome in place of a code", async () => {
    listSubmittedReports.mockResolvedValue([
      {
        code: "",
        submitted_at: "2026-10-12T10:00:00+00:00",
        included_save: false,
        app_version: "0.3.0",
        id: "e3",
        unconfirmed: true,
      },
      ...REPORTS,
    ]);

    render(<SubmittedReports refreshKey={0} />);

    expect(await screen.findByText("settings.reportOutcomeUnknown")).toBeInTheDocument();
    expect(screen.getAllByRole("button", { name: /copyCode/ })).toHaveLength(2);
  });

  /**
   * Given two entries with the same code and time and no ids (older files),
   * when the list renders
   * Then React reports no duplicate-key warning
   */
  it("keys entries without ids by position so equal entries do not collide", async () => {
    const consoleError = vi.spyOn(console, "error").mockImplementation(() => {});
    const twin = {
      code: "OFM-7K2QX9DM",
      submitted_at: "2026-10-12T10:00:00+00:00",
      included_save: false,
      app_version: "0.3.0",
      id: "",
      unconfirmed: false,
    };
    listSubmittedReports.mockResolvedValue([twin, { ...twin }]);

    render(<SubmittedReports refreshKey={0} />);

    await screen.findAllByText("OFM-7K2QX9DM");
    const keyWarnings = consoleError.mock.calls.filter((call) =>
      String(call[0]).includes("same key"),
    );
    consoleError.mockRestore();
    expect(keyWarnings).toHaveLength(0);
  });
});
