import { act, renderHook, waitFor } from "@testing-library/react";
import { invoke } from "@tauri-apps/api/core";
import { beforeEach, describe, expect, it, vi } from "vitest";

import type { GameStateData, LeagueData } from "../store/gameStore";
import { createPlayer, createTeam } from "../test-utils/factories";
import { useTeamSelection } from "./useTeamSelection";

vi.mock("@tauri-apps/api/core", () => ({ invoke: vi.fn() }));
vi.mock("../services/portraitService", () => ({
  prewarmManagerSquadPortraits: vi.fn(),
}));
// One `t` for every render, as react-i18next gives: a fresh function each time would make anything
// memoised on it (the derived regions) a new value on every render.
const translate = vi.hoisted(
  () => (key: string, params?: Record<string, string | number>) =>
    params?.defaultValue !== undefined ? String(params.defaultValue) : key,
);
const i18nState = vi.hoisted(() => ({ language: "en" }));

vi.mock("react-i18next", () => ({
  initReactI18next: { type: "3rdParty", init: () => {} },
  useTranslation: () => ({ t: translate, i18n: i18nState }),
}));

function league(overrides: Partial<LeagueData> = {}): LeagueData {
  return {
    id: "epl",
    name: "League",
    season: 2025,
    fixtures: [],
    standings: [],
    ...overrides,
  };
}

// Selected team (team-1) participates in epl (Domestic, requires home region
// "europe") and intl (Continental, requires non-home "south_america"); both are
// therefore mandatory. asia_cup is a non-mandatory Continental comp requiring
// "asia". This shape makes every toggle constraint branch reachable.
function buildGameState(): GameStateData {
  return {
    teams: [createTeam({ id: "team-1", country: "GB", short_name: "ALP" })],
    players: [createPlayer({ id: "p1", team_id: "team-1" })],
    competitions: [
      league({
        id: "epl",
        scope: "Domestic",
        kind: "League",
        region_id: "europe",
        country_id: "GB",
        participant_ids: ["team-1"],
        priority: 1,
      }),
      league({
        id: "intl",
        scope: "Continental",
        kind: "Cup",
        required_region_ids: ["south_america"],
        participant_ids: ["team-1"],
        priority: 2,
      }),
      league({
        id: "asia_cup",
        scope: "Continental",
        kind: "Cup",
        required_region_ids: ["asia"],
        participant_ids: ["team-9"],
        priority: 3,
      }),
    ],
    league: null,
    manager: { first_name: "Sam", last_name: "Boss" },
  } as unknown as GameStateData;
}

/** One club's facts as the backend gives them (`get_team_selection_ratings`), contesting `competitionIds`. */
function clubFacts(teamId: string, competitionIds: string[], overrides = {}) {
  return {
    team_id: teamId,
    average_rating: 73,
    players: [{ player_id: "p1", ovr: 77 }],
    stadium_name: "Ground",
    stadium_capacity: 20_000,
    transfer_budget: 5_000_000,
    weekly_wage_budget: 100_000,
    balance: 1_000_000,
    colors: { primary: "#112233", secondary: "#ffffff" },
    founded_year: 1900,
    competitions: competitionIds.map((id) => ({
      id,
      name: id,
      name_key: null,
      season: 2025,
      kind: "League",
      scope: "Domestic",
    })),
    key_players: [{ player_id: "p1", name: "J. Smith", position: "Striker", ovr: 77 }],
    reputation_key: "teamSelect.repAverage",
    opening_expectation: { target_position: 10, label_key: "teamSelect.expectations.midTable" },
    ...overrides,
  };
}

/** The backend, as the hook sees it: the club's facts, and a started career for `select_team`. */
function mockBackend(clubs = [clubFacts("team-1", ["epl", "intl"])]) {
  vi.mocked(invoke).mockImplementation(async (command: string) =>
    command === "get_team_selection_ratings"
      ? { clubs }
      : { manager: { display_name: "Sam Boss" } },
  );
}

