import { describe, expect, it } from "vitest";

import type { PlayerData } from "../../store/gameStore";
import { createPlayer } from "../../test-utils/factories";
import { sortSquadRoster } from "./SquadRosterView.sort";

/** Positions in the order the pitch reads them: keeper, defence, midfield, attack. */
const POSITION_RANK: Record<string, number> = {
  Goalkeeper: 0,
  Defender: 1,
  Midfielder: 2,
  Forward: 3,
};

const player = (id: string, overrides: Partial<PlayerData> = {}): PlayerData =>
  createPlayer({
    id,
    match_name: id,
    full_name: id,
    position: "Midfielder",
    natural_position: "Midfielder",
    date_of_birth: "1998-01-01",
    ...overrides,
  });

function sorted(
  players: PlayerData[],
  key: Parameters<typeof sortSquadRoster>[1]["sortKey"],
  sortDir: "asc" | "desc",
  starters: string[] = [],
): string[] {
  return sortSquadRoster(
    players,
    { sortKey: key, sortDir },
    {
      isStarter: (candidate) => starters.includes(candidate.id),
      positionRankOf: (position) => POSITION_RANK[position] ?? 99,
    },
    "en",
  ).map((candidate) => candidate.id);
}

describe("sortSquadRoster", () => {
  /**
   * Given three players, one with no shirt number
   * When the roster is sorted by number, up and then down
   * Then the numbers run in order both ways and the player with none is last both times, not
   *      first going down.
   */
  it("puts a player with no shirt number last in both directions", () => {
    const players = [
      player("seven", { jersey_number: 7 }),
      player("none", { jersey_number: null }),
      player("two", { jersey_number: 2 }),
    ];
    expect(sorted(players, "jersey", "asc")).toEqual(["two", "seven", "none"]);
    expect(sorted(players, "jersey", "desc")).toEqual(["seven", "two", "none"]);
  });

  /**
   * Given three players, one with no contract end
   * When the roster is sorted by contract, up and then down
   * Then the end dates run in order both ways, and the player with none is last both times.
   */
  it("puts a player with no contract end last in both directions", () => {
    const players = [
      player("late", { contract_end: "2030-06-30" }),
      player("none", { contract_end: null }),
      player("soon", { contract_end: "2027-06-30" }),
    ];
    expect(sorted(players, "contract", "asc")).toEqual(["soon", "late", "none"]);
    expect(sorted(players, "contract", "desc")).toEqual(["late", "soon", "none"]);
  });

  /**
   * Given two players of the same morale, "first" listed before "second", and a third higher
   * When the roster is sorted by morale, up and then down
   * Then "first" stays ahead of "second" both ways: a tie keeps the roster's order, where reversing
   *      a sorted list would swap them.
   */
  it("keeps tied players in the roster's order in both directions", () => {
    const players = [
      player("first", { morale: 70 }),
      player("second", { morale: 70 }),
      player("high", { morale: 90 }),
    ];
    expect(sorted(players, "morale", "asc")).toEqual(["first", "second", "high"]);
    expect(sorted(players, "morale", "desc")).toEqual(["high", "first", "second"]);
  });

  /**
   * Given a starting forward, two starting defenders of 60 and 80, and a bench keeper
   * When the roster is sorted by position (the default view)
   * Then the starters come first; among them the defence comes before the attack, and the stronger
   *      defender before the weaker; then the bench.
   */
  it("orders by position as XI first, then position order, then rating high to low", () => {
    const players = [
      player("bench-keeper", { position: "Goalkeeper", natural_position: "Goalkeeper", ovr: 90 }),
      player("forward", { position: "Forward", natural_position: "Forward", ovr: 75 }),
      player("weak-defender", { position: "Defender", natural_position: "Defender", ovr: 60 }),
      player("strong-defender", { position: "Defender", natural_position: "Defender", ovr: 80 }),
    ];
    const starters = ["forward", "weak-defender", "strong-defender"];
    expect(sorted(players, "pos", "asc", starters)).toEqual([
      "strong-defender",
      "weak-defender",
      "forward",
      "bench-keeper",
    ]);
  });
});
