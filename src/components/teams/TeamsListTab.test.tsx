import { act, fireEvent, render, screen, waitFor, within } from "@testing-library/react";
import { beforeEach, describe, expect, it, vi } from "vitest";
import { invoke } from "@tauri-apps/api/core";

import type { GameStateData } from "../../store/gameStore";
import { useSettingsStore } from "../../store/settingsStore";
import type { TeamCard, TeamsDirectory, TeamsDirectoryQuery } from "../../services/teamsService";
import TeamsListTab from "./TeamsListTab";
import { createSeasonContext } from "../../test-utils/factories";

vi.mock("@tauri-apps/api/core", () => ({
  invoke: vi.fn(),
}));

// One `t` for every render, as react-i18next keeps it while the language holds: the list refetches
// when `t` changes, so a new one each render would refetch without end.
vi.mock("react-i18next", () => {
  const translation = {
    t: (key: string, fallback?: string | { defaultValue?: string }) => {
      const labels: Record<string, string> = {
        "teams.yourTeam": "Your Team",
        "common.position": "Position",
        "teams.squad": "Squad",
        "teams.avgOvr": "Avg OVR",
        "teams.rep": "Rep",
        "common.value": "Value",
        "common.pts": "Pts",
        "common.played": "P",
        "common.team": "Team",
        "common.listViewTable": "Table",
        "common.listViewCards": "Cards",
        "teams.est": "Est",
        "teams.searchPlaceholder": "Search clubs",
        "teams.noResults": "No clubs match your search.",
        "teams.otherClubs": "Other clubs",
        "be.error.noActiveGameSession": "No active game session",
        "common.won": "W",
        "common.drawn": "D",
        "common.lost": "L",
      };
      if (labels[key]) return labels[key];
      if (fallback && typeof fallback === "object") {
        return fallback.defaultValue ?? key;
      }
      return fallback ?? key;
    },
    i18n: { language: "en" },
  };
  return { useTranslation: () => translation };
});

vi.mock("../ui", async (importOriginal) => {
  const actual = await importOriginal<typeof import("../ui")>();
  return {
    ...actual,
    TeamLocation: ({ city, countryCode }: { city: string; countryCode: string }) => (
      <span>{`${city}, ${countryCode}`}</span>
    ),
  };
});

const mockedInvoke = vi.mocked(invoke);

function gameStateWithManagerTeam(teamId: string | null): GameStateData {
  return {
    season_context: createSeasonContext(),
    clock: {
      current_date: "2026-08-01T00:00:00Z",
      start_date: "2026-07-01T00:00:00Z",
    },
    manager: {
      display_name: "Jane Doe",
      id: "manager-1",
      first_name: "Jane",
      last_name: "Doe",
      date_of_birth: "1980-01-01",
      nationality: "GB",
      reputation: 50,
      satisfaction: 50,
      fan_approval: 50,
      team_id: teamId,
      career_stats: {
        matches_managed: 0,
        wins: 0,
        draws: 0,
        losses: 0,
        trophies: 0,
        best_finish: null,
      },
      career_history: [],
    },
    teams: [],
    players: [],
    staff: [],
    messages: [],
    news: [],
    league: null,
    scouting_assignments: [],
    board_objectives: [],
  };
}

function buildCard(overrides: {
  id: string;
  name: string;
  city?: string;
  short_name?: string;
  founded_year?: number;
  play_style?: string;
  league_pos: number;
  points?: number;
  avg_ovr?: number | null;
  reputation?: number;
}): TeamCard {
  return {
    team: {
      id: overrides.id,
      name: overrides.name,
      short_name: overrides.short_name ?? overrides.name.slice(0, 3).toUpperCase(),
      city: overrides.city ?? "London",
      country: "GB",
      colors: { primary: "#000000", secondary: "#ffffff" },
      formation: "4-4-2",
      play_style: overrides.play_style ?? "Balanced",
      founded_year: overrides.founded_year ?? 1900,
      reputation: overrides.reputation ?? 50,
      media: { logo: null },
    },
    roster_size: 22,
    avg_ovr: overrides.avg_ovr === undefined ? 65 : overrides.avg_ovr,
    total_value: 1_000_000,
    league_pos: overrides.league_pos,
    standing:
      overrides.points !== undefined
        ? {
            played: 1,
            won: 0,
            drawn: 0,
            lost: 0,
            goals_for: 0,
            goals_against: 0,
            points: overrides.points,
          }
        : null,
  };
}