async function renderController({ pick = true } = {}) {
  // Build the game state once so the hook receives a stable reference across
  // re-renders (the real store does the same); a fresh object each render would
  // retrigger the region/competition memos and loop the sync effects.
  const gameState = buildGameState();
  const externals = {
    setGameState: vi.fn(),
    setGameActive: vi.fn(),
    navigate: vi.fn(),
  };
  mockBackend();
  const controller = renderHook(() => useTeamSelection({ gameState, ...externals }));
  // The club's competitions are the backend's answer, so the scope waits for it.
  await waitFor(() => expect(controller.result.current.getClubFacts("team-1")).toBeDefined());
  // Nothing is chosen on arrival; the scope tests are about the club the player picks.
  if (pick) act(() => controller.result.current.setSelectedTeamId("team-1"));
  return controller;
}

describe("useTeamSelection simulation scope", () => {
  beforeEach(() => {
    vi.clearAllMocks();
  });

  /**
   * Given a world with a home league, a cup that needs a second region and another that needs a
   *       third, and nobody has touched the simulation settings
   * When the screen opens, with and without a club chosen
   * Then the career simulates what it always has: every competition, and every region they need.
   *      The redesign changes the screen, not what an untouched career simulates.
   */
  it("simulates every competition and the regions they need until the settings are touched", async () => {
    const { result } = await renderController({ pick: false });

    expect(result.current.selectedHomeRegionId).toBe("europe");
    expect(result.current.activeRegionIds).toEqual(["asia", "europe", "south_america"]);
    expect(result.current.enabledCompetitionIds).toEqual(["epl", "intl", "asia_cup"]);

    act(() => result.current.setSelectedTeamId("team-1"));
    expect(result.current.activeRegionIds).toEqual(["asia", "europe", "south_america"]);
    expect(result.current.enabledCompetitionIds).toEqual(["epl", "intl", "asia_cup"]);
  });

  /**
   * Given the dialog is applied with only the locked regions on, then with the third region on, then
   *       with that cup unchosen
   * When the competitions to simulate are worked out each time
   * Then a competition whose region is off is blocked and not simulated; with its region on it is
   *      simulated again; and once the player has unchosen it, it is not.
   */
  it("applies the dialog's draft: a region off blocks its competition, on brings it back", async () => {
    const { result } = await renderController();

    act(() => result.current.applyScope({ ...result.current.scopeDraft, regions: {} }));
    expect(result.current.activeRegionIds).toEqual(["europe", "south_america"]);
    expect(result.current.enabledCompetitionIds).toEqual(["epl", "intl"]);

    act(() => result.current.applyScope({ ...result.current.scopeDraft, regions: { asia: true } }));
    expect(result.current.activeRegionIds).toEqual(["asia", "europe", "south_america"]);
    expect(result.current.enabledCompetitionIds).toEqual(["epl", "intl", "asia_cup"]);

    act(() =>
      result.current.applyScope({
        ...result.current.scopeDraft,
        competitions: { ...result.current.scopeDraft.competitions, asia_cup: false },
      }),
    );
    expect(result.current.enabledCompetitionIds).toEqual(["epl", "intl"]);
  });

  /**
   * Given a club chosen and the settings untouched
   * When the career is started
   * Then `select_team` is sent every competition and every region they need, which is what the old
   *      flow's career simulated (it sent every competition and the game widened its home-only
   *      region list to the regions they need).
   */
  it("sends select_team the full default set when the settings are untouched", async () => {
    const { result } = await renderController();

    await act(async () => {
      await result.current.handleConfirm();
    });

    expect(invoke).toHaveBeenCalledWith("select_team", {
      teamId: "team-1",
      activeRegionIds: ["asia", "europe", "south_america"],
      activeCompetitionIds: ["epl", "intl", "asia_cup"],
    });
  });

  /**
   * Given a club chosen and the dialog applied with the third region off, so that cup is blocked
   * When the career is started
   * Then `select_team` is sent only what is simulated: the blocked cup is not among them. It is the
   *      one place a stale rule would reach the game.
   */
  it("sends select_team only the regions and competitions simulated after the player's changes", async () => {
    const { result } = await renderController();
    act(() => result.current.applyScope({ ...result.current.scopeDraft, regions: {} }));

    await act(async () => {
      await result.current.handleConfirm();
    });

    expect(invoke).toHaveBeenCalledWith("select_team", {
      teamId: "team-1",
      activeRegionIds: ["europe", "south_america"],
      activeCompetitionIds: ["epl", "intl"],
    });
  });
});

