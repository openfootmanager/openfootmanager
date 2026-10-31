import { invoke } from "@tauri-apps/api/core";
import { beforeEach, describe, expect, it, vi } from "vitest";

import { getMatchSnapshot, stepLiveMatch } from "./liveMatchService";

vi.mock("@tauri-apps/api/core", () => ({ invoke: vi.fn() }));

const mockedInvoke = vi.mocked(invoke);

describe("liveMatchService", () => {
  beforeEach(() => {
    mockedInvoke.mockReset();
  });

  /**
   * Given a live match
   * When the screen steps it by one
   * Then the backend's step command is asked for exactly one minute, and its results come back.
   */
  it("steps the live match by the minutes asked for", async () => {
    const results = [{ minute: 121, phase: "PenaltyShootout", events: [] }];
    mockedInvoke.mockResolvedValueOnce(results);

    await expect(stepLiveMatch(1)).resolves.toBe(results);
    expect(mockedInvoke).toHaveBeenCalledWith("step_live_match", { minutes: 1 });
  });

  /**
   * Given a live match
   * When the screen asks where it stands
   * Then the backend's snapshot command answers.
   */
  it("reads the live match's snapshot", async () => {
    const snapshot = { phase: "PenaltyShootout" };
    mockedInvoke.mockResolvedValueOnce(snapshot);

    await expect(getMatchSnapshot()).resolves.toBe(snapshot);
    expect(mockedInvoke).toHaveBeenCalledWith("get_match_snapshot");
  });
});
