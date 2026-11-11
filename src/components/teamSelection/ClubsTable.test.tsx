import { fireEvent, render, screen, within } from "@testing-library/react";
import { describe, expect, it, vi } from "vitest";

import { createTeam } from "../../test-utils/factories";
import type { ClubSquadRatings } from "../../services/teamsService";
import ClubsTable from "./ClubsTable";

// The backend's keys are resolved by `resolveBackendText`; here a key reads as itself.
vi.mock("../../utils/backendI18n", () => ({
  resolveBackendText: (key: string | undefined, fallback: string) => key ?? fallback,
}));

vi.mock("react-i18next", () => ({
  initReactI18next: { type: "3rdParty", init: () => {} },
  useTranslation: () => ({
    t: (key: string, params?: Record<string, unknown>) =>
      params?.n !== undefined ? `${key}:${params.n}` : key,
    i18n: { language: "en" },
  }),
}));

vi.mock("../../hooks/useRatingTiers", async () => {
  const { cannedRatingTiersModule } = await import("../../test-utils/ratingTiers");
  return cannedRatingTiersModule();
});

const CLUBS = [
  createTeam({ id: "a", name: "Alpha FC", city: "Alton", transfer_budget: 1_000_000 }),
  createTeam({
    id: "b",
    name: "Northbridge Athletic Football and Athletics Club",
    city: "Northbridge",
    transfer_budget: 5_000_000,
  }),
  createTeam({ id: "c", name: "Charlie Rovers", city: "Carlow", transfer_budget: 3_000_000 }),
];
const AVERAGES: Record<string, number | null> = { a: 61, b: 74, c: null };

/** The backend's facts for each club (c has none yet): the figures the table shows and sorts by. */
function factsFor(
  team: (typeof CLUBS)[number],
  reputationKey: string,
  expectationKey: string,
): ClubSquadRatings {
  return {
    team_id: team.id,
    average_rating: 70,
    players: [],
    stadium_name: "Ground",
    stadium_capacity: 10_000,
    transfer_budget: team.transfer_budget,
    weekly_wage_budget: 150_000,
    balance: 1_000_000,
    colors: { primary: "#112233", secondary: "#ffffff" },
    founded_year: 1900,
    competitions: [],
    key_players: [],
    reputation_key: reputationKey,
    opening_expectation: { target_position: 5, label_key: expectationKey },
  };
}
const FACTS: Record<string, ClubSquadRatings> = {
  a: factsFor(CLUBS[0], "teamSelect.repAverage", "teamSelect.expectations.midTable"),
  b: factsFor(CLUBS[1], "teamSelect.repStrong", "teamSelect.expectations.title"),
};

function renderTable(overrides: Partial<Parameters<typeof ClubsTable>[0]> = {}) {
  const handlers = { onSearchChange: vi.fn(), onSelectTeam: vi.fn() };
  render(
    <ClubsTable
      leagueName="First Division"
      clubs={CLUBS}
      clubCount={3}
      search=""
      selectedTeamId={null}
      getTeamAvgOvr={(id) => AVERAGES[id] ?? null}
      getClubFacts={(id) => FACTS[id]}
      {...handlers}
      {...overrides}
    />,
  );
  return handlers;
}

function clubOrder(): string[] {
  return within(screen.getByRole("table", { name: "teamSelect.table.label" }))
    .getAllByRole("button", { pressed: false })
    .filter((button) => button.getAttribute("aria-pressed") !== null)
    .map((button) => button.getAttribute("aria-label") ?? "");
}

