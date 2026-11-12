import { beforeEach, describe, expect, it, vi } from "vitest";
import { invoke } from "@tauri-apps/api/core";

import {
  applyTacticPreset,
  getSquad,
  getTacticsBlueprintStatus,
  setPlayerSquadRole,
} from "./squadService";

vi.mock("@tauri-apps/api/core", () => ({
  invoke: vi.fn(),
}));

const mockedInvoke = vi.mocked(invoke);

describe("squadService", () => {
  beforeEach(() => {
    mockedInvoke.mockReset();
  });

  it("asks about the team's own style's blueprint, or about a given style's", async () => {
    mockedInvoke.mockResolvedValue({ modified: true });

    await getTacticsBlueprintStatus();
    await getTacticsBlueprintStatus("HighPress");

    expect(mockedInvoke).toHaveBeenNthCalledWith(1, "get_tactics_blueprint_status", undefined);
    expect(mockedInvoke).toHaveBeenNthCalledWith(2, "get_tactics_blueprint_status", {
      playStyle: "HighPress",
    });
  });

  /**
   * Given a preset
   * When it is applied whole, and, in another call, keeping the manager's instructions
   * Then the whole preset sends no `keepInstructions`, and keeping sends `keepInstructions: true`,
   *      both through the one apply_tactic_preset command.
   */
  it("applies a preset whole, or keeping the manager's instructions, in one command", async () => {
    mockedInvoke.mockResolvedValue({});

    await applyTacticPreset("3-4-3", "HighPress");
    await applyTacticPreset("3-4-3", "HighPress", true);

    expect(mockedInvoke).toHaveBeenNthCalledWith(1, "apply_tactic_preset", {
      careerToken: 0,
      formation: "3-4-3",
      playStyle: "HighPress",
    });
    expect(mockedInvoke).toHaveBeenNthCalledWith(2, "apply_tactic_preset", {
      careerToken: 0,
      formation: "3-4-3",
      keepInstructions: true,
      playStyle: "HighPress",
    });
  });

  it("calls the set player squad role backend command", async () => {
    const response = { manager: { id: "manager-1" } };
    mockedInvoke.mockResolvedValueOnce(response);

    await expect(setPlayerSquadRole("player-1", "Youth")).resolves.toBe(response);
    expect(mockedInvoke).toHaveBeenCalledWith("set_player_squad_role", {
      careerToken: 0,
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
