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

function renderController({ pick = true } = {}) {
  // Build the game state once so the hook receives a stable reference across
  // re-renders (the real store does the same); a fresh object each render would
  // retrigger the region/competition memos and loop the sync effects.
  const gameState = buildGameState();
  const externals = {
    setGameState: vi.fn(),
    setGameActive: vi.fn(),
    navigate: vi.fn(),
  };
  const controller = renderHook(() => useTeamSelection({ gameState, ...externals }));
  // Nothing is chosen on arrival; the scope tests are about the club the player picks.
  if (pick) act(() => controller.result.current.setSelectedTeamId("team-1"));
  return controller;
}

describe("useTeamSelection scope toggles", () => {
  beforeEach(() => {
    vi.clearAllMocks();
  });

  it("initializes the home region, region selection, and competition selection", () => {
    const { result } = renderController();

    expect(result.current.selectedHomeRegionId).toBe("europe");
    expect(result.current.regionSelection.europe).toBe(true);
    expect(result.current.selectedTeam?.id).toBe("team-1");
    expect(result.current.mandatoryCompetitionIds.has("epl")).toBe(true);
    expect(result.current.mandatoryCompetitionIds.has("intl")).toBe(true);
  });

  it("refuses to toggle the home region off", () => {
    const { result } = renderController();

    act(() => result.current.handleRegionToggle("europe"));

    expect(result.current.scopeMessage?.key).toBe(
      "teamSelect.scopeMessages.homeRegionAlwaysActive",
    );
    expect(result.current.regionSelection.europe).toBe(true);
  });

  it("enables a non-home region when toggled on", () => {
    const { result } = renderController();

    act(() => result.current.handleRegionToggle("asia"));

    expect(result.current.regionSelection.asia).toBe(true);
    expect(result.current.scopeMessage).toBeNull();
  });

  it("blocks disabling a region required by a mandatory competition", () => {
    const { result } = renderController();

    act(() => result.current.handleRegionToggle("south_america")); // enable
    act(() => result.current.handleRegionToggle("south_america")); // try disable

    expect(result.current.scopeMessage?.key).toBe(
      "teamSelect.scopeMessages.regionRequiredByCompetition",
    );
    expect(result.current.regionSelection.south_america).toBe(true);
  });

  it("disables stranded dependent competitions when a region is removed", () => {
    const { result } = renderController();

    act(() => result.current.handleRegionToggle("asia")); // enable, asia_cup stays on
    act(() => result.current.handleRegionToggle("asia")); // disable, strands asia_cup

    expect(result.current.competitionSelection.asia_cup).toBe(false);
    expect(result.current.scopeMessage?.key).toBe(
      "teamSelect.scopeMessages.regionRemovedDisablesCompetitions",
    );
  });

  it("auto-enables missing regions when enabling a competition", () => {
    const { result } = renderController();
    const asiaCup = () => result.current.availableCompetitions.find((c) => c.id === "asia_cup")!;

    act(() => result.current.handleCompetitionToggle(asiaCup())); // turn asia_cup off
    act(() => result.current.handleCompetitionToggle(asiaCup())); // turn back on -> needs asia

    expect(result.current.regionSelection.asia).toBe(true);
    expect(result.current.competitionSelection.asia_cup).toBe(true);
    expect(result.current.scopeMessage?.key).toBe("teamSelect.scopeMessages.autoEnabledRegions");
  });

  it("locks mandatory competitions against being disabled", () => {
    const { result } = renderController();
    const epl = result.current.availableCompetitions.find((c) => c.id === "epl")!;

    act(() => result.current.handleCompetitionToggle(epl));

    expect(result.current.scopeMessage?.key).toBe("teamSelect.scopeMessages.clubCompetitionLocked");
    expect(result.current.competitionSelection.epl).toBe(true);
  });

  it("disables an enabled non-locked competition and clears the message", () => {
    const { result } = renderController();
    const asiaCup = result.current.availableCompetitions.find((c) => c.id === "asia_cup")!;

    act(() => result.current.handleCompetitionToggle(asiaCup));

    expect(result.current.competitionSelection.asia_cup).toBe(false);
    expect(result.current.scopeMessage).toBeNull();
  });
});

describe("useTeamSelection home-region club filter", () => {
  /**
   * Given a club in a country that no competition places in a region
   * When team selection derives its regions
   * Then there is none and no home region is picked, so there is nothing to filter by and the club
   * is still listed: the frontend invents no region of its own, as it did when it guessed a club's
   * region from a table of country codes.
   */
  it("derives no region for a country no competition places, and still lists the club", () => {
    const gameState = {
      teams: [createTeam({ id: "team-gb", country: "GB", short_name: "ENG" })],
      players: [],
      competitions: [],
      league: null,
      manager: { first_name: "Sam", last_name: "Boss" },
    } as unknown as GameStateData;
    const externals = { setGameState: vi.fn(), setGameActive: vi.fn(), navigate: vi.fn() };
    const { result } = renderHook(() => useTeamSelection({ gameState, ...externals }));

    expect(result.current.selectedHomeRegionId).toBeNull();
    expect(result.current.visibleClubs.map((team) => team.id)).toEqual(["team-gb"]);
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

describe("useTeamSelection ratings", () => {
  /**
   * Given the backend rates team-1's squad before the career (p1 at 77, the club's average 73,
   *       which no player of the game state carries)
   * When the hook has its answer
   * Then the club's average, the player's rating and the likely eleven are the backend's, not
   *      figures read off the game state
   */
  it("reads the squads' ratings from the backend", async () => {
    vi.mocked(invoke).mockImplementation(async (command: string) =>
      command === "get_team_selection_ratings"
        ? {
            clubs: [
              { team_id: "team-1", average_rating: 73, players: [{ player_id: "p1", ovr: 77 }] },
            ],
          }
        : undefined,
    );

    const { result } = renderController();

    await waitFor(() => expect(result.current.getTeamAvgOvr("team-1")).toBe(73));
    expect(result.current.getPlayerRating("p1")).toBe(77);
    expect(result.current.selectedTeamXi.map((player) => player.id)).toEqual(["p1"]);
  });
});