// Two domestic leagues in one country, and a club in each: the rail lists the leagues, the table
// shows one of them.
function buildTwoLeagueGameState(): GameStateData {
  const clubs = [
    createTeam({ id: "top-a", name: "Alpha FC", city: "Alton", country: "GB" }),
    createTeam({ id: "top-b", name: "Bravo Town", city: "Bexley", country: "GB" }),
    createTeam({ id: "low-c", name: "Charlie Rovers", city: "Carlow", country: "GB" }),
  ];
  return {
    teams: clubs,
    players: [],
    competitions: [
      league({
        id: "top",
        name: "Top Division",
        scope: "Domestic",
        kind: "League",
        region_id: "europe",
        country_id: "GB",
        participant_ids: ["top-a", "top-b"],
        priority: 1,
      }),
      league({
        id: "low",
        name: "Lower Division",
        scope: "Domestic",
        kind: "League",
        region_id: "europe",
        country_id: "GB",
        participant_ids: ["low-c"],
        priority: 2,
      }),
    ],
    league: null,
    manager: { first_name: "Sam", last_name: "Boss" },
  } as unknown as GameStateData;
}

function renderTwoLeagues() {
  const gameState = buildTwoLeagueGameState();
  const externals = { setGameState: vi.fn(), setGameActive: vi.fn(), navigate: vi.fn() };
  return renderHook(() => useTeamSelection({ gameState, ...externals }));
}

// A club in each of two regions: a European one and a South American one.
function buildTwoRegionGameState(): GameStateData {
  return {
    teams: [
      createTeam({ id: "gb-a", name: "Alpha FC", country: "GB" }),
      createTeam({ id: "br-a", name: "Brasa SC", country: "BR" }),
    ],
    players: [],
    competitions: [
      league({
        id: "gb1",
        scope: "Domestic",
        kind: "League",
        region_id: "europe",
        country_id: "GB",
        participant_ids: ["gb-a"],
        priority: 1,
      }),
      league({
        id: "br1",
        scope: "Domestic",
        kind: "League",
        region_id: "south_america",
        country_id: "BR",
        participant_ids: ["br-a"],
        priority: 1,
      }),
    ],
    league: null,
    manager: { first_name: "Sam", last_name: "Boss" },
  } as unknown as GameStateData;
}

