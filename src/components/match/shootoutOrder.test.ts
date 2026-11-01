import { describe, expect, it } from "vitest";

import { isOrderChanged, moveKicker, placeInOrder, putBack } from "./shootoutOrder";

const AUTO = ["taker", "striker", "winger", "back", "keeper"] as const;

describe("moveKicker", () => {
  /**
   * Given the auto order
   * When the third player is moved up one place
   * Then he and the second swap, and everyone else keeps their place.
   */
  it("moves a player up one place, swapping with the one above", () => {
    expect(moveKicker(AUTO, 2, 1)).toEqual(["taker", "winger", "striker", "back", "keeper"]);
  });

  /**
   * Given the auto order
   * When the first player is moved to fourth
   * Then the players he passes each close up one place.
   */
  it("moves a player down several places, the others closing up", () => {
    expect(moveKicker(AUTO, 0, 3)).toEqual(["striker", "winger", "back", "taker", "keeper"]);
  });

  /**
   * Given the first player, or the last
   * When he is moved past the end of the order
   * Then the order is the very same one, so nothing on screen changes.
   */
  it("leaves the order alone for a move past either end", () => {
    expect(moveKicker(AUTO, 0, -1)).toBe(AUTO);
    expect(moveKicker(AUTO, 4, 5)).toBe(AUTO);
    expect(moveKicker(AUTO, 2, 2)).toBe(AUTO);
  });
});

describe("putBack", () => {
  /**
   * Given the striker lifted from second and carried down to fifth
   * When he is put back
   * Then the order is the one before he was lifted.
   */
  it("returns a lifted player to the place he was lifted from", () => {
    const carried = moveKicker(moveKicker(moveKicker(AUTO, 1, 2), 2, 3), 3, 4);
    expect(carried).toEqual(["taker", "winger", "back", "keeper", "striker"]);

    expect(putBack(carried, "striker", 1)).toEqual([...AUTO]);
  });

  /** Given a player who is not in the order, then putting him back changes nothing. */
  it("leaves the order alone for a player who is not in it", () => {
    expect(putBack(AUTO, "stranger", 0)).toBe(AUTO);
  });
});

describe("isOrderChanged", () => {
  /**
   * Given an order moved and then moved back
   * When it is compared with the auto order
   * Then it is unchanged, so starting the shootout sends nothing.
   */
  it("calls an order moved back to where it was unchanged", () => {
    const back = moveKicker(moveKicker(AUTO, 0, 1), 1, 0);
    expect(isOrderChanged(AUTO, back)).toBe(false);
  });

  /** Given two players swapped, then the order has changed. */
  it("calls any swap a change", () => {
    expect(isOrderChanged(AUTO, moveKicker(AUTO, 3, 4))).toBe(true);
  });
});

describe("placeInOrder", () => {
  /**
   * Given three kickers a side and five players in the order
   * When each place is read
   * Then the first three are kicks 1 to 3 and the last two sit out.
   */
  it("numbers the kickers and sits out the rest", () => {
    expect([0, 1, 2, 3, 4].map((index) => placeInOrder(index, 3))).toEqual([
      { kick: 1 },
      { kick: 2 },
      { kick: 3 },
      null,
      null,
    ]);
  });
});
