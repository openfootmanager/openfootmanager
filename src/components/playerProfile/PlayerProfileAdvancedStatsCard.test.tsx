import { render, screen } from "@testing-library/react";
import { afterEach, beforeAll, describe, expect, it } from "vitest";

import { changeAppLanguage, i18nReady } from "../../i18n";
import type { PlayerAdvancedStatsSummary } from "./PlayerProfile.helpers";
import PlayerProfileAdvancedStatsCard from "./PlayerProfileAdvancedStatsCard";

const SUMMARY: PlayerAdvancedStatsSummary = {
  percentileEligible: true,
  seasonRatingRated: false,
  metrics: {
    shots: { total: 20, per90: 4, percentile: 2 },
    shotsOnTarget: { total: 10, per90: 2, percentile: null },
    passes: { completed: 80, attempted: 100, accuracy: 80, percentile: null },
    tacklesWon: { total: 9, per90: 1.8, percentile: null },
    interceptions: { total: 6, per90: 1.2, percentile: null },
    foulsCommitted: { total: 5, per90: 1, percentile: null },
  },
};

describe("PlayerProfileAdvancedStatsCard percentiles", () => {
  beforeAll(async () => {
    await i18nReady;
  });

  afterEach(async () => {
    await changeAppLanguage("en");
  });

  /**
   * Given the game language is Portuguese and a shots percentile of 2
   * When the card renders
   * Then it reads "2.º", not "2nd"
   */
  it("writes the percentile as an ordinal in the game language", async () => {
    await changeAppLanguage("pt");

    render(<PlayerProfileAdvancedStatsCard summary={SUMMARY} t={(key) => key} />);

    expect(screen.getByText("2.º")).toBeInTheDocument();
    expect(screen.queryByText("2nd")).not.toBeInTheDocument();
  });

  /**
   * Given the game language is English
   * When the same card renders
   * Then it reads "2nd"
   */
  it("keeps English suffixes in English", async () => {
    await changeAppLanguage("en");

    render(<PlayerProfileAdvancedStatsCard summary={SUMMARY} t={(key) => key} />);

    expect(screen.getByText("2nd")).toBeInTheDocument();
  });
});
