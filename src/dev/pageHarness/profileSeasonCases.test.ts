import { describe, expect, it } from "vitest";

import { HARNESS_PROFILES } from "./harnessPage";
import { profileCase } from "./profileCases";

const METRICS = [
  "shots",
  "shotsOnTarget",
  "passes",
  "tacklesWon",
  "interceptions",
  "foulsCommitted",
] as const;

// The season stats the harness answers follow `get_player_stats_overview`: percentiles and a peer
// group when the backend can compute them (the player has minutes at a club in a league table), and
// none when it cannot (no club, no league). A page harness that shows a lone dash where the bar and
// the ordinal go shows nothing of the finished card.
describe("the harness's season stats answer the percentiles the backend does", () => {
  /**
   * Given the club's own player and a division rival, who both have a season behind them
   * When their season stats are read
   * Then each is eligible, with a peer group (his league and natural position) and a percentile in
   *      1 to 99 for every one of the six stats, no two alike
   */
  it.each(["own", "rival"] as const)(
    "gives the %s player percentiles and a peer group",
    (variant) => {
      const { seasonStats } = profileCase(variant);

      expect(seasonStats.percentileEligible).toBe(true);
      expect(seasonStats.peerGroup).toMatchObject({
        competitionName: expect.any(String),
        position: expect.any(String),
      });
      expect(seasonStats.peerGroup?.competitionName).not.toBe("");
      const percentiles = METRICS.map((key) => seasonStats.metrics[key].percentile);
      for (const percentile of percentiles) {
        expect(percentile).not.toBeNull();
        expect(percentile as number).toBeGreaterThanOrEqual(1);
        expect(percentile as number).toBeLessThanOrEqual(99);
      }
      expect(new Set(percentiles).size).toBe(METRICS.length);
    },
  );

  /**
   * Given a free agent (no club, so no league table and no peer group)
   * When his season stats are read
   * Then he is not eligible, has no peer group and no percentile on any stat
   */
  it("keeps the free agent ineligible, with no peer group and no percentile", () => {
    const { seasonStats } = profileCase("freeAgent");

    expect(seasonStats.percentileEligible).toBe(false);
    expect(seasonStats.peerGroup).toBeNull();
    expect(METRICS.every((key) => seasonStats.metrics[key].percentile === null)).toBe(true);
  });

  /**
   * Given every state the harness opens a profile on
   * When its season stats are read
   * Then a percentile only ever comes with a peer group: none is eligible without one
   */
  it.each(HARNESS_PROFILES)("never has a percentile without a peer group: %s", (variant) => {
    const { seasonStats } = profileCase(variant);
    const anyPercentile = METRICS.some((key) => seasonStats.metrics[key].percentile !== null);

    expect(anyPercentile).toBe(seasonStats.peerGroup !== null);
    expect(seasonStats.percentileEligible).toBe(seasonStats.peerGroup !== null);
  });
});
