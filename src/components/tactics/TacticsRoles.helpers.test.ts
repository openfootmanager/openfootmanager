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

describe("buildUpdatedMatchRolesForAssignment", () => {
  const effective: TeamMatchRolesData = {
    captain: "a",
    vice_captain: "b",
    penalty_taker: "b",
    free_kick_taker: "c",
    corner_taker: "a",
  };

  /**
   * Given a vice-captain
   * When he is made captain
   * Then the vice slot is cleared: nobody holds both.
   */
  it("clears the vice-captain when he becomes captain", () => {
    const next = buildUpdatedMatchRolesForAssignment(effective, "captain", "b", backend);

    expect(next.captain).toBe("b");
    expect(next.vice_captain).toBeNull();
  });

  /**
   * Given the captain
   * When he is made vice-captain
   * Then the captain slot falls back to the backend's pick, unless that is the same player.
   */
  it("gives the captaincy back to the backend's pick when the captain becomes vice", () => {
    const next = buildUpdatedMatchRolesForAssignment(
      { ...effective, vice_captain: null },
      "vice_captain",
      "a",
      { ...backend, captain: "c" },
    );
    expect(next.vice_captain).toBe("a");
    expect(next.captain).toBe("c");

    const same = buildUpdatedMatchRolesForAssignment(
      { ...effective, vice_captain: null },
      "vice_captain",
      "a",
      backend,
    );
    expect(same.captain).toBeNull();
  });

  /**
   * Given any other role
   * When a player is assigned
   * Then only that role changes.
   */
  it("changes only the assigned role otherwise", () => {
    expect(buildUpdatedMatchRolesForAssignment(effective, "corner_taker", "c", backend)).toEqual({
      ...effective,
      corner_taker: "c",
    });
  });
});
