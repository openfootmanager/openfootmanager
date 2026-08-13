import { describe, expect, it } from "vitest";
import { inferFrameBudget, quantile, summarise } from "./benchUi";

// These three carry the whole numeric story of the benchmark: every figure in
// docs/LINUX_GRAPHICS.md comes out of them. The phases themselves need a real renderer and are
// exercised by running the harness; these are the parts that can be wrong quietly.

describe("quantile", () => {
  it("uses nearest rank, so a quantile never reads past its own band", () => {
    const sorted = [1, 2, 3, 4, 5, 6, 7, 8, 9, 10];
    expect(quantile(sorted, 0.5)).toBe(5);
    // p10 of ten samples is the fastest one, not the second.
    expect(quantile(sorted, 0.1)).toBe(1);
  });

  it("never returns a negative index for q=0", () => {
    expect(quantile([5, 6, 7], 0)).toBe(5);
  });

  it("clamps at the top rather than reading past the end", () => {
    // Math.floor(1.0 * len) is len, which would be undefined without the clamp.
    expect(quantile([1, 2, 3], 1)).toBe(3);
  });

  it("returns 0 for an empty sample instead of NaN", () => {
    expect(quantile([], 0.5)).toBe(0);
  });

  it("rounds to two decimals", () => {
    expect(quantile([1.23456], 0.5)).toBe(1.23);
  });
});

describe("inferFrameBudget", () => {
  it("reads the display cadence from the fastest frames, not the average", () => {
    // A run that is mostly slow still reveals the refresh interval in the frames it did deliver
    // on time. Taking the median here would report ~100ms on a 60Hz display.
    const deltas = [...Array(10).fill(16.7), ...Array(90).fill(100)];
    expect(inferFrameBudget(deltas)).toBeCloseTo(16.7, 1);
  });

  it("floors at 4ms so coalesced callbacks cannot imply an absurd refresh rate", () => {
    expect(inferFrameBudget([0, 0, 0, 0, 1])).toBe(4);
  });

  it("falls back to 60Hz when there is nothing to infer from", () => {
    expect(inferFrameBudget([])).toBeCloseTo(16.67, 2);
  });
});

describe("summarise", () => {
  const budget = 16;

  it("drops the first delta, which straddles setup rather than the interaction", () => {
    const result = summarise("scroll", [999, 16, 16, 16], budget);
    expect(result.frames).toBe(3);
    expect(result.max).toBe(16);
  });

  it("counts frames over 1.5x the budget as dropped", () => {
    // 24ms is exactly the threshold and must not count; 25ms must.
    const result = summarise("composite", [0, 16, 24, 25, 200], budget);
    expect(result.dropped).toBe(2);
  });

  it("reports the collapse the investigation turned on", () => {
    // The shipped-vs-fixed contrast: ~224ms per composited frame against ~16ms.
    const slow = summarise("composite", [0, ...Array(20).fill(224)], budget);
    expect(slow.p50).toBe(224);
    expect(slow.dropped).toBe(20);
    expect(slow.fps).toBeLessThan(6);
  });

  it("produces zeros rather than NaN or -Infinity for an empty or single-sample phase", () => {
    for (const deltas of [[], [16]]) {
      const result = summarise("empty", deltas, budget);
      expect(result.frames).toBe(0);
      expect(result.fps).toBe(0);
      expect(result.max).toBe(0);
      expect(Number.isNaN(result.p50)).toBe(false);
    }
  });

  it("keeps the phase name it was given", () => {
    expect(summarise("repaint", [0, 16], budget).name).toBe("repaint");
  });
});
