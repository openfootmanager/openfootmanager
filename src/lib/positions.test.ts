import { describe, expect, it } from "vitest";

import { canonicalPosition, naturalPositionOf } from "./positions";

// Every value `domain::player::Position` can serialise to: each is already its own canonical form.
// The group and rank of each are the backend's (`Position::to_group_position` / `sort_rank`).
const VARIANTS = [
  "Goalkeeper",
  "Defender",
  "CenterBack",
  "LeftBack",
  "RightBack",
  "LeftWingBack",
  "RightWingBack",
  "Midfielder",
  "DefensiveMidfielder",
  "CentralMidfielder",
  "AttackingMidfielder",
  "LeftMidfielder",
  "RightMidfielder",
  "Forward",
  "LeftWinger",
  "RightWinger",
  "Striker",
];

// Short and spelled-out forms that reach the frontend from packages, the CLI and older saves.
// Pinned one by one: deleting an alias silently drops that player to the bottom of every roster.
const ALIASES: [alias: string, canonical: string][] = [
  ["gk", "Goalkeeper"],
  ["def", "Defender"],
  ["wingback", "Defender"],
  ["mid", "Midfielder"],
  ["fwd", "Forward"],
  ["winger", "Forward"],
  ["rb", "RightBack"],
  ["cb", "CenterBack"],
  ["centreback", "CenterBack"],
  ["lb", "LeftBack"],
  ["rwb", "RightWingBack"],
  ["lwb", "LeftWingBack"],
  ["dm", "DefensiveMidfielder"],
  ["cm", "CentralMidfielder"],
  ["am", "AttackingMidfielder"],
  ["rm", "RightMidfielder"],
  ["lm", "LeftMidfielder"],
  ["rw", "RightWinger"],
  ["lw", "LeftWinger"],
  ["st", "Striker"],
];

describe("positions", () => {
  it.each(VARIANTS)("keeps %s as it is", (position) => {
    expect(canonicalPosition(position)).toBe(position);
  });

  it.each(ALIASES)("reads the alias %s as %s", (alias, canonical) => {
    expect(canonicalPosition(alias)).toBe(canonical);
  });

  it("ignores case, spacing and punctuation", () => {
    expect(canonicalPosition(" Centre Back ")).toBe("CenterBack");
    expect(canonicalPosition("LEFT-WING-BACK")).toBe("LeftWingBack");
    expect(canonicalPosition("St")).toBe("Striker");
  });

  it("leaves anything it does not recognise as it is", () => {
    for (const unknown of ["Sweeper", "not a position"]) {
      expect(canonicalPosition(unknown)).toBe(unknown);
    }
    expect(canonicalPosition("   ")).toBe("");
  });
});

describe("naturalPositionOf", () => {
  /**
   * Given a player whose natural position is granular and whose `position` is the legacy bucket
   * When the position to show is asked for
   * Then the granular natural position wins, so a new game reads like a reloaded one.
   */
  it("prefers the granular natural position over the legacy bucket", () => {
    expect(naturalPositionOf({ position: "Defender", natural_position: "LeftBack" })).toBe(
      "LeftBack",
    );
  });

  /**
   * Given a player from an old save whose natural position is empty
   * When the position to show is asked for
   * Then it falls back to `position`.
   */
  it("falls back to position when the natural position is empty", () => {
    expect(naturalPositionOf({ position: "Midfielder", natural_position: "" })).toBe("Midfielder");
  });
});
