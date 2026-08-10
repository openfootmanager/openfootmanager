import { render, screen } from "@testing-library/react";
import { describe, expect, it, vi } from "vitest";

import TournamentsStandingsView from "./TournamentsStandingsView";
import type { TournamentsTeamLookup } from "./teamLookup";
import type { LeagueData, StandingData } from "../../store/gameStore";

vi.mock("react-i18next", async () => {
  const { useRef } = await import("react");
  return {
    useTranslation: () => {
      useRef(null);
      return { t: (key: string) => key };
    },
  };
});

vi.mock("./KnockoutBracket", () => ({
  default: () => <div data-testid="knockout-bracket" />,
}));

const teams: TournamentsTeamLookup = {
  userTeamId: "team-1",
  isClubTeam: () => true,
  resolveTeamName: (id) => `Name of ${id}`,
  onSelectTeam: vi.fn(),
};

function standing(teamId: string): StandingData {
  return {
    team_id: teamId,
    played: 4,
    won: 2,
    drawn: 1,
    lost: 1,
    goals_for: 7,
    goals_against: 5,
    points: 7,
  };
}

const league = {
  id: "league-1",
  name: "Premier",
  season: 2026,
  fixtures: [],
  standings: [standing("team-1")],
} as unknown as LeagueData;

const group = {
  id: "group-a",
  name: "A",
  standings: [standing("team-2")],
} as unknown as NonNullable<LeagueData["groups"]>[number];

const round = {
  id: "round-1",
  name: "Final",
  completed: false,
} as unknown as NonNullable<LeagueData["knockout_rounds"]>[number];

function renderView(
  props: Partial<React.ComponentProps<typeof TournamentsStandingsView>> = {},
) {
  return render(
    <TournamentsStandingsView
      league={league}
      standings={[standing("team-1")]}
      groups={[]}
      knockoutRounds={[]}
      isKnockout={false}
      isPreseason={false}
      zones={{ promotionSlots: 0, relegationSlots: 0 }}
      teams={teams}
      {...props}
    />,
  );
}

// Four competition shapes share this view and are chosen in a fixed order.
// Early returns make that order easy to shuffle without noticing, so each
// step of the precedence gets pinned here.
describe("TournamentsStandingsView", () => {
  it("shows a league its table", () => {
    renderView();

    expect(screen.getByTestId("tournaments-standing-team-1")).toBeInTheDocument();
  });

  it("shows a qualifying competition its groups instead of a table", () => {
    renderView({ groups: [group] });

    expect(screen.getByTestId("tournaments-group-group-a")).toBeInTheDocument();
    expect(screen.queryByTestId("tournaments-standing-team-1")).toBeNull();
  });

  it("shows a knockout its bracket", () => {
    renderView({ isKnockout: true, knockoutRounds: [round] });

    expect(screen.getByTestId("knockout-bracket")).toBeInTheDocument();
    expect(screen.queryByTestId("tournaments-standing-team-1")).toBeNull();
  });

  // A cup with a group stage running shows both, groups above the bracket.
  it("shows a knockout's group stage alongside its bracket", () => {
    renderView({ isKnockout: true, knockoutRounds: [round], groups: [group] });

    expect(screen.getByTestId("tournaments-group-group-a")).toBeInTheDocument();
    expect(screen.getByTestId("knockout-bracket")).toBeInTheDocument();
  });

  // Before the draw a cup has neither a bracket nor groups. An empty panel with
  // no explanation is what this replaced.
  it("explains an undrawn cup rather than showing an empty panel", () => {
    renderView({ isKnockout: true });

    expect(screen.getByText("season.standingsLocked")).toBeInTheDocument();
    expect(screen.queryByTestId("knockout-bracket")).toBeNull();
  });

  it("takes the bracket over the table for a knockout in preseason", () => {
    renderView({ isKnockout: true, knockoutRounds: [round], isPreseason: true });

    expect(screen.getByTestId("knockout-bracket")).toBeInTheDocument();
    expect(screen.queryByText("season.standingsLocked")).toBeNull();
  });

  // Groups outrank the preseason lock: a qualifying competition's groups are
  // drawn before a ball is kicked, so there is something real to show.
  it("shows a qualifying competition's groups even in preseason", () => {
    renderView({ groups: [group], isPreseason: true });

    expect(screen.getByTestId("tournaments-group-group-a")).toBeInTheDocument();
    expect(screen.queryByText("season.standingsLocked")).toBeNull();
  });

  it("locks a league's table until its season starts", () => {
    renderView({ isPreseason: true });

    expect(screen.getByText("season.standingsLocked")).toBeInTheDocument();
    expect(screen.queryByTestId("tournaments-standing-team-1")).toBeNull();
  });

  it("explains the promotion and relegation marks when a league has them", () => {
    renderView({ zones: { promotionSlots: 1, relegationSlots: 1 } });

    expect(screen.getByText("schedule.promotionZone")).toBeInTheDocument();
    expect(screen.getByText("schedule.relegationZone")).toBeInTheDocument();
  });

  it("leaves the key out when a league has neither", () => {
    renderView();

    expect(screen.queryByText("schedule.promotionZone")).toBeNull();
    expect(screen.queryByText("schedule.relegationZone")).toBeNull();
  });
});
