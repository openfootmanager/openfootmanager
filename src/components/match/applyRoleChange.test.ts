import { invoke } from "@tauri-apps/api/core";
import { beforeEach, describe, expect, it, vi } from "vitest";

import { applyRoleChange } from "./applyRoleChange";
import type { MatchSnapshot } from "./types";

vi.mock("@tauri-apps/api/core", () => ({ invoke: vi.fn() }));

const mockedInvoke = vi.mocked(invoke);

describe("applyRoleChange", () => {
  beforeEach(() => {
    mockedInvoke.mockReset();
  });

  /**
   * Given the engine accepts the role
   * When the change is applied
   * Then the engine hears the side, player and role, the screen gets the new snapshot, and the
   *      change reports success.
   */
  it("sends the role to the engine and publishes the snapshot", async () => {
    const snapshot = { home_subs_made: 0 } as unknown as MatchSnapshot;
    mockedInvoke.mockResolvedValueOnce(snapshot);
    const onSnapshot = vi.fn();

    const outcome = await applyRoleChange("Home", "p9", "Poacher", onSnapshot);

    expect(outcome).toEqual({ applied: true });
    expect(mockedInvoke).toHaveBeenCalledWith("apply_match_command", {
      command: { ChangePlayerRole: { side: "Home", player_id: "p9", role: "Poacher" } },
    });
    expect(onSnapshot).toHaveBeenCalledWith(snapshot);
  });

  /**
   * Given the engine refuses the role
   * When the change is applied
   * Then the refusal's key is returned and the screen's snapshot is left alone.
   */
  it("returns the engine's refusal and publishes nothing", async () => {
    mockedInvoke.mockRejectedValueOnce("be.error.roleNotValidForPosition");
    const onSnapshot = vi.fn();
    vi.spyOn(console, "error").mockImplementation(() => {});

    const outcome = await applyRoleChange("Away", "p9", "SweeperKeeper", onSnapshot);

    expect(outcome).toEqual({ applied: false, reason: "be.error.roleNotValidForPosition" });
    expect(onSnapshot).not.toHaveBeenCalled();
  });
});
