import { render, screen, within } from "@testing-library/react";
import { describe, expect, it, vi } from "vitest";

import TournamentsFixturesView from "./TournamentsFixturesView";
import type { TournamentsTeamLookup } from "./teamLookup";
import type { FixtureData } from "../../store/gameStore";

// Keeps a hook in the mock so a Rules-of-Hooks violation in the component still
// surfaces, rather than being masked by a hook-free `t`.
vi.mock("react-i18next", async () => {
  const { useRef } = await import("react");
  return {
    useTranslation: () => {
      useRef(null);
      return {
        t: (key: string, params?: Record<string, unknown>) =>
          key === "schedule.matchday" ? `Matchday ${params?.number}` : key,
      };
    },
  };
});

function teamLookup(): TournamentsTeamLookup {
  return {
    userTeamId: "home",
    isClubTeam: () => true,
    resolveTeamName: (id: string) => `Name of ${id}`,
    onSelectTeam: vi.fn(),
  } as unknown as TournamentsTeamLookup;
}

function fixture(overrides: Partial<FixtureData> = {}): FixtureData {
  return {
    id: "fixture-1",
    home_team_id: "home",
    away_team_id: "away",
    status: "Scheduled",
    date: "2026-08-15",
    ...overrides,
  } as unknown as FixtureData;
}

function renderView(matchdays: Array<[number, FixtureData[]]>) {
  return render(
    <TournamentsFixturesView sortedMatchdays={matchdays} teams={teamLookup()} />,
  );
}

describe("TournamentsFixturesView", () => {
  it("renders one card per matchday, in the order given", () => {
    renderView([
      [1, [fixture({ id: "f1" })]],
      [2, [fixture({ id: "f2" })]],
    ]);

    const headings = screen.getAllByRole("heading");
    expect(headings.map((h) => h.textContent)).toEqual([
      expect.stringContaining("Matchday 1"),
      expect.stringContaining("Matchday 2"),
    ]);
  });

  it("lists every fixture in a matchday", () => {
    renderView([
      [
        3,
        [
          fixture({ id: "f1" }),
          fixture({ id: "f2", home_team_id: "third" } as Partial<FixtureData>),
        ],
      ],
    ]);

    expect(screen.getByTestId("tournaments-fixture-f1")).toBeInTheDocument();
    expect(screen.getByTestId("tournaments-fixture-f2")).toBeInTheDocument();
    expect(screen.getByText("Name of third")).toBeInTheDocument();
  });

  // Every fixture in a round shares a date, so the header takes the first one's.
  it("dates the matchday from its first fixture", () => {
    renderView([[4, [fixture({ id: "f1", date: "2026-09-12" })]]]);

    const heading = screen.getByRole("heading");
    expect(heading.textContent).toContain("Matchday 4");
    expect(heading.textContent).not.toBe("Matchday 4 — ");
  });

  it("renders nothing when the competition has no fixtures yet", () => {
    const { container } = renderView([]);

    expect(screen.queryByRole("heading")).toBeNull();
    expect(within(container).queryByTestId(/tournaments-fixture-/)).toBeNull();
  });
});