describe("useTeamSelection countries", () => {
  /**
   * Given clubs in two regions, with the European one the home region on arrival
   * When the rail's countries are listed, and the South American country is picked
   * Then every country is listed, not only the home region's; and picking one makes its region the
   *      home region and shows its clubs, so a club anywhere is a pick away, not behind the
   *      simulation settings.
   */
  it("lists every country, and picking one makes its region the home one", () => {
    const gameState = buildTwoRegionGameState();
    const externals = { setGameState: vi.fn(), setGameActive: vi.fn(), navigate: vi.fn() };
    const { result } = renderHook(() => useTeamSelection({ gameState, ...externals }));

    expect([...result.current.allCountryCodes].sort()).toEqual(["BR", "GB"]);
    const homeBefore = result.current.selectedHomeRegionId;

    act(() => result.current.handleSelectCountry("BR"));

    expect(result.current.selectedHomeRegionId).toBe("south_america");
    expect(result.current.selectedHomeRegionId).not.toBe(homeBefore);
    expect(result.current.selectedCountryCode).toBe("BR");
    expect(result.current.visibleClubs.map((team) => team.id)).toEqual(["br-a"]);
  });

  /**
   * Given settings nobody has touched, in a world with a competition in each of two regions
   * When the South American country is picked
   * Then both regions are still simulated, as the career always has: nothing is dropped because the
   *      home region moved.
   */
  it("changes nothing a career simulates when the home region moves and the settings are untouched", () => {
    const gameState = buildTwoRegionGameState();
    const externals = { setGameState: vi.fn(), setGameActive: vi.fn(), navigate: vi.fn() };
    const { result } = renderHook(() => useTeamSelection({ gameState, ...externals }));
    expect(result.current.activeRegionIds).toEqual(["europe", "south_america"]);

    act(() => result.current.handleSelectCountry("BR"));

    expect(result.current.activeRegionIds).toEqual(["europe", "south_america"]);
    expect(result.current.enabledCompetitionIds).toEqual(["gb1", "br1"]);
  });

  /**
   * Given a player who applied the settings with only the home region on, then browsed to a South
   *       American country
   * When the home region moves
   * Then the European region is no longer simulated, and its competition is blocked: the home
   *      region follows the country, and a region that was only home is not kept.
   */
  it("stops simulating the region it leaves once the player has chosen their regions", () => {
    const gameState = buildTwoRegionGameState();
    const externals = { setGameState: vi.fn(), setGameActive: vi.fn(), navigate: vi.fn() };
    const { result } = renderHook(() => useTeamSelection({ gameState, ...externals }));
    act(() => result.current.applyScope({ ...result.current.scopeDraft, regions: {} }));
    expect(result.current.activeRegionIds).toEqual(["europe"]);

    act(() => result.current.handleSelectCountry("BR"));

    expect(result.current.activeRegionIds).toEqual(["south_america"]);
    expect(result.current.enabledCompetitionIds).toEqual(["br1"]);
  });

  /**
   * Given a player who chose the South American region in the settings, and then went to a South
   *       American country and back to a European one
   * When the home region moves away from South America
   * Then South America stays simulated: the player chose it, and only a region that was just home
   *      is dropped.
   */
  it("keeps simulating a region the player chose themselves", () => {
    const gameState = buildTwoRegionGameState();
    const externals = { setGameState: vi.fn(), setGameActive: vi.fn(), navigate: vi.fn() };
    const { result } = renderHook(() => useTeamSelection({ gameState, ...externals }));

    act(() =>
      result.current.applyScope({ ...result.current.scopeDraft, regions: { south_america: true } }),
    );
    act(() => result.current.handleSelectCountry("BR"));
    act(() => result.current.handleSelectCountry("GB"));

    expect(result.current.activeRegionIds).toEqual(["europe", "south_america"]);
  });
});

describe("useTeamSelection club table", () => {
  /**
   * Given a game with clubs and a league to show
   * When the screen opens
   * Then no club is chosen: the player picks one, and the first of the list is not picked for them.
   */
  it("chooses no club on arrival", () => {
    const { result } = renderTwoLeagues();

    expect(result.current.visibleClubs.length).toBeGreaterThan(0);
    expect(result.current.selectedTeam).toBeNull();
  });

  /**
   * Given two leagues in the home country, the stronger first
   * When the screen opens, and then the lower league is picked in the rail
   * Then the rail lists both with their clubs counted, and the table shows the stronger league's
   *      clubs until the lower is picked, and that league's after.
   */
  it("lists the country's leagues and shows the picked league's clubs", () => {
    const { result } = renderTwoLeagues();

    expect(result.current.leagueGroups.map((group) => [group.id, group.teams.length])).toEqual([
      ["top", 2],
      ["low", 1],
    ]);
    expect(result.current.activeLeagueId).toBe("top");
    expect(result.current.visibleClubs.map((team) => team.id)).toEqual(["top-a", "top-b"]);

    act(() => result.current.setSelectedLeagueId("low"));

    expect(result.current.activeLeagueId).toBe("low");
    expect(result.current.visibleClubs.map((team) => team.id)).toEqual(["low-c"]);
  });

  /**
   * Given a club chosen in the top league
   * When the lower league is picked in the rail
   * Then the choice is dropped: Manage would otherwise name a club that is no longer in view.
   */
  it("drops the chosen club when its league leaves the table", () => {
    const { result } = renderTwoLeagues();
    act(() => result.current.setSelectedTeamId("top-a"));
    expect(result.current.selectedTeam?.id).toBe("top-a");

    act(() => result.current.setSelectedLeagueId("low"));

    expect(result.current.selectedTeam).toBeNull();
  });

  /**
   * Given a search typed over the top league
   * When it matches a club's city and not the others
   * Then only that club is in the table, while the rail still counts the whole league.
   */
  it("narrows the table by name or city, and leaves the rail's counts alone", () => {
    const { result } = renderTwoLeagues();

    act(() => result.current.setClubSearch("bexley"));

    expect(result.current.visibleClubs.map((team) => team.id)).toEqual(["top-b"]);
    expect(result.current.leagueGroups[0].teams).toHaveLength(2);
  });
});

