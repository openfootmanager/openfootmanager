import { fireEvent, render, screen } from "@testing-library/react";
import { describe, expect, it, vi } from "vitest";

import StandingsTable from "./StandingsTable";
import type { TournamentsTeamLookup } from "./teamLookup";
import type { StandingData } from "../../store/gameStore";

vi.mock("react-i18next", async () => {
  const { useRef } = await import("react");
  return {
    useTranslation: () => {
      useRef(null);
      return { t: (key: string) => key };
    },
  };
});

function standing(overrides: Partial<StandingData> = {}): StandingData {
  return {
    team_id: "team-1",
    played: 10,
    won: 6,
    drawn: 2,
    lost: 2,
    goals_for: 18,
    goals_against: 9,
    points: 20,
    ...overrides,
  };
}

const table = [
  standing({ team_id: "top", points: 20, goals_for: 18, goals_against: 9 }),
  standing({ team_id: "mid", points: 12, goals_for: 10, goals_against: 10 }),
  standing({ team_id: "bottom", points: 4, goals_for: 6, goals_against: 20 }),
];

function teamLookup(
  overrides: Partial<TournamentsTeamLookup> = {},
): TournamentsTeamLookup {
  return {
    userTeamId: "mid",
    isClubTeam: () => true,
    resolveTeamName: (id) => `Name of ${id}`,
    onSelectTeam: vi.fn(),
    ...overrides,
  };
}

function renderTable(
  props: Partial<React.ComponentProps<typeof StandingsTable>> = {},
) {
  return render(
    <StandingsTable
      standings={table}
      variant="full"
      teams={teamLookup()}
      testIdPrefix="row"
      {...props}
    />,
  );
}

describe("StandingsTable", () => {
  it("shows goals for and against in the full table", () => {
    renderTable();

    expect(screen.getByText("common.gf")).toBeInTheDocument();
    expect(screen.getByText("common.ga")).toBeInTheDocument();
  });

  // The overview shows the table beside everything else, so it drops the two
  // goal columns. Losing that distinction is the way a shared table regresses.
  it("drops goals for and against in the compact table", () => {
    renderTable({ variant: "compact" });

    expect(screen.queryByText("common.gf")).not.toBeInTheDocument();
    expect(screen.queryByText("common.ga")).not.toBeInTheDocument();
    // The rest of the columns stay.
    expect(screen.getByText("common.played")).toBeInTheDocument();
    expect(screen.getByText("common.gd")).toBeInTheDocument();
    expect(screen.getByText("common.pts")).toBeInTheDocument();
  });

  it("signs goal difference and leaves a level record unsigned", () => {
    renderTable();

    expect(screen.getByText("+9")).toBeInTheDocument();
    expect(screen.getByText("0")).toBeInTheDocument();
    expect(screen.getByText("-14")).toBeInTheDocument();
  });

  it("marks the promotion and relegation places when a competition has them", () => {
    renderTable({ zones: { promotionSlots: 1, relegationSlots: 1 } });

    expect(screen.getByTestId("tournaments-promotion-top")).toBeInTheDocument();
    expect(
      screen.getByTestId("tournaments-relegation-bottom"),
    ).toBeInTheDocument();
    expect(screen.queryByTestId("tournaments-promotion-mid")).toBeNull();
  });

  it("leaves every place unmarked when no zones are given", () => {
    renderTable();

    expect(screen.queryByTestId("tournaments-promotion-top")).toBeNull();
    expect(screen.queryByTestId("tournaments-relegation-bottom")).toBeNull();
  });

  it("highlights the row belonging to the club being managed", () => {
    renderTable();

    expect(screen.getByTestId("row-mid").className).toContain("bg-primary-50");
    expect(screen.getByTestId("row-top").className).not.toContain(
      "bg-primary-50",
    );
  });

  it("opens a team from its row", () => {
    const onSelectTeam = vi.fn();
    renderTable({ teams: teamLookup({ onSelectTeam }) });

    fireEvent.click(screen.getByTestId("row-bottom"));

    expect(onSelectTeam).toHaveBeenCalledWith("bottom");
  });
});
