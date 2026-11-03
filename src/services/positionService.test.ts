import { beforeEach, describe, expect, it, vi } from "vitest";
import { invoke } from "@tauri-apps/api/core";

import { getPositionRatings } from "./positionService";

vi.mock("@tauri-apps/api/core", () => ({ invoke: vi.fn() }));
const mockedInvoke = vi.mocked(invoke);

beforeEach(() => {
  mockedInvoke.mockReset();
});

describe("positionService", () => {
  /**
   * Given a player the club knows
   * When his profile asks for his position ratings
   * Then the command is called with his id and the answer comes back as it is
   */
  it("asks the backend for a player's position ratings", async () => {
    const answer = { source: { kind: "live" }, rows: [] };
    mockedInvoke.mockResolvedValueOnce(answer);

    await expect(getPositionRatings("p1")).resolves.toEqual(answer);
    expect(mockedInvoke).toHaveBeenCalledWith("get_position_ratings", { playerId: "p1" });
  });

  /**
   * Given no active game
   * When the ratings are asked for
   * Then the backend's translated error reaches the caller
   */
  it("passes on the backend's error", async () => {
    mockedInvoke.mockRejectedValueOnce("be.error.noActiveGameSession");

    await expect(getPositionRatings("p1")).rejects.toBe("be.error.noActiveGameSession");
  });
});
