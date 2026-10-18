import { describe, expect, it } from "vitest";
import {
  createDashboardProfileNavigationState,
  goBackDashboardProfile,
  hasDashboardProfileHistory,
  navigateDashboardProfiles,
  openDashboardSearchPlayer,
  openDashboardSearchTeam,
  openProfileTitle,
  resetDashboardToTab,
  selectDashboardPlayer,
  selectDashboardTeam,
} from "./dashboardProfileNavigation";

describe("dashboardProfileNavigation", () => {
  it("resets to a tab and clears selected profiles and history", () => {
    const state = {
      ...createDashboardProfileNavigationState("Inbox"),
      selectedPlayerId: "player-1",
      selectedTeamId: "team-1",
      navHistory: [{ tab: "Home", playerId: null, teamId: null }],
    };

    expect(resetDashboardToTab(state, "Squad", "message-1")).toEqual({
      activeTab: "Squad",
      selectedPlayerId: null,
      selectedPlayerOptions: null,
      selectedTeamId: null,
      initialMessageId: "message-1",
      navHistory: [],
    });
  });

  it("navigates to a selected player while preserving history", () => {
    const next = navigateDashboardProfiles(
      createDashboardProfileNavigationState("Home"),
      "__selectPlayer",
      { messageId: "player-7" },
    );

    expect(next.selectedPlayerId).toBe("player-7");
    expect(next.selectedTeamId).toBeNull();
    expect(next.navHistory).toEqual([{ tab: "Home", playerId: null, teamId: null }]);
  });

  it("restores the previous entry when navigating back", () => {
    const selectedPlayer = selectDashboardPlayer(
      createDashboardProfileNavigationState("Squad"),
      "player-2",
      { openRenewal: true },
    );
    const selectedTeam = selectDashboardTeam(selectedPlayer, "team-9");

    const previous = goBackDashboardProfile(selectedTeam);

    expect(previous.activeTab).toBe("Squad");
    expect(previous.selectedPlayerId).toBe("player-2");
    expect(previous.selectedPlayerOptions).toBeNull();
    expect(previous.selectedTeamId).toBeNull();
    expect(previous.navHistory).toHaveLength(1);
  });

  it("tracks whether a profile or history stack is active", () => {
    const base = createDashboardProfileNavigationState("Home");

    expect(hasDashboardProfileHistory(base)).toBe(false);
    expect(hasDashboardProfileHistory(selectDashboardTeam(base, "team-1"))).toBe(true);
  });

  it("replaces the current profile selection when opening a search result", () => {
    const withTeamOpen = selectDashboardTeam(
      createDashboardProfileNavigationState("Home"),
      "team-1",
    );

    const playerSearchState = openDashboardSearchPlayer(withTeamOpen, "player-5");
    expect(playerSearchState.selectedPlayerId).toBe("player-5");
    expect(playerSearchState.selectedTeamId).toBeNull();

    const teamSearchState = openDashboardSearchTeam(playerSearchState, "team-9");
    expect(teamSearchState.selectedTeamId).toBe("team-9");
    expect(teamSearchState.selectedPlayerId).toBeNull();
  });

  /**
   * Given a profile that is open
   * When another profile is opened from search, and Back is pressed
   * Then the first profile is shown again, instead of being lost
   */
  it("keeps the open profile in Back when another is opened from search", () => {
    const withPlayerOpen = selectDashboardPlayer(
      createDashboardProfileNavigationState("Squad"),
      "player-1",
    );

    const fromSearch = openDashboardSearchPlayer(withPlayerOpen, "player-2");
    const back = goBackDashboardProfile(fromSearch);

    expect(fromSearch.selectedPlayerId).toBe("player-2");
    expect(back.selectedPlayerId).toBe("player-1");

    const teamFromSearch = openDashboardSearchTeam(withPlayerOpen, "team-9");
    expect(goBackDashboardProfile(teamFromSearch).selectedPlayerId).toBe("player-1");
  });

  const world = {
    players: [{ id: "player-1", match_name: "J. Smith" }],
    teams: [{ id: "team-2", name: "Beta FC" }],
  };

  /**
   * Given a player's profile opened from the Squad tab
   * When the header asks what is open
   * Then it gets the player's name, not the tab's
   */
  it("names an open player profile by the player", () => {
    const state = selectDashboardPlayer(createDashboardProfileNavigationState("Squad"), "player-1");

    expect(openProfileTitle(state, world)).toBe("J. Smith");
  });

  /**
   * Given a club's profile opened from a player's profile
   * When the header asks what is open
   * Then it gets the club, which is what the screen shows
   */
  it("names an open club profile by the club, even with a player behind it", () => {
    const withPlayer = selectDashboardPlayer(
      createDashboardProfileNavigationState("Squad"),
      "player-1",
    );
    const state = { ...selectDashboardTeam(withPlayer, "team-2"), selectedPlayerId: "player-1" };

    expect(openProfileTitle(state, world)).toBe("Beta FC");
  });

  /**
   * Given a profile whose player is no longer in the world (a retired player)
   * When the header asks what is open
   * Then there is no name to give
   */
  it("has no name for a profile whose player is not in the world", () => {
    const state = selectDashboardPlayer(
      createDashboardProfileNavigationState("Squad"),
      "retired-9",
    );

    expect(openProfileTitle(state, world)).toBeNull();
  });

  /**
   * Given no profile is open
   * When the header asks what is open
   * Then nothing is, and the tab names the screen
   */
  it("has no name when no profile is open", () => {
    expect(openProfileTitle(createDashboardProfileNavigationState("Squad"), world)).toBeNull();
  });
});