function applySearch(cards: TeamCard[], search: string | null): TeamCard[] {
  if (!search) return cards;
  const needle = search.toLowerCase();
  return cards.filter(
    (card) =>
      card.team.name.toLowerCase().includes(needle) ||
      card.team.city.toLowerCase().includes(needle),
  );
}

function buildDirectory(cards: TeamCard[], search: string | null): TeamsDirectory {
  const filtered = applySearch(cards, search);
  if (filtered.length === 0) {
    return { regions: [] };
  }
  const sorted = [...filtered].sort((a, b) => a.league_pos - b.league_pos);
  return {
    regions: [
      {
        id: "europe",
        team_count: sorted.length,
        leagues: [
          {
            id: "league-1",
            name: "League",
            teams: sorted,
          },
        ],
      },
    ],
  };
}

function setupDirectoryMock(cards: TeamCard[]) {
  mockedInvoke.mockImplementation(async (cmd: string, args?: unknown) => {
    if (cmd !== "get_teams_directory") return undefined;
    const query = (args as { query: TeamsDirectoryQuery }).query;
    return buildDirectory(cards, query.search);
  });
}

function setListView(view: "table" | "cards") {
  useSettingsStore.setState({
    settings: { ...useSettingsStore.getState().settings, list_view: view },
  });
}

/** A card's terms and what each says: its status band's and its two facts'. */
function cardTerms(card: HTMLElement): Record<string, string | undefined> {
  return Object.fromEntries(
    within(card)
      .getAllByRole("term")
      .map((term) => [term.textContent ?? "", term.nextElementSibling?.textContent ?? undefined]),
  );
}

/** A card's hero, the big figure at the end of its header. */
function cardHero(card: HTMLElement): HTMLElement {
  const hero = card.querySelector("header")?.lastElementChild;
  if (!(hero instanceof HTMLElement)) throw new Error("the card has no hero");
  return hero;
}

