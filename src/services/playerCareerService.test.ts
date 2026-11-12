import { invoke } from "@tauri-apps/api/core";
import { beforeEach, describe, expect, it, vi } from "vitest";

import { getPlayerCareer } from "./playerCareerService";

vi.mock("@tauri-apps/api/core", () => ({ invoke: vi.fn() }));
const mockedInvoke = vi.mocked(invoke);

beforeEach(() => {
  mockedInvoke.mockReset();
});

describe("playerCareerService", () => {
  /**
   * Given a player the game has
   * When his profile asks for his career
   * Then the command is called with his id (camelCase on this side) and the answer comes back as it is
   */
  it("asks the backend for a player's career", async () => {
    const answer = {
      by_season: [],
      by_club: [],
      totals: { appearances: 0, goals: 0, assists: 0 },
    };
    mockedInvoke.mockResolvedValueOnce(answer);

    await expect(getPlayerCareer("p1")).resolves.toEqual(answer);
    expect(mockedInvoke).toHaveBeenCalledWith("get_player_career", { playerId: "p1" });
  });

  /**
   * Given an id no active player has
   * When the career is asked for
   * Then the null the backend answers reaches the caller, not an empty career
   */
  it("passes on a missing player as null", async () => {
    mockedInvoke.mockResolvedValueOnce(null);

    await expect(getPlayerCareer("ghost")).resolves.toBeNull();
  });

  /**
   * Given no active game
   * When the career is asked for
   * Then the backend's translated error reaches the caller
   */
  it("passes on the backend's error", async () => {
    mockedInvoke.mockRejectedValueOnce("be.error.noActiveGameSession");

    await expect(getPlayerCareer("p1")).rejects.toBe("be.error.noActiveGameSession");
  });
});
