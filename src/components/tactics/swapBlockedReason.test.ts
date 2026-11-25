import { describe, expect, it } from "vitest";

import { createPlayer } from "../../test-utils/factories";
import { swapVerdict } from "./swapBlockedReason";

const xi = ["g", "d1", "d2"];
const player = (id: string, eligible: boolean | undefined) =>
  createPlayer({ id, match_day_eligible: eligible });

describe("swapVerdict", () => {
  /**
   * Given a starter and a bench player the backend has ruled out of the match day
   * When the swap is read
   * Then it cannot be confirmed, and the reason names that bench player as unavailable
   */
  it("names a ruled-out bench player as the reason a swap is off", () => {
    const verdict = swapVerdict({
      selected: { player: player("d1", true), section: "xi" },
      compare: { player: player("b1", false), section: "bench" },
      startingXiIds: xi,
    });

    expect(verdict).toEqual({
      canConfirm: false,
      reason: { kind: "unavailable", playerId: "b1" },
    });
  });

  /**
   * Given a bench player whose match-day verdict the squad fetch has not brought yet
   * When the swap is read
   * Then it waits ("unchecked"), and does not claim the player is unavailable
   */
  it("says unchecked, not unavailable, while the verdict is absent", () => {
    const verdict = swapVerdict({
      selected: { player: player("b1", undefined), section: "bench" },
      compare: { player: player("d1", true), section: "xi" },
      startingXiIds: xi,
    });

    expect(verdict).toEqual({
      canConfirm: false,
      reason: { kind: "unchecked", playerId: "b1" },
    });
  });

  /**
   * Given two bench players, both eligible
   * When the swap is read
   * Then it would not change the eleven, and says so
   */
  it("says a swap that leaves the eleven as it is would change nothing", () => {
    const verdict = swapVerdict({
      selected: { player: player("b1", true), section: "bench" },
      compare: { player: player("b2", true), section: "bench" },
      startingXiIds: xi,
    });

    expect(verdict).toEqual({ canConfirm: false, reason: { kind: "unchanged" } });
  });

  /**
   * Given a starter and an eligible bench player
   * When the swap is read
   * Then it can be confirmed and there is no reason
   */
  it("confirms a swap of a starter for an eligible bench player", () => {
    const verdict = swapVerdict({
      selected: { player: player("d1", true), section: "xi" },
      compare: { player: player("b1", true), section: "bench" },
      startingXiIds: xi,
    });

    expect(verdict).toEqual({ canConfirm: true, reason: null });
  });

  /**
   * Given only one player picked
   * When the swap is read
   * Then there is nothing to confirm and nothing to explain
   */
  it("has no reason before a second player is picked", () => {
    const verdict = swapVerdict({
      selected: { player: player("d1", true), section: "xi" },
      compare: null,
      startingXiIds: xi,
    });

    expect(verdict).toEqual({ canConfirm: false, reason: null });
  });
});
