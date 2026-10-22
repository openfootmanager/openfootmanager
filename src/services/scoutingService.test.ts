import { beforeEach, describe, expect, it, vi } from "vitest";
import { invoke } from "@tauri-apps/api/core";

import {
  cancelYouthScouting,
  getPlayerKnowledge,
  getScoutedReport,
  getVisibleWage,
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

  it("asks the backend for a player's scouted report", async () => {
    mockedInvoke.mockResolvedValueOnce(null);

    await expect(getScoutedReport("p1")).resolves.toBeNull();

    expect(mockedInvoke).toHaveBeenCalledWith("get_scouted_report", { playerId: "p1" });
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

  /**
   * Given the backend's knowledge of two players, one the club's own and one scouted
   * When the screen asks about both
   * Then the command is called with their ids and the tagged answer comes back as it is
   */
  it("asks the backend what the club knows about each player", async () => {
    const answer = {
      own: { kind: "own" },
      rival: { kind: "scouted", scouted_on: "2026-06-04", current: true },
    };
    mockedInvoke.mockResolvedValueOnce(answer);

    await expect(getPlayerKnowledge(["own", "rival"])).resolves.toEqual(answer);
    expect(mockedInvoke).toHaveBeenCalledWith("get_player_knowledge", {
      playerIds: ["own", "rival"],
    });
  });

  /**
   * Given no active game
   * When the screen asks what the club knows
   * Then the backend's translated error reaches the caller
   */
  /**
   * Given a scouted player
   * When his profile asks for the wage it may show
   * Then the command is called with his id and the backend's rounded figure comes back as it is
   */
  it("asks the backend for the wage the club may see", async () => {
    mockedInvoke.mockResolvedValueOnce({ kind: "approximate", weekly: 16_000 });

    await expect(getVisibleWage("p1")).resolves.toEqual({ kind: "approximate", weekly: 16_000 });
    expect(mockedInvoke).toHaveBeenCalledWith("get_visible_wage", { playerId: "p1" });
  });

  it("passes on the backend's error when there is no game", async () => {
    mockedInvoke.mockRejectedValueOnce("be.error.noActiveGameSession");

    await expect(getPlayerKnowledge(["p1"])).rejects.toBe("be.error.noActiveGameSession");
  });
});
