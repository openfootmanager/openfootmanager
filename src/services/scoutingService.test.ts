import { beforeEach, describe, expect, it, vi } from "vitest";
import { invoke } from "@tauri-apps/api/core";

import {
  cancelYouthScouting,
  reassignYouthScouting,
  sendScout,
  startYouthScouting,
} from "./scoutingService";

vi.mock("@tauri-apps/api/core", () => ({
  invoke: vi.fn(),
}));

const mockedInvoke = vi.mocked(invoke);

describe("scoutingService", () => {
  beforeEach(() => {
    mockedInvoke.mockReset();
  });

  it("calls the send scout backend command", async () => {
    const response = { manager: { id: "manager-1" } };
    mockedInvoke.mockResolvedValueOnce(response);

    await expect(sendScout("staff-1", "player-1")).resolves.toBe(response);
    expect(mockedInvoke).toHaveBeenCalledWith("send_scout", {
      scoutId: "staff-1",
      playerId: "player-1",
    });
  });

  it("calls the youth scouting backend command", async () => {
    const response = { manager: { id: "manager-1" } };
    mockedInvoke.mockResolvedValueOnce(response);

    await expect(
      startYouthScouting({
        scoutId: "staff-1",
        region: "Domestic",
        objective: "Balanced",
        targetPosition: "Defender",
      }),
    ).resolves.toBe(response);
    expect(mockedInvoke).toHaveBeenCalledWith("start_youth_scouting", {
      scoutId: "staff-1",
      region: "Domestic",
      objective: "Balanced",
      targetPosition: "Defender",
    });
  });

  it("calls the cancel youth scouting backend command", async () => {
    const response = { manager: { id: "manager-1" } };
    mockedInvoke.mockResolvedValueOnce(response);

    await expect(cancelYouthScouting("ysa-1")).resolves.toBe(response);
    expect(mockedInvoke).toHaveBeenCalledWith("cancel_youth_scouting", {
      assignmentId: "ysa-1",
    });
  });

  it("calls the reassign youth scouting backend command", async () => {
    const response = { manager: { id: "manager-1" } };
    mockedInvoke.mockResolvedValueOnce(response);

    await expect(reassignYouthScouting("ysa-1", "staff-2")).resolves.toBe(response);
    expect(mockedInvoke).toHaveBeenCalledWith("reassign_youth_scouting", {
      assignmentId: "ysa-1",
      scoutId: "staff-2",
    });
  });
});