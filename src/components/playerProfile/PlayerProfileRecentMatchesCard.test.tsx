import { render, screen } from "@testing-library/react";
import { describe, expect, it, vi } from "vitest";
import PlayerProfileRecentMatchesCard, {
  type PlayerRecentMatchEntry,
} from "./PlayerProfileRecentMatchesCard";

vi.mock("./PlayerRatingTrendChart", () => ({
  PlayerRatingTrendChart: ({ matches }: { matches: PlayerRecentMatchEntry[] }) => (
    <div data-testid="trend" data-count={matches.length} />
  ),
}));

const t = (key: string, options?: { defaultValue?: string }) => options?.defaultValue ?? key;

function match(id: string, rating: number, rated: boolean): PlayerRecentMatchEntry {
  return {
    fixture_id: id,
    date: "2026-09-01",
    competition: "League",
    matchday: 1,
    opponent_team_id: "t2",
    opponent_name: `Opp ${id}`,
    team_goals: 1,
    opponent_goals: 0,
    minutes_played: 90,
    goals: 0,
    assists: 0,
    shots: 0,
    shots_on_target: 0,
    rating,
    rated,
  };
}

describe("PlayerProfileRecentMatchesCard ratings", () => {
  /**
   * Given matches whose stored rating is the unrated zero
   * When the card renders
   * Then each shows a dash, not 0.0, and there is no trend chart
   */
  it("shows a dash and no trend when no match has a rating", () => {
    render(
      <PlayerProfileRecentMatchesCard
        matches={[match("a", 0, false), match("b", 0, false)]}
        t={t}
      />,
    );

    expect(screen.queryByText("0.0")).not.toBeInTheDocument();
    expect(screen.getAllByText("–")).toHaveLength(2);
    expect(screen.queryByTestId("trend")).not.toBeInTheDocument();
  });

  /**
   * Given a mix of rated and unrated matches
   * When the card renders
   * Then rated ones show their rating and the trend plots only the rated ones
   */
  it("plots only the rated matches", () => {
    render(
      <PlayerProfileRecentMatchesCard
        matches={[match("a", 7.2, true), match("b", 0, false), match("c", 6.1, true)]}
        t={t}
      />,
    );

    expect(screen.getByText("7.2")).toBeInTheDocument();
    expect(screen.getAllByText("–")).toHaveLength(1);
    expect(screen.getByTestId("trend")).toHaveAttribute("data-count", "2");
  });

  /**
   * Given a single rated match among unrated ones
   * When the card renders
   * Then no trend block appears, since one point is not a trend
   */
  it("hides the trend until two matches are rated", () => {
    render(
      <PlayerProfileRecentMatchesCard
        matches={[match("a", 7.2, true), match("b", 0, false)]}
        t={t}
      />,
    );

    expect(screen.getByText("7.2")).toBeInTheDocument();
    expect(screen.queryByTestId("trend")).not.toBeInTheDocument();
  });
});
