import { describe, it, expect } from "vitest";

import {
  ACADEMY_SHOWCASE_SAMPLE,
  NAME_PACK_SAMPLE,
  SAMPLE_PACKAGES,
} from "./sampleData";

describe("bundled World Editor samples", () => {
  it("every sample has a unique package id", () => {
    const ids = SAMPLE_PACKAGES.map((s) => s.meta.id);
    expect(new Set(ids).size).toBe(ids.length);
  });

  it("each sample's references resolve within the same package", () => {
    for (const sample of SAMPLE_PACKAGES) {
      const teamIds = new Set(sample.teams.map((t) => t.id));

      // Players and staff may be unattached ("") but never reference a missing club.
      for (const player of sample.players) {
        if (player.club) expect(teamIds.has(player.club)).toBe(true);
      }
      for (const staff of sample.staff ?? []) {
        if (staff.club) expect(teamIds.has(staff.club)).toBe(true);
      }

      // Explicit competition participants must all be defined teams.
      for (const comp of sample.competitions) {
        for (const id of comp.participants.explicit ?? []) {
          expect(teamIds.has(id)).toBe(true);
        }
      }
    }
  });

  it("the academy showcase exercises youth players and staff", () => {
    expect(ACADEMY_SHOWCASE_SAMPLE.players.some((p) => p.youth === true)).toBe(true);
    expect((ACADEMY_SHOWCASE_SAMPLE.staff ?? []).length).toBeGreaterThan(0);
    // It also ships a name pool so the Names section has content.
    expect(Object.keys(ACADEMY_SHOWCASE_SAMPLE.names.pools).length).toBeGreaterThan(0);
  });

  it("the name pack is a names-only patch", () => {
    expect(NAME_PACK_SAMPLE.meta.packageType).toBe("patch");
    expect(NAME_PACK_SAMPLE.teams).toHaveLength(0);
    expect(NAME_PACK_SAMPLE.competitions).toHaveLength(0);
    expect(Object.keys(NAME_PACK_SAMPLE.names.pools).length).toBeGreaterThan(0);
  });
});