describe("TeamsListTab", () => {
  beforeEach(() => {
    mockedInvoke.mockReset();
    setListView("cards");
  });

  it("orders teams by league position and marks the user team", async () => {
    setupDirectoryMock([
      buildCard({ id: "team-1", name: "Alpha FC", league_pos: 2, points: 1 }),
      buildCard({ id: "team-2", name: "Beta FC", league_pos: 1, points: 3 }),
    ]);

    render(<TeamsListTab gameState={gameStateWithManagerTeam("team-1")} onSelectTeam={vi.fn()} />);

    await screen.findByText("Beta FC");
    const headings = screen.getAllByRole("heading", { level: 3 });
    expect(headings[0]).toHaveTextContent("Beta FC");
    expect(headings[1]).toHaveTextContent("Alpha FC");
    expect(screen.getByText("Your Team")).toBeInTheDocument();
  });

  /**
   * Given a division rival whose card carries an average of 71, and a club in another division
   *       whose card the backend sends with no average
   * When the clubs list shows them
   * Then the rival's card shows 71 and the other's "–", beside its reputation
   */
  it("shows a dash for the average of a club outside the manager's division", async () => {
    setupDirectoryMock([
      buildCard({ id: "rival", name: "Rival FC", league_pos: 1, avg_ovr: 71 }),
      buildCard({ id: "faraway", name: "Faraway FC", league_pos: 2, avg_ovr: null }),
    ]);

    render(<TeamsListTab gameState={gameStateWithManagerTeam("rival")} onSelectTeam={vi.fn()} />);

    await screen.findByText("Faraway FC");
    const averages = screen
      .getAllByText("Avg OVR")
      .map((label) => label.nextElementSibling?.textContent);
    expect(averages).toEqual(["71", "–"]);
  });

  it("filters clubs by name via the search box", async () => {
    setupDirectoryMock([
      buildCard({ id: "team-1", name: "Alpha FC", league_pos: 2 }),
      buildCard({ id: "team-2", name: "Beta FC", league_pos: 1 }),
    ]);

    render(<TeamsListTab gameState={gameStateWithManagerTeam("team-1")} onSelectTeam={vi.fn()} />);

    await screen.findByText("Alpha FC");

    fireEvent.change(screen.getByPlaceholderText("Search clubs"), {
      target: { value: "beta" },
    });

    await waitFor(() => {
      expect(screen.queryByText("Alpha FC")).not.toBeInTheDocument();
    });
    expect(screen.getByText("Beta FC")).toBeInTheDocument();
  });

  it("shows an empty state when no clubs match", async () => {
    setupDirectoryMock([buildCard({ id: "team-1", name: "Alpha FC", league_pos: 1 })]);

    render(<TeamsListTab gameState={gameStateWithManagerTeam("team-1")} onSelectTeam={vi.fn()} />);

    await screen.findByText("Alpha FC");

    fireEvent.change(screen.getByPlaceholderText("Search clubs"), {
      target: { value: "zzzzz" },
    });

    await waitFor(() => {
      expect(screen.getByText("No clubs match your search.")).toBeInTheDocument();
    });
  });

  it("selects a team when its card is clicked", async () => {
    const onSelectTeam = vi.fn();
    setupDirectoryMock([
      buildCard({ id: "team-1", name: "Alpha FC", league_pos: 2 }),
      buildCard({ id: "team-2", name: "Beta FC", league_pos: 1 }),
    ]);

    render(
      <TeamsListTab gameState={gameStateWithManagerTeam("team-1")} onSelectTeam={onSelectTeam} />,
    );

    fireEvent.click(await screen.findByText("Beta FC"));

    expect(onSelectTeam).toHaveBeenCalledWith("team-2");
  });

  /**
   * Given the league leader on 10 points from three wins and a draw, a squad of 22 and a
   *       reputation of 64
   * When its card is drawn
   * Then its hero is its league position, 1; its status gives the standing (3, 1, 0, 10 points)
   *      and the squad's average; and its two facts are the squad's size and the reputation.
   */
  it("draws a club's card with its position, standing, average, squad and reputation", async () => {
    const leader = buildCard({
      id: "lead",
      name: "Leader FC",
      league_pos: 1,
      reputation: 64,
      avg_ovr: 71,
    });
    leader.standing = {
      played: 4,
      won: 3,
      drawn: 1,
      lost: 0,
      goals_for: 9,
      goals_against: 2,
      points: 10,
    };
    setupDirectoryMock([leader]);
    render(<TeamsListTab gameState={gameStateWithManagerTeam("lead")} onSelectTeam={vi.fn()} />);

    const card = await screen.findByRole("article", { name: "Leader FC" });
    expect(cardTerms(card)).toEqual({
      W: "3",
      D: "1",
      L: "0",
      Pts: "10",
      "Avg OVR": "71",
      Squad: "22",
      Rep: "64",
    });
    expect(cardHero(card)).toHaveTextContent("Position 1");
  });

  /**
   * Given a club with no league position yet and no standing
   * When its card is drawn
   * Then its hero reads "—" and its points read "—" with no won, drawn and lost, rather than
   *      zeros that look like a record.
   */
  it("draws an unranked club's position and standing as a dash", async () => {
    setupDirectoryMock([buildCard({ id: "new", name: "New FC", league_pos: 0, avg_ovr: 60 })]);
    render(<TeamsListTab gameState={gameStateWithManagerTeam("new")} onSelectTeam={vi.fn()} />);

    const card = await screen.findByRole("article", { name: "New FC" });
    expect(cardHero(card)).toHaveTextContent("Position —");
    expect(cardTerms(card)).toEqual({ Pts: "—", "Avg OVR": "60", Squad: "22", Rep: "50" });
  });

  /**
   * Given the directory fails to load with the backend's error key
   * When the list draws the failure
   * Then it says it in the player's language, as an alert above the list.
   */
  it("says why the directory failed to load, in the player's words", async () => {
    mockedInvoke.mockRejectedValue("be.error.noActiveGameSession");
    render(<TeamsListTab gameState={gameStateWithManagerTeam(null)} onSelectTeam={vi.fn()} />);

    expect(await screen.findByRole("alert")).toHaveTextContent("No active game session");
  });

  /**
   * Given a club's card
   * When a keyboard user reaches it
   * Then its name is a button that opens the club, where the old card was a clickable box the
   *      keyboard could not reach.
   */
  it("opens a club from its card's name, a button the keyboard reaches", async () => {
    const onSelectTeam = vi.fn();
    setupDirectoryMock([buildCard({ id: "beta", name: "Beta FC", league_pos: 1 })]);
    render(
      <TeamsListTab gameState={gameStateWithManagerTeam("beta")} onSelectTeam={onSelectTeam} />,
    );

    const card = await screen.findByRole("article", { name: "Beta FC" });
    const open = within(card).getByRole("button", { name: "Beta FC" });
    open.focus();
    expect(open).toHaveFocus();
    fireEvent.click(open);

    expect(onSelectTeam).toHaveBeenCalledWith("beta");
  });
});

