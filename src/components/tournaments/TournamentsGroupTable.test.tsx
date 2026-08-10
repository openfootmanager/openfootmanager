import { fireEvent, render, screen } from "@testing-library/react";
import { describe, expect, it, vi } from "vitest";

import TournamentsGroupTable from "./TournamentsGroupTable";
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

type Group = NonNullable<LeagueData["groups"]>[number];

function standing(overrides: Partial<StandingData> = {}): StandingData {
  return {
    team_id: "team-1",
    played: 3,
    won: 1,
    drawn: 1,
    lost: 1,
    goals_for: 4,
    goals_against: 4,
    points: 4,
    ...overrides,
  };
}

// Deliberately out of order: the component is what has to sort them.
function group(overrides: Partial<Group> = {}): Group {
  return {
    id: "group-a",
    name: "A",
    standings: [
      standing({ team_id: "second", points: 6, goals_for: 5, goals_against: 4 }),
      standing({ team_id: "first", points: 9, goals_for: 8, goals_against: 2 }),
      standing({ team_id: "third", points: 1, goals_for: 2, goals_against: 9 }),
    ],
    ...overrides,
  } as Group;
}

function teamLookup(
  overrides: Partial<TournamentsTeamLookup> = {},
): TournamentsTeamLookup {
  return {
    userTeamId: "second",
    isClubTeam: () => true,
    resolveTeamName: (id) => `Name of ${id}`,
    onSelectTeam: vi.fn(),
    ...overrides,
  };
}

function renderGroup(
  props: Partial<React.ComponentProps<typeof TournamentsGroupTable>> = {},
) {
  return render(
    <TournamentsGroupTable group={group()} teams={teamLookup()} {...props} />,
  );
}

describe("TournamentsGroupTable", () => {
  // The table shows its columns by position alone — there is no room for a
  // visible header row — so the names have to exist for assistive tech.
  it("names its columns for a screen reader", () => {
    renderGroup();

    for (const name of [
      "common.position",
      "common.team",
      "common.played",
      "common.pts",
    ]) {
      expect(screen.getByRole("columnheader", { name })).toBeInTheDocument();
    }
  });

  it("orders the group by table position, not by the order it arrived in", () => {
    renderGroup();

    const names = screen
      .getAllByRole("button")
      .map((button) => button.textContent);

    expect(names).toEqual(["Name of first", "Name of second", "Name of third"]);
  });

  it("picks out the managed club's row", () => {
    renderGroup();

    expect(
      screen.getByTestId("tournaments-group-standing-second").className,
    ).toContain("bg-primary-50");
    expect(
      screen.getByTestId("tournaments-group-standing-first").className,
    ).not.toContain("bg-primary-50");
  });

  // The row is clickable and so is the name inside it. A click on the name
  // bubbles to the row, so without stopPropagation one press opens two teams.
  it("opens a club exactly once when its name is pressed", () => {
    const onSelectTeam = vi.fn();
    renderGroup({ teams: teamLookup({ onSelectTeam }) });

    fireEvent.click(screen.getByRole("button", { name: "Name of first" }));

    expect(onSelectTeam).toHaveBeenCalledTimes(1);
    expect(onSelectTeam).toHaveBeenCalledWith("first");
  });

  it("leaves national teams inert, having no page to open", () => {
    const onSelectTeam = vi.fn();
    renderGroup({ teams: teamLookup({ isClubTeam: () => false, onSelectTeam }) });

    expect(screen.queryByRole("button")).toBeNull();

    fireEvent.click(screen.getByTestId("tournaments-group-standing-first"));

    expect(onSelectTeam).not.toHaveBeenCalled();
  });
});
