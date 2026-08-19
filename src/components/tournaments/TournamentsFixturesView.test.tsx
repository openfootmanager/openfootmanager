import { render, screen, within } from "@testing-library/react";
import { describe, expect, it, vi } from "vitest";

import TournamentsFixturesView from "./TournamentsFixturesView";
import { formatMatchDate } from "../../lib/helpers";
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
  const view = render(
    <TournamentsFixturesView sortedMatchdays={matchdays} teams={teamLookup()} />,
  );
  return {
    ...view,
    // Re-renders the same mounted instance, which is what makes a changing hook count observable.
    rerenderWith: (next: Array<[number, FixtureData[]]>) =>
      view.rerender(
        <TournamentsFixturesView sortedMatchdays={next} teams={teamLookup()} />,
      ),
  };
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
    // Without this the test is about headings, not cards: dropping the <Card> wrapper entirely
    // leaves both headings in place and the assertion above still passes.
    expect(
      headings.map((h) => h.closest("[class*='rounded-xl']")).filter(Boolean),
    ).toHaveLength(2);
  });

  // The mock keeps a hook inside `useTranslation` so a Rules-of-Hooks violation surfaces here
  // rather than in the app. That only bites if the hook *count* can change between renders of one
  // mounted instance — moving `useTranslation()` inside the matchday map is invisible to any
  // number of separate first renders.
  it("survives the matchday count changing under it", () => {
    const { rerenderWith } = renderView([[1, [fixture({ id: "f1" })]]]);

    expect(() =>
      rerenderWith([
        [1, [fixture({ id: "f1" })]],
        [2, [fixture({ id: "f2" })]],
      ]),
    ).not.toThrow();

    expect(screen.getAllByRole("heading")).toHaveLength(2);
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

    // Each club side renders as a button named for the team, so the two rows are
    // countable without reaching for a test id.
    expect(
      screen.getByRole("button", { name: "Name of home" }),
    ).toBeInTheDocument();
    expect(
      screen.getByRole("button", { name: "Name of third" }),
    ).toBeInTheDocument();
    expect(
      screen.getAllByRole("button", { name: "Name of away" }),
    ).toHaveLength(2);
  });

  // Every fixture in a round shares a date, so the header takes the first one's.
  // The second fixture carries a different date on purpose: with only one
  // fixture rendered, "takes the first one's date" and "takes any date" look the
  // same, and the test would pass either way.
  it("dates the matchday from its first fixture", () => {
    renderView([
      [
        4,
        [
          fixture({ id: "f1", date: "2026-09-12" }),
          fixture({ id: "f2", date: "2026-09-13" }),
        ],
      ],
    ]);

    const heading = screen.getByRole("heading");
    expect(heading).toHaveTextContent(
      `Matchday 4 — ${formatMatchDate("2026-09-12")}`,
    );
    expect(heading.textContent).not.toContain(formatMatchDate("2026-09-13"));
  });

  it("renders nothing when the competition has no fixtures yet", () => {
    const { container } = renderView([]);

    expect(screen.queryByRole("heading")).toBeNull();
    expect(within(container).queryByRole("button")).toBeNull();
  });
});
