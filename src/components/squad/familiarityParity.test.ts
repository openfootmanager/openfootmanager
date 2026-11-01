import { describe, expect, it } from "vitest";

import type { PlayerData } from "../../store/gameStore";
import { cannedPositionTable } from "../../test-utils/positionTable";
import fixture from "./familiarity.generated.json";
import { getSquadTacticalFit } from "./SquadTab.helpers";

/** The backend's three words, and the tiers the screens still compute for them. */
const TIER_OF = { natural: "natural", can_play: "adapted", out_of_position: "out" } as const;

/** The slots a formation names: every position but the three legacy buckets. */
const BUCKETS = new Set(["Defender", "Midfielder", "Forward"]);

interface FixtureRow {
  position: string;
  natural_position: string;
  alternate_positions: string[];
  familiarity: Record<string, keyof typeof TIER_OF>;
}

const label = (row: FixtureRow): string =>
  `${row.natural_position} (position ${row.position}, alternates [${row.alternate_positions.join(", ")}])`;

describe("a player's fit for a slot, as the squad and tactics screens grade it", () => {
  /**
   * Given the backend's own answer (`player_rating::familiarity`) for every position as a natural
   *       position, with and without an alternate, at every slot a formation names
   * When the screens' helper grades the same player in the same slot
   * Then it gives the tier of the backend's word: natural, adapted for can play, out for out of
   *      position. Every disagreement is listed, so a divergence is read in one run.
   */
  it("agrees with the backend's familiarity at every slot", () => {
    const table = cannedPositionTable();
    const disagreements: string[] = [];
    // The grid is 52 players and a natural goalkeeper listed as a centre back; a shorter or empty
    // fixture must not pass by having nothing to compare.
    expect(fixture).toHaveLength(53);

    for (const row of fixture as FixtureRow[]) {
      const player = {
        position: row.position,
        natural_position: row.natural_position,
        alternate_positions: row.alternate_positions,
      } as unknown as PlayerData;
      for (const [slot, word] of Object.entries(row.familiarity)) {
        if (BUCKETS.has(slot)) continue;
        const grade = getSquadTacticalFit(player, slot, table);
        if (grade !== TIER_OF[word]) {
          disagreements.push(`${label(row)} at ${slot}: backend ${word}, screens ${grade}`);
        }
      }
    }

    expect(disagreements).toEqual([]);
  });
});
