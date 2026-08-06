import { fireEvent, render, screen } from "@testing-library/react";
import { describe, expect, it, vi } from "vitest";

import TournamentsFixtureRow from "./TournamentsFixtureRow";
import type { TournamentsTeamLookup } from "./teamLookup";
import type { FixtureData } from "../../store/gameStore";

vi.mock("react-i18next", async () => {
  const { useRef } = await import("react");
  return {
    useTranslation: () => {
      useRef(null);
      return { t: (key: string) => key };
    },
  };
});

function teamLookup(
  overrides: Partial<TournamentsTeamLookup> = {},
): TournamentsTeamLookup {
  return {
    userTeamId: "home",
    isClubTeam: () => true,
    resolveTeamName: (id) => `Name of ${id}`,
    onSelectTeam: vi.fn(),
    ...overrides,
  };
}

function fixture(overrides: Partial<FixtureData> = {}): FixtureData {
  return {
    id: "fixture-1",
    home_team_id: "home",
    away_team_id: "away",
    status: "Scheduled",
    ...overrides,
  } as unknown as FixtureData;
}

function renderRow(
  props: Partial<React.ComponentProps<typeof TournamentsFixtureRow>> = {},
) {
  return render(
    <TournamentsFixtureRow
      fixture={fixture()}
      testId="row"
      teams={teamLookup()}
      {...props}
    />,
  );
}

describe("TournamentsFixtureRow", () => {
  it("shows both teams and no score before a fixture is played", () => {
    renderRow();

    expect(screen.getByText("Name of home")).toBeInTheDocument();
    expect(screen.getByText("Name of away")).toBeInTheDocument();
    expect(screen.getByText("common.vs")).toBeInTheDocument();
  });

  it("shows the score once a fixture is complete", () => {
    renderRow({
      fixture: fixture({
        status: "Completed",
        result: { home_goals: 3, away_goals: 1 },
      } as Partial<FixtureData>),
    });

    expect(screen.getByText("3 - 1")).toBeInTheDocument();
    expect(screen.queryByText("common.vs")).toBeNull();
  });

  // A completed fixture with no result payload must not claim a scoreline.
  it("falls back to the placeholder when a completed fixture has no result", () => {
    renderRow({
      fixture: fixture({ status: "Completed" } as Partial<FixtureData>),
    });

    expect(screen.getByText("common.vs")).toBeInTheDocument();
  });

  it("picks out a fixture the managed club is playing in", () => {
    const { container } = renderRow();

    expect(screen.getByTestId("row").className).toContain("bg-primary-50/50");
    expect(container).toBeTruthy();
  });

  it("leaves a fixture between other clubs unhighlighted", () => {
    renderRow({ teams: teamLookup({ userTeamId: "someone-else" }) });

    expect(screen.getByTestId("row").className).not.toContain("bg-primary-50/50");
  });

  // The names used to be clickable spans, which keyboard users could not reach.
  it("opens a club from a focusable control", () => {
    const onSelectTeam = vi.fn();
    renderRow({ teams: teamLookup({ onSelectTeam }) });

    fireEvent.click(screen.getByRole("button", { name: "Name of away" }));

    expect(onSelectTeam).toHaveBeenCalledWith("away");
  });

  it("leaves national teams as plain text, having no page to open", () => {
    renderRow({ teams: teamLookup({ isClubTeam: () => false }) });

    expect(screen.queryByRole("button")).toBeNull();
    expect(screen.getByText("Name of home")).toBeInTheDocument();
  });
});