describe("useTeamSelection: Manage does not wait for the club's facts", () => {
  /**
   * Given the backend's club facts are still pending, or the request failed
   * When a club is chosen and the career is started
   * Then `select_team` is sent the full default set all the same: Manage never waits on the facts, so
   *      a failed ask cannot block a career, and the game adds the club's own competitions itself.
   */
  it.each([
    ["pending", () => new Promise(() => {})],
    ["failed", () => Promise.reject(new Error("the ratings could not be read"))],
  ])("starts the career with the default scope while the facts are %s", async (_state, ratings) => {
    vi.mocked(invoke).mockImplementation(async (command: string) =>
      command === "get_team_selection_ratings"
        ? ratings()
        : { manager: { display_name: "Sam Boss" } },
    );
    const gameState = buildGameState();
    const externals = { setGameState: vi.fn(), setGameActive: vi.fn(), navigate: vi.fn() };
    const { result } = renderHook(() => useTeamSelection({ gameState, ...externals }));
    act(() => result.current.setSelectedTeamId("team-1"));
    expect(result.current.selectedClubFacts).toBeUndefined();

    await act(async () => {
      await result.current.handleConfirm();
    });

    expect(invoke).toHaveBeenCalledWith("select_team", {
      teamId: "team-1",
      activeRegionIds: ["asia", "europe", "south_america"],
      activeCompetitionIds: ["epl", "intl", "asia_cup"],
    });
  });
});

describe("useTeamSelection ratings", () => {
  /**
   * Given the backend rates team-1's squad before the career (the club's average 73, a key player
   *       at 77, which no player of the game state carries) and names its reputation and the board's
   *       opening expectation
   * When the hook has its answer
   * Then the club's average and its facts are the backend's, not figures read off the game state:
   *      the reputation word and the expectation are the translation keys it names
   */
  it("reads the clubs' facts from the backend", async () => {
    const { result } = await renderController();

    expect(result.current.getTeamAvgOvr("team-1")).toBe(73);
    expect(result.current.selectedClubFacts).toMatchObject({
      reputation_key: "teamSelect.repAverage",
      opening_expectation: { label_key: "teamSelect.expectations.midTable" },
    });
    expect(result.current.selectedClubFacts?.key_players.map((player) => player.player_id)).toEqual(
      ["p1"],
    );
  });

  /**
   * Given the backend says the club contests only its league
   * When the club is chosen
   * Then the club's own competitions, which are always simulated, are the backend's list, not one
   *      worked out here from the fixtures.
   */
  it("takes the club's competitions from the backend's answer", async () => {
    mockBackend([clubFacts("team-1", ["epl"])]);
    const gameState = buildGameState();
    const externals = { setGameState: vi.fn(), setGameActive: vi.fn(), navigate: vi.fn() };
    const { result } = renderHook(() => useTeamSelection({ gameState, ...externals }));
    await waitFor(() => expect(result.current.getClubFacts("team-1")).toBeDefined());

    act(() => result.current.setSelectedTeamId("team-1"));

    expect(result.current.selectedTeamCompetitions.map((competition) => competition.id)).toEqual([
      "epl",
    ]);
  });
});
