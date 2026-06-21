import { describe, expect, it } from "vitest";
import { createPlayer } from "../../test-utils/factories";
import {
  EMPTY_MATCH_ROLES,
  resolveEffectiveMatchRoles,
  buildUpdatedMatchRolesForAssignment,
  pickBestCandidate,
} from "./TacticsRoles.helpers";

// Captain score = (leadership + teamwork) / 2
const strongLeader = createPlayer({
  id: "leader",
  full_name: "Abel Leader",
  attributes: { ...createPlayer().attributes, leadership: 90, teamwork: 90 },
});
const weakLeader = createPlayer({
  id: "follower",
  full_name: "Bob Follower",
  attributes: { ...createPlayer().attributes, leadership: 50, teamwork: 50 },
});

describe("pickBestCandidate", () => {
  it("returns the player with the highest score for the role", () => {
    expect(pickBestCandidate([weakLeader, strongLeader], "captain")).toBe("leader");
  });

  it("excludes ids in the exclusion list", () => {
    expect(pickBestCandidate([weakLeader, strongLeader], "captain", ["leader"])).toBe("follower");
  });

  it("returns null when all candidates are excluded", () => {
    expect(pickBestCandidate([strongLeader], "captain", ["leader"])).toBeNull();
  });

  it("allows goalkeepers for captain role", () => {
    const gk = createPlayer({ id: "gk", position: "Goalkeeper" });
    expect(pickBestCandidate([gk], "captain")).toBe("gk");
  });

  it("excludes goalkeepers from non-captain roles", () => {
    const gk = createPlayer({ id: "gk", position: "Goalkeeper" });
    const outfielder = createPlayer({ id: "out", position: "Forward" });
    expect(pickBestCandidate([gk, outfielder], "penalty")).toBe("out");
  });
});

describe("resolveEffectiveMatchRoles", () => {
  it("picks best captain from starting lineup when no roles are stored", () => {
    const roles = resolveEffectiveMatchRoles([weakLeader, strongLeader]);
    expect(roles.captain).toBe("leader");
  });

  it("keeps stored captain when they are still in the lineup", () => {
    const stored = { ...EMPTY_MATCH_ROLES, captain: "follower" };
    const roles = resolveEffectiveMatchRoles([weakLeader, strongLeader], stored);
    expect(roles.captain).toBe("follower");
  });

  it("falls back to best candidate when stored captain is not in lineup", () => {
    const stored = { ...EMPTY_MATCH_ROLES, captain: "absent-player" };
    const roles = resolveEffectiveMatchRoles([weakLeader, strongLeader], stored);
    expect(roles.captain).toBe("leader");
  });

  it("captain and vice_captain are different players", () => {
    const roles = resolveEffectiveMatchRoles([weakLeader, strongLeader]);
    expect(roles.captain).not.toBeNull();
    expect(roles.vice_captain).not.toBeNull();
    expect(roles.captain).not.toBe(roles.vice_captain);
  });

  it("returns null roles when starting lineup is empty", () => {
    const roles = resolveEffectiveMatchRoles([]);
    expect(roles.captain).toBeNull();
    expect(roles.vice_captain).toBeNull();
  });
});

describe("buildUpdatedMatchRolesForAssignment", () => {
  const players = [weakLeader, strongLeader];
  const baseRoles = { ...EMPTY_MATCH_ROLES, captain: "leader", vice_captain: "follower" };

  it("updates the specified role", () => {
    const updated = buildUpdatedMatchRolesForAssignment(baseRoles, players, "penalty_taker", "leader");
    expect(updated.penalty_taker).toBe("leader");
  });

  it("reassigns vice_captain when new captain was the vice_captain", () => {
    // Assigning vice_captain (follower) as captain should free the vice_captain slot
    const updated = buildUpdatedMatchRolesForAssignment(baseRoles, players, "captain", "follower");
    expect(updated.captain).toBe("follower");
    expect(updated.vice_captain).not.toBe("follower");
  });

  it("reassigns captain when new vice_captain was the captain", () => {
    const updated = buildUpdatedMatchRolesForAssignment(baseRoles, players, "vice_captain", "leader");
    expect(updated.vice_captain).toBe("leader");
    expect(updated.captain).not.toBe("leader");
  });
});
