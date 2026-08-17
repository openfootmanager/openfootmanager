import { describe, expect, it } from "vitest";
import { FORMATIONS, MINUTES_PER_TICK, SPEED_MS } from "./types";
import { FORMATIONS as TACTICS_FORMATIONS } from "../tactics/TacticsTab.helpers";

describe("FORMATIONS", () => {
  it("includes 5-3-2 and 4-1-4-1 in the match types list", () => {
    expect(FORMATIONS).toContain("5-3-2");
    expect(FORMATIONS).toContain("4-1-4-1");
  });

  it("tactics and match FORMATIONS are identical", () => {
    expect(TACTICS_FORMATIONS).toEqual(FORMATIONS);
  });

  it("all formations are valid strings of the form N-N-N or N-N-N-N", () => {
    for (const f of FORMATIONS) {
      expect(f).toMatch(/^\d+-\d+-\d+(-\d+)?$/);
    }
  });
});

describe("match tick pacing", () => {
  it("only instant batches minutes; every watchable speed steps one at a time", () => {
    // Batching is what makes instant cheap, but it also means the UI sees one snapshot for
    // several minutes. At any speed a player is actually watching, that would visibly skip.
    expect(MINUTES_PER_TICK.instant).toBeGreaterThan(1);
    for (const speed of ["slow", "normal", "fast"] as const) {
      expect(MINUTES_PER_TICK[speed]).toBe(1);
    }
  });

  it("paused never steps", () => {
    expect(MINUTES_PER_TICK.paused).toBe(0);
    expect(SPEED_MS.paused).toBe(0);
  });

  it("every speed has an interval long enough to complete a tick", () => {
    // A tick is two IPC round trips plus a full React render. The old instant value of 10ms
    // could not finish one, so ticks queued back to back and starved the compositor of idle
    // frames — the app looked frozen while claiming to be fastest.
    for (const speed of ["slow", "normal", "fast", "instant"] as const) {
      expect(SPEED_MS[speed]).toBeGreaterThanOrEqual(100);
    }
  });

  it("faster speeds simulate more minutes per second than slower ones", () => {
    const rate = (speed: keyof typeof SPEED_MS) =>
      MINUTES_PER_TICK[speed] / SPEED_MS[speed];

    expect(rate("normal")).toBeGreaterThan(rate("slow"));
    expect(rate("fast")).toBeGreaterThan(rate("normal"));
    expect(rate("instant")).toBeGreaterThan(rate("fast"));
  });
});