/** The league's table: the clubs down its rows in the order drawn, read from each row's open button. */
const tableClubs = () =>
  within(screen.getByRole("table", { name: "League" }))
    .getAllByRole("row")
    .slice(1)
    .map((row) => within(row).getAllByRole("button")[0].getAttribute("aria-label") ?? "");

/** A row's cells, read by the header above each. */
function rowCells(club: string): Record<string, string> {
  const table = screen.getByRole("table", { name: "League" });
  const headers = within(table)
    .getAllByRole("columnheader")
    .map((header) => header.getAttribute("aria-label") ?? header.textContent?.trim() ?? "");
  const row = within(table)
    .getAllByRole("row")
    .find((candidate) => within(candidate).queryByRole("button", { name: club }));
  if (!row) throw new Error(`no row for ${club}`);
  const cells = within(row).getAllByRole("cell");
  return Object.fromEntries(
    headers.map((header, index) => [header, cells[index].textContent ?? ""]),
  );
}

/** Three clubs in one league: a leader, the manager's club second, and a club he doesn't know. */
function leagueOfThree() {
  return [
    buildCard({
      id: "leader",
      name: "Leader FC",
      league_pos: 1,
      points: 9,
      avg_ovr: 64,
      reputation: 40,
    }),
    buildCard({
      id: "mine",
      name: "Mine FC",
      league_pos: 2,
      points: 6,
      avg_ovr: 71,
      reputation: 60,
    }),
    buildCard({
      id: "unknown",
      name: "Unknown FC",
      league_pos: 3,
      points: 3,
      avg_ovr: null,
      reputation: 80,
    }),
  ];
}

