import { describe, expect, it } from "vitest";

import type { PlayerData } from "../../store/gameStore";
import type { TacticsLineup } from "../../store/generated/TacticsLineup";
import { buildTableRows } from "./TacticsPlayerTable.helpers";

const player = (id: string): PlayerData => ({ id }) as unknown as PlayerData;

const gk = player("gk");
const st = player("st");
const sub1 = player("sub1");
const sub2 = player("sub2");
const hurt = player("hurt");

const lineup: TacticsLineup = {
  formation: "4-4-2",
  eleven: [
    { position: "Goalkeeper", player_id: "gk", role: "SweeperKeeper", familiarity: "natural" },
    { position: "LeftBack", player_id: "st", role: "Standard", familiarity: "out_of_position" },
  ],
  // The match ranks sub2 above sub1.
  substitutes: ["sub2", "sub1"],
  reserves: ["hurt"],
};

describe("buildTableRows", () => {
  /**
   * Given the eleven, two substitutes and an injured player, and the backend's lineup
   * When the table's rows are built
   * Then the eleven come first by slot, then the substitutes in the order the match ranks them,
   *      then the reserves; each starter carries his slot's position, role and fit, the backend's.
   */
  it("orders the eleven by slot, the substitutes as the match ranks them, then the reserves", () => {
    const rows = buildTableRows(
      [gk, st],
      [sub1, sub2, hurt],
      ["Goalkeeper", "LeftBack"],
      lineup,
      {},
    );

    expect(rows.map((row) => [row.player.id, row.group])).toEqual([
      ["gk", "xi"],
      ["st", "xi"],
      ["sub2", "substitutes"],
      ["sub1", "substitutes"],
      ["hurt", "reserves"],
    ]);
    expect(rows[0]).toMatchObject({
      position: "Goalkeeper",
      role: "SweeperKeeper",
      familiarity: "natural",
    });
    expect(rows[1]).toMatchObject({ position: "LeftBack", familiarity: "out_of_position" });
  });

  /**
   * Given a starter the backend's lineup doesn't name in his slot (a change just made, not yet
   *       answered)
   * When the rows are built
   * Then his slot takes the formation's position and his own stored role, and no fit is claimed.
   */
  it("claims no fit for a slot the backend hasn't described yet", () => {
    const rows = buildTableRows([sub1], [], ["Goalkeeper"], lineup, { sub1: "Stopper" });

    expect(rows[0]).toMatchObject({ position: "Goalkeeper", role: "Stopper", familiarity: null });
  });
});
