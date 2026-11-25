import { describe, expect, it } from "vitest";

import type { PlayerMovementEntry } from "../../store/gameStore";
import { withClubNames } from "./PlayerProfile.movements";

const TEAMS = [
  { id: "c1f5-uuid", name: "North FC" },
  { id: "9ab2-uuid", name: "South FC" },
];

describe("withClubNames", () => {
  /**
   * Given an opening contract and a migrated one, which name their club by id alone (as the game
   *       writes them), the id differing from the name
   * When the clubs are named
   * Then each entry carries the club's name from the game's teams
   */
  it("names a club the ledger recorded only by its id", () => {
    const named = withClubNames(
      [
        { date: "2029-07-01", kind: "initial_contract", to_team_id: "c1f5-uuid" },
        {
          date: "2030-07-01",
          kind: "permanent_transfer",
          from_team_id: "c1f5-uuid",
          to_team_id: "9ab2-uuid",
        },
      ],
      TEAMS,
    );

    expect(named[0].to_team_name).toBe("North FC");
    expect(named[1]).toMatchObject({ from_team_name: "North FC", to_team_name: "South FC" });
  });

  /**
   * Given a recorded name that differs from the game's current one (a club renamed since)
   * When the clubs are named
   * Then the recorded name stands: it is what the ledger says
   */
  it("keeps a name the ledger recorded", () => {
    const entry: PlayerMovementEntry = {
      date: "2029-07-01",
      kind: "initial_contract",
      to_team_id: "c1f5-uuid",
      to_team_name: "Old Name FC",
    };

    expect(withClubNames([entry], TEAMS)[0].to_team_name).toBe("Old Name FC");
  });

  /**
   * Given a club the game no longer has
   * When the clubs are named
   * Then the entry is left unnamed, never given the id as a name
   */
  it("leaves a club the game does not know unnamed", () => {
    const [entry] = withClubNames(
      [{ date: "2029-07-01", kind: "initial_contract", to_team_id: "gone-uuid" }],
      TEAMS,
    );

    expect(entry.to_team_name ?? null).toBeNull();
  });
});
