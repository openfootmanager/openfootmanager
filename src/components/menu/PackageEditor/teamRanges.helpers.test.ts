import { describe, expect, it } from "vitest";

import {
  REPUTATION_MAX,
  REPUTATION_MIN,
  TRANSFER_BUDGET_SHARE,
  deriveTransferBudget,
  formatCompactAmount,
  reputationBandKey,
} from "./teamRanges.helpers";

describe("reputationBandKey", () => {
  it("reads on the 0-1000 scale, not 0-100", () => {
    // The trap this exists to close: 92 looks like "elite" on a 0-100 scale
    // but is actually below amateur. A whole authored package was written
    // that way before anything in the editor said otherwise.
    expect(reputationBandKey(92)).toBe("amateur");
    expect(reputationBandKey(920)).toBe("elite");
  });

  it("labels each documented band", () => {
    expect(reputationBandKey(0)).toBe("amateur");
    expect(reputationBandKey(299)).toBe("amateur");
    expect(reputationBandKey(300)).toBe("lowerLeague");
    expect(reputationBandKey(549)).toBe("lowerLeague");
    expect(reputationBandKey(550)).toBe("midTable");
    expect(reputationBandKey(719)).toBe("midTable");
    expect(reputationBandKey(720)).toBe("topFlight");
    expect(reputationBandKey(849)).toBe("topFlight");
    expect(reputationBandKey(850)).toBe("elite");
    expect(reputationBandKey(REPUTATION_MAX)).toBe("elite");
  });

  it("clamps out-of-range input rather than returning nothing", () => {
    expect(reputationBandKey(-50)).toBe("amateur");
    expect(reputationBandKey(5000)).toBe("elite");
    expect(REPUTATION_MIN).toBe(0);
    expect(REPUTATION_MAX).toBe(1000);
  });
});

describe("deriveTransferBudget", () => {
  it("is 15% of the club's finance, matching world generation", () => {
    // generator/mod.rs: team.transfer_budget = finance * 0.15
    expect(TRANSFER_BUDGET_SHARE).toBe(0.15);
    expect(deriveTransferBudget(4_000_000)).toBe(600_000);
    expect(deriveTransferBudget(0)).toBe(0);
  });

  it("floors to a whole unit like the backend cast does", () => {
    expect(deriveTransferBudget(1)).toBe(0);
    expect(deriveTransferBudget(11)).toBe(1);
  });

  it("ignores a negative finance value", () => {
    expect(deriveTransferBudget(-100)).toBe(0);
  });
});

describe("formatCompactAmount", () => {
  it("shortens large amounts so a range stays readable", () => {
    expect(formatCompactAmount(4_000_000)).toBe("4M");
    expect(formatCompactAmount(3_500_000)).toBe("3.5M");
    expect(formatCompactAmount(450_000)).toBe("450K");
    expect(formatCompactAmount(1_200)).toBe("1.2K");
    expect(formatCompactAmount(750)).toBe("750");
  });

  it("rounds before deciding whether a fraction is needed", () => {
    // 1,999,999 must not read "2.0M" — rounding has to happen first.
    expect(formatCompactAmount(1_999_999)).toBe("2M");
  });

  it("handles zero and negatives without producing junk", () => {
    expect(formatCompactAmount(0)).toBe("0");
    expect(formatCompactAmount(-2_000_000)).toBe("-2M");
  });

  it("uses each locale's own magnitude conventions", () => {
    // Hardcoded K/M suffixes would ship English units to every language.
    expect(formatCompactAmount(4_000_000, "zh-CN")).toBe("400万");
    expect(formatCompactAmount(450_000, "zh-CN")).toBe("45万");
    // Intl separates the unit with a non-breaking space in German.
    expect(formatCompactAmount(4_000_000, "de")).toBe("4\u00a0Mio.");
  });
});
