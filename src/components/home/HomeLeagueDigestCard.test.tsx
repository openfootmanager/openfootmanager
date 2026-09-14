import { fireEvent, render, screen } from "@testing-library/react";
import { describe, expect, it, vi } from "vitest";

import type { NewsArticle } from "../../store/gameStore";
import HomeLeagueDigestCard from "./HomeLeagueDigestCard";

vi.mock("react-i18next", () => ({
  useTranslation: () => ({
    t: (key: string) => {
      if (key === "dashboard.news") return "News";
      if (key === "home.leagueDigest") return "League Digest";
      if (key === "home.noLeagueDigest") return "No league digest yet.";
      if (key === "news.categories.StandingsUpdate") return "Standings";
      return key;
    },
  }),
}));

function createArticle(overrides: Partial<NewsArticle> = {}): NewsArticle {
  return {
    id: "news-1",
    headline: "Standings headline",
    body: "Body",
    source: "OpenFoot Times",
    date: "2025-01-15",
    category: "StandingsUpdate",
    team_ids: [],
    player_ids: [],
    match_score: null,
    read: false,
    ...overrides,
  };
}

describe("HomeLeagueDigestCard", () => {
  it("renders digest articles and delegates navigation", () => {
    const onNavigate = vi.fn();

    render(<HomeLeagueDigestCard articles={[createArticle()]} lang="en" onNavigate={onNavigate} />);

    expect(screen.getByText("League Digest")).toBeInTheDocument();
    expect(screen.getByText("Standings headline")).toBeInTheDocument();

    fireEvent.click(screen.getByRole("button", { name: /Standings headline/i }));

    expect(onNavigate).toHaveBeenCalledWith("News");
  });

  it("renders the empty state when there are no digest articles", () => {
    render(<HomeLeagueDigestCard articles={[]} lang="en" />);

    expect(screen.getByText("No league digest yet.")).toBeInTheDocument();
  });
});
