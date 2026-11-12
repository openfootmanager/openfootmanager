import { describe, expect, it } from "vitest";

import { isSnapshotNotOlder } from "./gameSnapshot";

describe("isSnapshotNotOlder", () => {
  /**
   * Given no game held yet (the store starts empty, also after a reload)
   * When a snapshot arrives
   * Then it is applied, whatever its numbers.
   */
  it("applies a snapshot when nothing is held", () => {
    expect(isSnapshotNotOlder(null, { career_token: 4, revision: 9 })).toBe(true);
    expect(isSnapshotNotOlder(undefined, {})).toBe(true);
  });

  /**
   * Given a game of career 3 at revision 5 is held
   * When a snapshot of the same career arrives
   * Then a newer or equal revision is applied (equal is the same content), and an older one is
   *      not: a slow refetch never replaces a newer game.
   */
  it.each([
    [6, true],
    [5, true],
    [4, false],
    [0, false],
  ])("same career: revision %i against a held 5 is applied: %s", (revision, applied) => {
    expect(
      isSnapshotNotOlder({ career_token: 3, revision: 5 }, { career_token: 3, revision }),
    ).toBe(applied);
  });

  /**
   * Given a game of career 3 at a high revision is held
   * When a snapshot of a later career (a new or loaded game: a higher token, revision 0) arrives
   * Then it is applied, and a snapshot of an earlier career is not, however high its revision.
   */
  it("orders by career first: a later career wins at revision 0, an earlier one never does", () => {
    const held = { career_token: 3, revision: 90 };

    expect(isSnapshotNotOlder(held, { career_token: 4, revision: 0 })).toBe(true);
    expect(isSnapshotNotOlder(held, { career_token: 2, revision: 500 })).toBe(false);
  });

  /**
   * Given a snapshot that carries neither number (a state built before the fields existed)
   * When it is compared with another such one
   * Then both read as 0 and it is applied, so nothing that never had a revision is refused.
   */
  it("reads a missing token and revision as 0", () => {
    expect(isSnapshotNotOlder({}, {})).toBe(true);
    expect(isSnapshotNotOlder({ revision: 2 }, {})).toBe(false);
    expect(isSnapshotNotOlder({}, { revision: 1 })).toBe(true);
  });
});
