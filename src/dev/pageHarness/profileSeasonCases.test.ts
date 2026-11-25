import { describe, expect, it } from "vitest";

import { naturalPositionOf } from "../../lib/positions";
import { gameState } from "./fixture.generated";
import type { HarnessProfile } from "./harnessPage";
import { profileCase } from "./profileCases";

const METRICS = [
  "shots",
  "shotsOnTarget",
  "passes",
  "tacklesWon",
  "interceptions",
  "foulsCommitted",
] as const;

// Who the backend ranks: a player with a season at a club in a league table. The club's own
// players and a division rival have one; a free agent has no club, and the scouted, unknown and
// retired cases show no season at all.
const ELIGIBLE: HarnessProfile[] = ["own", "keeper", "injured", "loaned", "rival"];
const INELIGIBLE: HarnessProfile[] = ["scouted", "unknown", "freeAgent", "retired"];

// The season stats the harness answers follow `get_player_stats_overview`: percentiles and a peer
// group when the backend can compute them, and none when it cannot. A page harness that shows a
// lone dash where the bar and the ordinal go shows nothing of the finished card.
describe("the harness's season stats answer the percentiles the backend does", () => {
  /**
   * Given every state the harness opens a profile on
   * When its season stats are read
   * Then the club's own players and the division rival are eligible, and the free agent, scouted,
   *      unknown and retired cases are not, with no peer group and no percentile
   */
  it.each([
    ...ELIGIBLE.map((variant) => [variant, true] as const),
    ...INELIGIBLE.map((variant) => [variant, false] as const),
  ])("%s: eligible is %s", (variant, eligible) => {
    const { seasonStats } = profileCase(variant);

    expect(seasonStats.percentileEligible).toBe(eligible);
    expect(seasonStats.peerGroup !== null).toBe(eligible);
    expect(METRICS.some((key) => seasonStats.metrics[key].percentile !== null)).toBe(eligible);
  });

  /**
   * Given each eligible case
   * When its season stats are read
   * Then every one of the six stats has a percentile in 1 to 99, no two alike
   */
  it.each(ELIGIBLE)("gives the %s player a distinct percentile on every stat", (variant) => {
    const { seasonStats } = profileCase(variant);
    const percentiles = METRICS.map((key) => seasonStats.metrics[key].percentile);

    for (const percentile of percentiles) {
      expect(percentile).not.toBeNull();
      expect(percentile as number).toBeGreaterThanOrEqual(1);
      expect(percentile as number).toBeLessThanOrEqual(99);
    }
    expect(new Set(percentiles).size).toBe(METRICS.length);
  });

  /**
   * Given each eligible case, and the fixture's league and the player the case is open on
   * When its peer group is read
   * Then it is that league (id, name, key, country) and the player's own natural position, so the
   *      goalkeeper is ranked among goalkeepers and not among the club's outfield players
   */
  it.each(ELIGIBLE)(
    "ranks the %s player in the fixture's league at his natural position",
    (variant) => {
      const { playerId, seasonStats } = profileCase(variant);
      const player = gameState.players.find((candidate) => candidate.id === playerId);
      const league = gameState.competitions.find((competition) => competition.kind === "League");

      expect(player).toBeDefined();
      expect(league).toBeDefined();
      expect(seasonStats.peerGroup).toMatchObject({
        competitionId: league?.id,
        competitionName: league?.name,
        competitionNameKey: league?.name_key,
        countryId: league?.country_id,
        position: player ? naturalPositionOf(player) : "",
      });
    },
  );

  /**
   * Given the club's goalkeeper and an outfield player of the club
   * When their peer groups are read
   * Then the two differ in position, which is what the assertion above would not catch if every
   *      case answered the same one
   */
  it("ranks the keeper among goalkeepers and the own outfield player among others", () => {
    const keeper = profileCase("keeper").seasonStats.peerGroup?.position;
    const outfield = profileCase("own").seasonStats.peerGroup?.position;

    expect(keeper).toBe("Goalkeeper");
    expect(outfield).not.toBe("Goalkeeper");
  });
});
