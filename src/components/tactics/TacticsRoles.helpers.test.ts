import { describe, expect, it } from "vitest";

import type { AutoSetPieces } from "../../services/squadService";
import type { PlayerData, TeamMatchRolesData } from "../../store/gameStore";
import {
  EMPTY_MATCH_ROLES,
  buildUpdatedMatchRolesForAssignment,
  resolveEffectiveMatchRoles,
} from "./TacticsRoles.helpers";

const xi = ["a", "b", "c"].map((id) => ({ id }) as PlayerData);

const backend: AutoSetPieces = {
  captain: "a",
  penalty_taker: "b",
  free_kick_taker: "c",
  corner_taker: "a",
};

describe("resolveEffectiveMatchRoles", () => {
  /**
   * Given nothing assigned by the manager
   * When the roles are resolved
   * Then each is what the backend would use for the match, not a ranking of our own.
   */
  it("takes the backend's pick for every role the manager has not assigned", () => {
    expect(resolveEffectiveMatchRoles(xi, EMPTY_MATCH_ROLES, backend)).toEqual({
      captain: "a",
      vice_captain: null,
      penalty_taker: "b",
      free_kick_taker: "c",
      corner_taker: "a",
    });
  });

  /**
   * Given a stored taker who is in the eleven
   * When the roles are resolved
   * Then the manager's choice wins over the backend's.
   */
  it("keeps a stored role whose player is in the eleven", () => {
    const stored: TeamMatchRolesData = { ...EMPTY_MATCH_ROLES, penalty_taker: "c" };

    expect(resolveEffectiveMatchRoles(xi, stored, backend).penalty_taker).toBe("c");
  });

  /**
   * Given a stored taker who has left the eleven
   * When the roles are resolved
   * Then the backend's pick stands in.
   */
  it("replaces a stored role whose player is no longer in the eleven", () => {
    const stored: TeamMatchRolesData = { ...EMPTY_MATCH_ROLES, captain: "gone" };

    expect(resolveEffectiveMatchRoles(xi, stored, backend).captain).toBe("a");
  });

  /**
   * Given the backend has not answered yet, or names someone outside the eleven
   * When the roles are resolved
   * Then the role is left unassigned rather than guessed.
   */
  it("leaves a role unassigned when the backend has no usable pick", () => {
    expect(resolveEffectiveMatchRoles(xi, EMPTY_MATCH_ROLES, null).captain).toBeNull();
    expect(
      resolveEffectiveMatchRoles(xi, EMPTY_MATCH_ROLES, { ...backend, captain: "outsider" })
        .captain,
    ).toBeNull();
  });

  /**
   * Given a stored vice-captain
   * When the roles are resolved
   * Then he is kept; with none stored the slot stays empty, since the backend has no rule for it.
   */
  it("keeps a stored vice-captain and never invents one", () => {
    const stored: TeamMatchRolesData = { ...EMPTY_MATCH_ROLES, vice_captain: "b" };

    expect(resolveEffectiveMatchRoles(xi, stored, backend).vice_captain).toBe("b");
    expect(resolveEffectiveMatchRoles(xi, EMPTY_MATCH_ROLES, backend).vice_captain).toBeNull();
  });
});

describe("an assignment followed by the next resolution", () => {
  /**
   * Given the backend's captain is "a" and the manager makes "a" vice-captain
   * When the saved roles are resolved again
   * Then "a" is not handed the captaincy back by the fallback: no one holds both armbands.
   */
  it("does not restore the displaced captain from the backend's pick", () => {
    const saved = buildUpdatedMatchRolesForAssignment(xi, undefined, "vice_captain", "a");

    const next = resolveEffectiveMatchRoles(xi, saved, backend);

    expect(next.vice_captain).toBe("a");
    expect(next.captain).not.toBe("a");
  });
});

describe("buildUpdatedMatchRolesForAssignment", () => {
  const stored: TeamMatchRolesData = {
    captain: "a",
    vice_captain: "b",
    penalty_taker: null,
    free_kick_taker: null,
    corner_taker: null,
  };

  /**
   * Given a vice-captain
   * When he is made captain
   * Then the vice slot is cleared: nobody holds both.
   */
  it("clears the vice-captain when he becomes captain", () => {
    const next = buildUpdatedMatchRolesForAssignment(xi, stored, "captain", "b");

    expect(next.captain).toBe("b");
    expect(next.vice_captain).toBeNull();
  });

  /**
   * Given the captain
   * When he is made vice-captain
   * Then the captain slot is left empty, not filled from the backend's pick: the backend keeps
   *      choosing the captain at match time.
   */
  it("leaves the captaincy empty when the captain becomes vice", () => {
    const next = buildUpdatedMatchRolesForAssignment(
      xi,
      { ...stored, vice_captain: null },
      "vice_captain",
      "a",
    );

    expect(next.vice_captain).toBe("a");
    expect(next.captain).toBeNull();
  });

  /**
   * Given only some roles the manager has assigned
   * When another is assigned
   * Then the payload holds the manager's roles plus that one, and every unassigned role stays null
   *      for the backend to choose; nothing the screen shows as a fallback is saved.
   */
  it("saves only the manager's roles plus the assignment", () => {
    expect(buildUpdatedMatchRolesForAssignment(xi, stored, "corner_taker", "c")).toEqual({
      ...stored,
      corner_taker: "c",
    });
  });

  /**
   * Given a stored role whose player has left the eleven
   * When another role is assigned
   * Then the departed player's role is saved as unassigned.
   */
  it("drops a stored role whose player is no longer in the eleven", () => {
    const next = buildUpdatedMatchRolesForAssignment(
      xi,
      { ...stored, penalty_taker: "gone" },
      "corner_taker",
      "c",
    );

    expect(next.penalty_taker).toBeNull();
  });
});
