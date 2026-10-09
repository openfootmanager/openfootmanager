import { beforeEach, describe, expect, it, vi } from "vitest";
import { invoke } from "@tauri-apps/api/core";

import { getSquad, setPlayerSquadRole } from "./squadService";

vi.mock("@tauri-apps/api/core", () => ({
  invoke: vi.fn(),
}));

const mockedInvoke = vi.mocked(invoke);

describe("squadService", () => {
  beforeEach(() => {
    mockedInvoke.mockReset();
  });

  it("calls the set player squad role backend command", async () => {
    const response = { manager: { id: "manager-1" } };
    mockedInvoke.mockResolvedValueOnce(response);

    await expect(setPlayerSquadRole("player-1", "Youth")).resolves.toBe(response);
    expect(mockedInvoke).toHaveBeenCalledWith("set_player_squad_role", {
      playerId: "player-1",
      squadRole: "Youth",
    });
  });
  // Given the backend squad projection, when the service fetches it, then the
  // eligibility verdict reaches the UI without a client-side eligibility rule.
  it("passes backend match-day eligibility through the roster service", async () => {
    const response = [{ id: "youth", squad_role: "Youth", match_day_eligible: true }];
    mockedInvoke.mockResolvedValueOnce(response);
    await expect(getSquad("club")).resolves.toBe(response);
    expect(mockedInvoke).toHaveBeenCalledWith("get_squad", { teamId: "club" });
  });

  // Given a missing backend session, when the squad is fetched, then the service
  // rejects with the backend error so its caller can handle it.
  it("propagates a roster request failure", async () => {
    mockedInvoke.mockRejectedValueOnce("be.error.noActiveGameSession");
    await expect(getSquad("club")).rejects.toBe("be.error.noActiveGameSession");
  });
});
