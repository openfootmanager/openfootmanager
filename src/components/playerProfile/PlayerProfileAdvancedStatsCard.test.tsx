import { render, screen } from "@testing-library/react";
import { describe, expect, it } from "vitest";

import type { PlayerAdvancedStatsSummary } from "./PlayerProfile.helpers";
import PlayerProfileAdvancedStatsCard from "./PlayerProfileAdvancedStatsCard";

const METRICS: PlayerAdvancedStatsSummary["metrics"] = {
  shots: { total: 20, per90: 4, percentile: 67 },
  shotsOnTarget: { total: 10, per90: 2, percentile: null },
  passes: { completed: 80, attempted: 100, accuracy: 80, percentile: null },
  tacklesWon: { total: 9, per90: 1.8, percentile: null },
  interceptions: { total: 6, per90: 1.2, percentile: null },
  foulsCommitted: { total: 5, per90: 1, percentile: 100 },
};

function summary(peerGroup: PlayerAdvancedStatsSummary["peerGroup"]): PlayerAdvancedStatsSummary {
  return { percentileEligible: true, peerGroup, seasonRatingRated: false, metrics: METRICS };
}

const t = (key: string, options?: Record<string, unknown>) =>
  options ? `${key}|${Object.values(options).join("|")}` : key;

describe("PlayerProfileAdvancedStatsCard peer group", () => {
  /**
   * Given the backend names a striker's group as the second division
   * When the card renders
   * Then the screen says which group the percentiles compare with
   */
  it("states the league and position the percentiles are measured within", () => {
    render(
      <PlayerProfileAdvancedStatsCard
        summary={summary({
          competitionId: "div-2",
          competitionName: "Second Division",
          competitionNameKey: null,
          countryId: null,
          position: "Striker",
        })}
        t={t}
      />,
    );

    expect(
      screen.getByText("playerProfile.percentileGroup|common.positions.Striker|Second Division"),
    ).toBeInTheDocument();
  });

  /**
   * Given the backend gives no peer group (a player with no league)
   * When the card renders
   * Then no group line is shown
   */
  it("shows no group line without a peer group", () => {
    render(<PlayerProfileAdvancedStatsCard summary={summary(null)} t={t} />);

    expect(screen.queryByText(/percentileGroup/)).not.toBeInTheDocument();
  });

  /**
   * Given the backend ranks fouls with fewer as better and gives a 100th percentile
   * When the card renders
   * Then the percentile is shown as given, not recomputed
   */
  it("shows the backend's percentile without re-deriving the direction", () => {
    render(<PlayerProfileAdvancedStatsCard summary={summary(null)} t={t} />);

    expect(screen.getByText("100th")).toBeInTheDocument();
    expect(screen.getByText("67th")).toBeInTheDocument();
  });
});
