import { describe, it, expect, vi } from "vitest";
import { render, screen } from "@testing-library/react";
import MatchPitch2D from "./MatchPitch2D";
import { makeSnapshot } from "../matchScreenFixtures";

vi.mock("react-i18next", () => ({
  useTranslation: () => ({
    t: (key: string, options?: { defaultValue?: string } | string) => {
      if (typeof options === "string") return options;
      return options?.defaultValue ?? key;
    },
    i18n: { language: "en" },
  }),
}));

describe("MatchPitch2D", () => {
  it("renders the 2D football pitch with player dots and ball", () => {
    const snapshot = makeSnapshot();
    const { container } = render(<MatchPitch2D snapshot={snapshot} />);

    // Renders the pitch container and SVG field
    expect(container.querySelector('[data-testid="match-pitch-2d"]')).toBeInTheDocument();
    expect(container.querySelector("svg")).toBeInTheDocument();

    // Renders player dots
    const dots = container.querySelectorAll('[data-testid^="player-dot-"]');
    expect(dots.length).toBeGreaterThan(0);

    // Renders the ball dot
    expect(container.querySelector('[data-testid="ball-dot"]')).toBeInTheDocument();
  });

  it("displays home and away team names in the scoreboard header", () => {
    const snapshot = makeSnapshot();
    render(<MatchPitch2D snapshot={snapshot} />);

    expect(screen.getByText("Alpha FC")).toBeInTheDocument();
    expect(screen.getByText("Beta FC")).toBeInTheDocument();
    expect(screen.getByText(/2\s*-\s*1/)).toBeInTheDocument();
  });

  it("shows goal celebration indicator when a goal event is active", () => {
    const snapshot = makeSnapshot();
    const goalEvents = [
      {
        minute: 88,
        event_type: "Goal",
        side: "Home" as const,
        zone: "AwayBox",
        player_id: "p1",
        secondary_player_id: null,
      },
    ];

    render(<MatchPitch2D snapshot={snapshot} latestEvents={goalEvents} />);
    expect(screen.getByText(/GOAL!/i)).toBeInTheDocument();
  });
});
