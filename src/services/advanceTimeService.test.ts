import { beforeEach, describe, expect, it, vi } from "vitest";
import { invoke } from "@tauri-apps/api/core";

import {
  advanceTimeWithMode,
  checkBlockingActions,
  skipToMatchDay,
} from "./advanceTimeService";

vi.mock("@tauri-apps/api/core", () => ({
  invoke: vi.fn(),
}));

const mockedInvoke = vi.mocked(invoke);

describe("advanceTimeService", () => {
  beforeEach(() => {
    mockedInvoke.mockReset();
  });

  it("calls the advance-time backend command with the requested mode", async () => {
    const response = { action: "advanced" };
    mockedInvoke.mockResolvedValueOnce(response);

    await expect(advanceTimeWithMode("delegate")).resolves.toBe(response);

    expect(mockedInvoke).toHaveBeenCalledWith("advance_time_with_mode", {
      mode: "delegate",
    });
  });

  it("returns blocker data when the blocker command succeeds", async () => {
    const blockers = [
      {
        id: "urgent_messages",
        severity: "info",
        text: "1 urgent unread message(s)",
        tab: "Inbox",
      },
    ];
    mockedInvoke.mockResolvedValueOnce(blockers);

    await expect(checkBlockingActions("test")).resolves.toEqual(blockers);
    expect(mockedInvoke).toHaveBeenCalledWith("check_blocking_actions");
  });

  it("falls back to an empty blocker list when the blocker command fails", async () => {
    const consoleWarnSpy = vi
      .spyOn(console, "warn")
      .mockImplementation(() => { });
    mockedInvoke.mockRejectedValueOnce(new Error("boom"));

    try {
      await expect(checkBlockingActions("test")).resolves.toEqual([]);
    } finally {
      consoleWarnSpy.mockRestore();
    }
  });

  it("calls the skip-to-match-day backend command", async () => {
    const response = { action: "advanced", days_skipped: 3 };
    mockedInvoke.mockResolvedValueOnce(response);

    await expect(skipToMatchDay()).resolves.toBe(response);
    expect(mockedInvoke).toHaveBeenCalledWith("skip_to_match_day");
  });
});