import { describe, expect, it } from "vitest";

import {
  canonicalPosition,
  normalisePosition,
  positionGroupRank,
  positionSortRank,
} from "./positions";

// Every value `domain::player::Position` can serialise to, with the rank each ranker must give it.
// These two functions order the team profile, squad and tactics rosters, so a change here moves
// players in all three.
const VARIANTS: [position: string, group: string, groupRank: number, sortRank: number][] = [
  ["Goalkeeper", "Goalkeeper", 10, 10],
  ["Defender", "Defender", 20, 20],
  ["CenterBack", "Defender", 20, 21],
  ["LeftBack", "Defender", 20, 22],
  ["RightBack", "Defender", 20, 23],
  ["LeftWingBack", "Defender", 20, 24],
  ["RightWingBack", "Defender", 20, 25],
  ["Midfielder", "Midfielder", 30, 30],
  ["DefensiveMidfielder", "Midfielder", 30, 31],
  ["CentralMidfielder", "Midfielder", 30, 32],
  ["AttackingMidfielder", "Midfielder", 30, 33],
  ["LeftMidfielder", "Midfielder", 30, 34],
  ["RightMidfielder", "Midfielder", 30, 35],
  ["Forward", "Forward", 40, 40],
  ["LeftWinger", "Forward", 40, 41],
  ["RightWinger", "Forward", 40, 42],
  ["Striker", "Forward", 40, 43],
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
  it.each(VARIANTS)("ranks %s", (position, group, groupRank, sortRank) => {
    expect(canonicalPosition(position)).toBe(position);
    expect(normalisePosition(position)).toBe(group);
    expect(positionGroupRank(position)).toBe(groupRank);
    expect(positionSortRank(position)).toBe(sortRank);
  });

  it.each(ALIASES)("reads the alias %s as %s", (alias, canonical) => {
    expect(canonicalPosition(alias)).toBe(canonical);
    expect(positionSortRank(alias)).toBe(positionSortRank(canonical));
    expect(positionGroupRank(alias)).toBe(positionGroupRank(canonical));
  });

  it("ignores case, spacing and punctuation", () => {
    expect(canonicalPosition(" Centre Back ")).toBe("CenterBack");
    expect(canonicalPosition("LEFT-WING-BACK")).toBe("LeftWingBack");
    expect(canonicalPosition("St")).toBe("Striker");
  });

  it("sorts anything it does not recognise after every real position", () => {
    for (const unknown of ["", "   ", "Sweeper", "not a position"]) {
      expect(positionGroupRank(unknown)).toBeGreaterThan(positionGroupRank("Striker"));
      expect(positionSortRank(unknown)).toBeGreaterThan(positionSortRank("Striker"));
    }
  });

  it("keeps a legacy bucket at the head of its own line", () => {
    // Unmigrated players still carry the four bucket names. Ranking each bucket just before its
    // granular positions keeps them beside their teammates instead of scattering them.
    expect(positionSortRank("Defender")).toBeLessThan(positionSortRank("CenterBack"));
    expect(positionSortRank("Defender")).toBeGreaterThan(positionSortRank("Goalkeeper"));
    expect(positionSortRank("Midfielder")).toBeGreaterThan(positionSortRank("RightWingBack"));
    expect(positionSortRank("Forward")).toBeGreaterThan(positionSortRank("RightMidfielder"));
  });
});
