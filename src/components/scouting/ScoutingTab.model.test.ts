import { describe, expect, it } from "vitest";

import type { ScoutingAssignment } from "../../store/gameStore";
import { buildAlreadyScoutingIds } from "./ScoutingTab.model";

function createAssignment(overrides: Partial<ScoutingAssignment> = {}): ScoutingAssignment {
  return {
    id: "assignment-1",
    scout_id: "staff-1",
    player_id: "player-1",
    days_remaining: 3,
    ...overrides,
  };
}

describe("ScoutingTab.model", () => {
  it("builds the set of already scouted player ids", () => {
    const ids = buildAlreadyScoutingIds([
      createAssignment({ player_id: "player-1" }),
      createAssignment({ id: "assignment-2", player_id: "player-2" }),
    ]);

    expect(ids.has("player-1")).toBe(true);
    expect(ids.has("player-2")).toBe(true);
    expect(ids.has("player-3")).toBe(false);
  });
});
