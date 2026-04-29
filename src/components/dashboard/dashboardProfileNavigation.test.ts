import { describe, expect, it } from "vitest";
import {
  createDashboardProfileNavigationState,
  goBackDashboardProfile,
  hasDashboardProfileHistory,
  navigateDashboardProfiles,
  openDashboardSearchPlayer,
  openDashboardSearchTeam,
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
      preferredNation: null,
      preferredCompetitionId: null,
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
    expect(next.navHistory).toEqual([
      { tab: "Home", playerId: null, teamId: null },
    ]);
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
    expect(hasDashboardProfileHistory(selectDashboardTeam(base, "team-1"))).toBe(
      true,
    );
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
});