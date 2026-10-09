import { render, screen } from "@testing-library/react";
import { describe, expect, it } from "vitest";
import type { PlayerSeasonStats } from "../../store/gameStore";
import PlayerProfileSeasonStatsCard from "./PlayerProfileSeasonStatsCard";

const stats = {
  appearances: 3,
  goals: 0,
  assists: 0,
  minutes_played: 270,
  clean_sheets: 0,
  yellow_cards: 0,
  red_cards: 0,
  avg_rating: 0,
} as PlayerSeasonStats;

const t = (key: string) => key;

describe("PlayerProfileSeasonStatsCard average rating", () => {
  /**
   * Given the backend flags the season rating as unavailable
   * When the card renders
   * Then the average rating is a dash, not 0.0
   */
  it("shows a dash when no match is rated", () => {
    render(<PlayerProfileSeasonStatsCard stats={stats} seasonRatingRated={false} t={t} />);

    expect(screen.getByText("–")).toBeInTheDocument();
    expect(screen.queryByText("0.0")).not.toBeInTheDocument();
  });

  /**
   * Given the backend flags the season rating as available
   * When the card renders
   * Then the stored average shows
   */
  it("shows the average once matches are rated", () => {
    render(
      <PlayerProfileSeasonStatsCard
        stats={{ ...stats, avg_rating: 7.24 }}
        seasonRatingRated
        t={t}
      />,
    );

    expect(screen.getByText("7.2")).toBeInTheDocument();
  });
});
