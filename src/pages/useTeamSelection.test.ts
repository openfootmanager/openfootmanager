import { act, renderHook } from "@testing-library/react";
import { beforeEach, describe, expect, it, vi } from "vitest";

import type { GameStateData, LeagueData } from "../store/gameStore";
import { createPlayer, createTeam } from "../test-utils/factories";
import { useTeamSelection } from "./useTeamSelection";

vi.mock("@tauri-apps/api/core", () => ({ invoke: vi.fn() }));
vi.mock("../services/portraitService", () => ({
  prewarmManagerSquadPortraits: vi.fn(),
}));
vi.mock("react-i18next", () => ({
  initReactI18next: { type: "3rdParty", init: () => {} },
  useTranslation: () => ({
    t: (key: string, params?: Record<string, string | number>) =>
      params?.defaultValue !== undefined ? String(params.defaultValue) : key,
    i18n: { language: "en" },
  }),
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
    regions: [
      { id: "europe", name: "Europe", country_codes: ["GB"] },
      { id: "south_america", name: "South America", country_codes: ["BR"] },
      { id: "asia", name: "Asia", country_codes: ["JP"] },
    ],
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

function renderController() {
  // Build the game state once so the hook receives a stable reference across
  // re-renders (the real store does the same); a fresh object each render would
  // retrigger the region/competition memos and loop the sync effects.
  const gameState = buildGameState();
  const externals = {
    setGameState: vi.fn(),
    setGameActive: vi.fn(),
    navigate: vi.fn(),
  };
  return renderHook(() => useTeamSelection({ gameState, ...externals }));
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
    const asiaCup = () =>
      result.current.availableCompetitions.find((c) => c.id === "asia_cup")!;

    act(() => result.current.handleCompetitionToggle(asiaCup())); // turn asia_cup off
    act(() => result.current.handleCompetitionToggle(asiaCup())); // turn back on -> needs asia

    expect(result.current.regionSelection.asia).toBe(true);
    expect(result.current.competitionSelection.asia_cup).toBe(true);
    expect(result.current.scopeMessage?.key).toBe(
      "teamSelect.scopeMessages.autoEnabledRegions",
    );
  });

  it("locks mandatory competitions against being disabled", () => {
    const { result } = renderController();
    const epl = result.current.availableCompetitions.find((c) => c.id === "epl")!;

    act(() => result.current.handleCompetitionToggle(epl));

    expect(result.current.scopeMessage?.key).toBe(
      "teamSelect.scopeMessages.clubCompetitionLocked",
    );
    expect(result.current.competitionSelection.epl).toBe(true);
  });

  it("disables an enabled non-locked competition and clears the message", () => {
    const { result } = renderController();
    const asiaCup = result.current.availableCompetitions.find(
      (c) => c.id === "asia_cup",
    )!;

    act(() => result.current.handleCompetitionToggle(asiaCup));

    expect(result.current.competitionSelection.asia_cup).toBe(false);
    expect(result.current.scopeMessage).toBeNull();
  });
});
