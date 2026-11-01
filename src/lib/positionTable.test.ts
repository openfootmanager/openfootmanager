import { describe, expect, it } from "vitest";

import { createPositionTable, type PositionEntry } from "./positionTable";

/** A slice of the backend's table: the four buckets and a few granular positions. */
const ENTRIES: PositionEntry[] = [
  { position: "Goalkeeper", group: "Goalkeeper", rank: 10 },
  { position: "Defender", group: "Defender", rank: 20 },
  { position: "CenterBack", group: "Defender", rank: 21 },
  { position: "RightBack", group: "Defender", rank: 23 },
  { position: "Midfielder", group: "Midfielder", rank: 30 },
  { position: "DefensiveMidfielder", group: "Midfielder", rank: 31 },
  { position: "Forward", group: "Forward", rank: 40 },
  { position: "Striker", group: "Forward", rank: 43 },
];

describe("createPositionTable", () => {
  const table = createPositionTable(ENTRIES);

  /**
   * Given a table that puts a wing-back-like "RightBack" in the defender group
   * When groups are asked for by name and by the short alias the squad screens use
   * Then both read the table's group, and a position it does not name is its own group.
   */
  it("reads a position's group from the table, through its aliases", () => {
    expect(table.groupOf("CenterBack")).toBe("Defender");
    expect(table.groupOf("RB")).toBe("Defender");
    expect(table.groupOf("Unheard Of")).toBe("Unheard Of");
  });

  /**
   * Given the table's ranks
   * When positions are ranked singly and by their line
   * Then a granular position keeps its own place, its line ties with its bucket, and an unknown
   *      position sorts after every known one.
   */
  it("ranks a position and its line", () => {
    expect(table.rankOf("CenterBack")).toBe(21);
    expect(table.lineRankOf("CenterBack")).toBe(20);
    expect(table.lineRankOf("Defender")).toBe(20);
    expect(table.rankOf("Unheard Of")).toBe(999);
    expect(table.lineRankOf("Unheard Of")).toBe(999);
  });

  /**
   * Given a group with detailed members
   * When its members are asked for
   * Then they come in pitch order without the bucket itself; a group with none is just itself.
   */
  it("lists a group's detailed members in pitch order", () => {
    // The table arrives in whatever order the backend writes it; the members come back by rank.
    const shuffled = createPositionTable([...ENTRIES].reverse());

    expect(shuffled.membersOf("Defender")).toEqual(["CenterBack", "RightBack"]);
    expect(table.membersOf("Defender")).toEqual(["CenterBack", "RightBack"]);
    expect(table.membersOf("Goalkeeper")).toEqual(["Goalkeeper"]);
  });

  /**
   * Given the table has not arrived
   * When anything is asked of it
   * Then no group is invented and every position ties at the end, instead of a guessed order.
   */
  it("answers nothing invented before the table arrives", () => {
    const empty = createPositionTable([]);

    expect(empty.loaded).toBe(false);
    expect(table.loaded).toBe(true);
    expect(empty.groupOf("CenterBack")).toBe("CenterBack");
    expect(empty.lineRankOf("CenterBack")).toBe(999);
    expect(empty.membersOf("Defender")).toEqual(["Defender"]);
  });

  /**
   * Given a table that arrives in any order, and one that has not arrived
   * When the lines are asked for
   * Then they come goalkeeper first by rank, the buckets only; before arrival there are none.
   */
  it("lists the lines in pitch order, and none before the table arrives", () => {
    expect(createPositionTable([...ENTRIES].reverse()).lines()).toEqual([
      "Goalkeeper",
      "Defender",
      "Midfielder",
      "Forward",
    ]);
    expect(createPositionTable([]).lines()).toEqual([]);
  });
});