describe("TeamsListTab as a table", () => {
  beforeEach(() => {
    mockedInvoke.mockReset();
    setListView("table");
  });

  /**
   * Given the list view is Table (the default) and the manager's league holds three clubs
   * When the clubs list opens
   * Then the league is a table named for the league, its clubs in league order, with the columns
   *      #, Team, P, Pts, Avg OVR, Rep, Squad and Value: no wage bill or balance, since a rival's
   *      finances are hidden.
   */
  it("lists a league's clubs as a table, in league order, without their finances", async () => {
    setupDirectoryMock(leagueOfThree());

    render(<TeamsListTab gameState={gameStateWithManagerTeam("mine")} onSelectTeam={vi.fn()} />);

    await screen.findByRole("table", { name: "League" });
    expect(tableClubs()).toEqual(["Leader FC", "Mine FC", "Unknown FC"]);
    expect(Object.keys(rowCells("Leader FC"))).toEqual([
      "Position",
      "Team",
      "P",
      "Pts",
      "Avg OVR",
      "Rep",
      "Squad",
      "Value",
    ]);
    expect(rowCells("Leader FC")).toMatchObject({
      Position: "1",
      P: "1",
      Pts: "9",
      "Avg OVR": "64",
      Rep: "40",
      Squad: "22",
    });
    expect(screen.queryByRole("heading", { level: 3 })).toBeNull();
  });

  /**
   * Given the manager's own club in the table
   * When it is drawn
   * Then its row says "Your Team", and no other row does.
   */
  it("marks the manager's own club in the table", async () => {
    setupDirectoryMock(leagueOfThree());

    render(<TeamsListTab gameState={gameStateWithManagerTeam("mine")} onSelectTeam={vi.fn()} />);

    await screen.findByRole("table", { name: "League" });
    expect(rowCells("Mine FC").Team).toContain("Your Team");
    expect(rowCells("Leader FC").Team).not.toContain("Your Team");
  });

  /**
   * Given a club outside what the manager knows, whose average the backend withholds
   * When the table is sorted by Avg OVR, high to low and then low to high
   * Then that club shows "–" and comes last both times, under the clubs whose averages are known.
   */
  it("shows a withheld average as a dash and sorts it last both ways", async () => {
    setupDirectoryMock(leagueOfThree());
    render(<TeamsListTab gameState={gameStateWithManagerTeam("mine")} onSelectTeam={vi.fn()} />);
    await screen.findByRole("table", { name: "League" });
    expect(rowCells("Unknown FC")["Avg OVR"]).toBe("–");

    fireEvent.click(screen.getByRole("button", { name: "Avg OVR" }));
    expect(tableClubs()).toEqual(["Mine FC", "Leader FC", "Unknown FC"]);

    fireEvent.click(screen.getByRole("button", { name: "Avg OVR" }));
    expect(tableClubs()).toEqual(["Leader FC", "Mine FC", "Unknown FC"]);
  });

  /**
   * Given the table in league order
   * When the Team header is pressed, and then the Rep header
   * Then the clubs run A to Z by name first, and by reputation high to low next: each column
   *      starts in its own direction.
   */
  it("sorts by the header pressed, each column from its own starting direction", async () => {
    setupDirectoryMock([
      buildCard({ id: "b", name: "Beta FC", league_pos: 1, reputation: 30 }),
      buildCard({ id: "c", name: "Gamma FC", league_pos: 2, reputation: 90 }),
      buildCard({ id: "a", name: "Alpha FC", league_pos: 3, reputation: 60 }),
    ]);
    render(<TeamsListTab gameState={gameStateWithManagerTeam("b")} onSelectTeam={vi.fn()} />);
    await screen.findByRole("table", { name: "League" });

    fireEvent.click(screen.getByRole("button", { name: "Team" }));
    expect(tableClubs()).toEqual(["Alpha FC", "Beta FC", "Gamma FC"]);

    fireEvent.click(screen.getByRole("button", { name: "Rep" }));
    expect(tableClubs()).toEqual(["Gamma FC", "Alpha FC", "Beta FC"]);
  });

  /**
   * Given a league whose table has two ranked clubs and one not yet ranked (position 0)
   * When the table opens in league order, and then the # header turns the order round
   * Then the unranked club reads "—" and comes last both times, under the ranked ones.
   */
  it("puts a club with no league position last in both directions", async () => {
    setupDirectoryMock([
      buildCard({ id: "first", name: "First FC", league_pos: 1 }),
      buildCard({ id: "second", name: "Second FC", league_pos: 2 }),
      buildCard({ id: "new", name: "New FC", league_pos: 0 }),
    ]);
    render(<TeamsListTab gameState={gameStateWithManagerTeam("first")} onSelectTeam={vi.fn()} />);
    await screen.findByRole("table", { name: "League" });
    expect(tableClubs()).toEqual(["First FC", "Second FC", "New FC"]);
    expect(rowCells("New FC").Position).toBe("—");

    fireEvent.click(screen.getByRole("button", { name: "Position" }));

    expect(tableClubs()).toEqual(["Second FC", "First FC", "New FC"]);
  });

  /**
   * Given two clubs, one of them in Leeds, whose name doesn't say so
   * When the search asks for "leeds"
   * Then the Leeds club's row is the one left, and it shows its city, so the reader can see why
   *      it matched.
   */
  it("shows a club's city in its row, so a match by city has its reason in view", async () => {
    setupDirectoryMock([
      buildCard({ id: "white", name: "Whites AFC", city: "Leeds", league_pos: 1 }),
      buildCard({ id: "other", name: "Other FC", city: "London", league_pos: 2 }),
    ]);
    render(<TeamsListTab gameState={gameStateWithManagerTeam("other")} onSelectTeam={vi.fn()} />);
    await screen.findByRole("table", { name: "League" });

    fireEvent.change(screen.getByPlaceholderText("Search clubs"), { target: { value: "leeds" } });

    await waitFor(() => expect(tableClubs()).toEqual(["Whites AFC"]));
    expect(rowCells("Whites AFC").Team).toContain("Leeds");
  });

  /**
   * Given the table
   * When a club's name is pressed (the row's button, which Enter presses too)
   * Then that club's profile opens.
   */
  it("opens a club from its row", async () => {
    const onSelectTeam = vi.fn();
    setupDirectoryMock(leagueOfThree());
    render(
      <TeamsListTab gameState={gameStateWithManagerTeam("mine")} onSelectTeam={onSelectTeam} />,
    );
    await screen.findByRole("table", { name: "League" });

    fireEvent.click(screen.getByRole("button", { name: "Leader FC" }));

    expect(onSelectTeam).toHaveBeenCalledWith("leader");
  });

  /**
   * Given the clubs list as a table
   * When Cards is picked on the view switch
   * Then the clubs are drawn as cards, and the choice is saved as the one list view every list
   *      opens with.
   */
  it("switches to cards from the view switch and saves the choice", async () => {
    setupDirectoryMock(leagueOfThree());
    render(<TeamsListTab gameState={gameStateWithManagerTeam("mine")} onSelectTeam={vi.fn()} />);
    await screen.findByRole("table", { name: "League" });

    await act(async () => {
      fireEvent.click(screen.getByRole("radio", { name: "Cards" }));
    });

    expect(useSettingsStore.getState().settings.list_view).toBe("cards");
    expect(screen.queryByRole("table")).toBeNull();
    expect(screen.getAllByRole("heading", { level: 3 })[0]).toHaveTextContent("Leader FC");
  });
});
