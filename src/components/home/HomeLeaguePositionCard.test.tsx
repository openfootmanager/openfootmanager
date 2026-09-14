import { render, screen } from "@testing-library/react";
import { describe, expect, it, vi } from "vitest";

import HomeLeaguePositionCard from "./HomeLeaguePositionCard";

vi.mock("react-i18next", () => ({
  useTranslation: () => ({
    t: (key: string, params?: Record<string, string | number>) => {
      if (key === "home.standings") return "Standings";
      if (key === "home.leaguePosition") return "League Position";
      if (key === "season.phases.Preseason") return "Preseason";
      if (key === "season.startsOn") return `Starts on ${params?.date}`;
      if (key === "season.noOpener") return "No opener";
      if (key === "season.standingsLocked") return "Standings are locked before kickoff.";
      if (key === "common.place.2") return "2nd place";
      if (key === "home.winningStreak") return "Winning Streak";
      if (key === "home.noLeague") return "No league data";
      return key;
    },
  }),
}));

describe("HomeLeaguePositionCard", () => {
  it("renders preseason standings lock messaging", () => {
    render(
      <HomeLeaguePositionCard
        isPreseason={true}
        phase="Preseason"
        seasonStartLabel="Jan 12"
        myStanding={null}
        myStandingData={null}
        teamForm={[]}
      />,
    );

    expect(screen.getByText("Preseason")).toBeInTheDocument();
    expect(screen.getByText("Starts on Jan 12")).toBeInTheDocument();
    expect(screen.getByText("Standings are locked before kickoff.")).toBeInTheDocument();
  });

  it("renders league table summary and form streak data", () => {
    render(
      <HomeLeaguePositionCard
        isPreseason={false}
        phase="RegularSeason"
        seasonStartLabel={null}
        myStanding={2}
        myStandingData={{
          team_id: "team-1",
          played: 5,
          won: 3,
          drawn: 1,
          lost: 1,
          goals_for: 9,
          goals_against: 4,
          points: 10,
        }}
        teamForm={["W", "W", "W"]}
      />,
    );

    expect(screen.getByText("League Position")).toBeInTheDocument();
    expect(screen.getByText("2")).toBeInTheDocument();
    expect(screen.getByText("2nd place")).toBeInTheDocument();
    expect(screen.getByText("10 pts")).toBeInTheDocument();
    expect(screen.getByText("Winning Streak")).toBeInTheDocument();
  });
});