describe("ClubsTable", () => {
  /**
   * Given a league with a club whose name is long
   * When the table is drawn
   * Then the name's cell may wrap and is not held to one line (the name is never cut off), with
   *      the club's city under it.
   */
  it("lets a club's name wrap, with its city under it", () => {
    renderTable();

    const name = "Northbridge Athletic Football and Athletics Club";
    const cell = screen.getByText(name).closest("td");
    expect(cell).toHaveClass("whitespace-normal");
    expect(within(cell as HTMLElement).getByText("Northbridge")).toBeInTheDocument();
  });

  /**
   * Given the league's clubs, one of whom has no rated squad yet
   * When the table is drawn
   * Then they open strongest squad first, and the club with no rating is last with a dash, not a
   *      zero.
   */
  it("opens strongest squad first and puts an unrated club last", () => {
    renderTable();

    expect(clubOrder()).toEqual([
      "Northbridge Athletic Football and Athletics Club",
      "Alpha FC",
      "Charlie Rovers",
    ]);
    // His row has no rating chip and reads dashes (his facts have not come either), not a zero.
    const charlie = within(
      screen.getByRole("button", { name: "Charlie Rovers" }).closest("tr") as HTMLElement,
    );
    expect(charlie.queryByTitle(/common\.overallRating/)).toBeNull();
    expect(charlie.getAllByText("—").length).toBeGreaterThan(0);
  });

  /**
   * Given a club chosen
   * When the table is drawn, and then a row is pressed
   * Then the chosen club's button says it is pressed and the others say they are not, and pressing
   *      a row asks for that club.
   */
  it("says which club is chosen, and asks for the one pressed", () => {
    const handlers = renderTable({ selectedTeamId: "a" });

    expect(screen.getByRole("button", { name: "Alpha FC" })).toHaveAttribute(
      "aria-pressed",
      "true",
    );
    expect(screen.getByRole("button", { name: "Charlie Rovers" })).toHaveAttribute(
      "aria-pressed",
      "false",
    );

    fireEvent.click(screen.getByRole("button", { name: "Charlie Rovers" }));
    expect(handlers.onSelectTeam).toHaveBeenCalledWith("c");
  });

  /**
   * Given the backend's facts for two clubs, and none yet for a third
   * When the table is drawn
   * Then each club's reputation word and the board's expectation are the keys the backend names,
   *      its wage budget is already weekly (its unit written once), and the club with no facts yet
   *      reads dashes: the table judges no number.
   */
  it("shows the reputation word and the expectation the backend names, and dashes before it answers", () => {
    renderTable();

    const rowOf = (name: string) =>
      screen.getByRole("button", { name }).closest("tr") as HTMLElement;
    expect(rowOf("Alpha FC")).toHaveTextContent("teamSelect.repAverage");
    expect(rowOf("Alpha FC")).toHaveTextContent("teamSelect.expectations.midTable");
    expect(rowOf("Alpha FC")).toHaveTextContent("150K/wk");
    expect(rowOf("Alpha FC")).not.toHaveTextContent(/week/i);
    expect(rowOf("Northbridge Athletic Football and Athletics Club")).toHaveTextContent(
      "teamSelect.repStrong",
    );
    expect(rowOf("Charlie Rovers")).not.toHaveTextContent("teamSelect.rep");
  });

  it("sorts by a pressed header, and the other way on a second press", () => {
    renderTable();

    fireEvent.click(screen.getByRole("button", { name: "teamSelect.table.transfers" }));
    expect(clubOrder()[0]).toBe("Northbridge Athletic Football and Athletics Club");

    fireEvent.click(screen.getByRole("button", { name: "teamSelect.table.transfers" }));
    expect(clubOrder()[0]).toBe("Alpha FC");
  });

  it("names the league and counts its clubs, and hands the search on as typed", () => {
    const handlers = renderTable();

    expect(screen.getByRole("heading", { name: /First Division/ })).toHaveTextContent(
      "teamSelect.clubCount:3",
    );
    fireEvent.change(screen.getByRole("searchbox", { name: "teamSelect.searchClubs" }), {
      target: { value: "bex" },
    });
    expect(handlers.onSearchChange).toHaveBeenCalledWith("bex");
  });

  /**
   * Given a search that leaves two of the league's three clubs
   * When the table is drawn
   * Then a polite status says how many clubs are listed, so the change is heard as it is typed.
   */
  it("says aloud how many clubs the search leaves", () => {
    renderTable({ clubs: CLUBS.slice(0, 2), search: "a" });

    expect(screen.getByRole("status")).toHaveTextContent("teamSelect.clubCount:2");
  });

  it("says no club matches when the search leaves none", () => {
    renderTable({ clubs: [], search: "zzz" });

    expect(screen.getByText("teamSelect.noClubsMatch")).toBeInTheDocument();
  });
});
