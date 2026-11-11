import { describe, expect, it } from "vitest";

import { createPlayer } from "../../test-utils/factories";
import { keyPlayers } from "./keyPlayers";

describe("keyPlayers", () => {
  const xi = ["a", "b", "c", "d", "e", "f", "g"].map((id) =>
    createPlayer({ id, match_name: id.toUpperCase(), natural_position: "Striker" }),
  );
  const ratings = new Map([
    ["a", 61],
    ["b", 77],
    ["c", 70],
    ["d", 65],
    ["e", 80],
    ["f", 59],
  ]);

  /**
   * Given a likely eleven rated by the backend, one of whom it has not rated
   * When the key players are taken
   * Then they are the five best-rated, best first, and the unrated player is left out rather than
   *      ranked as zero.
   */
  it("takes the five best-rated players, best first, and skips an unrated one", () => {
    const players = keyPlayers(
      xi,
      (id) => ratings.get(id) ?? null,
      () => 25,
    );

    expect(players.map((player) => [player.id, player.ovr])).toEqual([
      ["e", 80],
      ["b", 77],
      ["c", 70],
      ["d", 65],
      ["a", 61],
    ]);
  });

  /**
   * Given a player with a natural position and a name the match uses
   * When he is a key player
   * Then his entry carries that position, that name and the age the caller works out.
   */
  it("carries his natural position, match name and age", () => {
    const [first] = keyPlayers(
      xi,
      (id) => ratings.get(id) ?? null,
      () => 27,
    );

    expect(first).toMatchObject({ name: "E", position: "Striker", age: 27 });
  });

  it("is empty before the backend has rated anyone", () => {
    expect(
      keyPlayers(
        xi,
        () => null,
        () => 25,
      ),
    ).toEqual([]);
  });
});
