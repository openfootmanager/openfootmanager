import type { PlayerSelectionOptions } from "../../store/gameStore";

export interface DashboardNavigateContext {
  messageId?: string;
}

export interface DashboardProfileHistoryEntry {
  tab: string;
  playerId: string | null;
  teamId: string | null;
}

export interface DashboardProfileNavigationState {
  activeTab: string;
  selectedPlayerId: string | null;
  selectedPlayerOptions: PlayerSelectionOptions | null;
  selectedTeamId: string | null;
  initialMessageId: string | null;
  navHistory: DashboardProfileHistoryEntry[];
}

/** `selectedPlayerId` opens that player's profile at once: the dev page harness starts on one. */
export function createDashboardProfileNavigationState(
  activeTab: string,
  selectedPlayerId: string | null = null,
): DashboardProfileNavigationState {
  return {
    activeTab,
    selectedPlayerId,
    selectedPlayerOptions: null,
    selectedTeamId: null,
    initialMessageId: null,
    navHistory: [],
  };
}

export function clearDashboardProfileSelection(
  state: DashboardProfileNavigationState,
): DashboardProfileNavigationState {
  return {
    ...state,
    selectedPlayerId: null,
    selectedPlayerOptions: null,
    selectedTeamId: null,
  };
}

export function resetDashboardToTab(
  state: DashboardProfileNavigationState,
  tab: string,
  messageId?: string,
): DashboardProfileNavigationState {
  return {
    ...clearDashboardProfileSelection(state),
    activeTab: tab,
    initialMessageId: messageId ?? null,
    navHistory: [],
  };
}

export function pushDashboardHistory(
  state: DashboardProfileNavigationState,
): DashboardProfileNavigationState {
  return {
    ...state,
    navHistory: [
      ...state.navHistory,
      {
        tab: state.activeTab,
        playerId: state.selectedPlayerId,
        teamId: state.selectedTeamId,
      },
    ],
  };
}

export function navigateDashboardProfiles(
  state: DashboardProfileNavigationState,
  tab: string,
  context?: DashboardNavigateContext,
): DashboardProfileNavigationState {
  if (tab === "__selectTeam" && context?.messageId) {
    return {
      ...pushDashboardHistory(state),
      selectedTeamId: context.messageId,
      selectedPlayerId: null,
      selectedPlayerOptions: null,
    };
  }

  if (tab === "__selectPlayer" && context?.messageId) {
    return {
      ...pushDashboardHistory(state),
      selectedPlayerId: context.messageId,
      selectedPlayerOptions: null,
      selectedTeamId: null,
    };
  }

  return resetDashboardToTab(state, tab, context?.messageId);
}

export function goBackDashboardProfile(
  state: DashboardProfileNavigationState,
): DashboardProfileNavigationState {
  if (state.navHistory.length === 0) {
    return clearDashboardProfileSelection(state);
  }

  const previous = state.navHistory[state.navHistory.length - 1];

  return {
    ...state,
    activeTab: previous.tab,
    selectedPlayerId: previous.playerId,
    selectedPlayerOptions: null,
    selectedTeamId: previous.teamId,
    navHistory: state.navHistory.slice(0, -1),
  };
}

export function selectDashboardPlayer(
  state: DashboardProfileNavigationState,
  id: string,
  options?: PlayerSelectionOptions,
): DashboardProfileNavigationState {
  return {
    ...pushDashboardHistory(state),
    selectedPlayerId: id,
    selectedPlayerOptions: options ?? null,
    selectedTeamId: null,
  };
}

export function selectDashboardTeam(
  state: DashboardProfileNavigationState,
  id: string,
): DashboardProfileNavigationState {
  return {
    ...pushDashboardHistory(state),
    selectedTeamId: id,
    selectedPlayerId: null,
    selectedPlayerOptions: null,
  };
}

export function openDashboardSearchPlayer(
  state: DashboardProfileNavigationState,
  id: string,
): DashboardProfileNavigationState {
  return {
    ...pushDashboardHistory(state),
    selectedPlayerId: id,
    selectedPlayerOptions: null,
    selectedTeamId: null,
  };
}

export function openDashboardSearchTeam(
  state: DashboardProfileNavigationState,
  id: string,
): DashboardProfileNavigationState {
  return {
    ...pushDashboardHistory(state),
    selectedTeamId: id,
    selectedPlayerId: null,
    selectedPlayerOptions: null,
  };
}

export function hasDashboardProfileHistory(state: DashboardProfileNavigationState): boolean {
  return (
    state.navHistory.length > 0 || state.selectedPlayerId !== null || state.selectedTeamId !== null
  );
}

interface ProfileSubjects {
  players: ReadonlyArray<{ id: string; match_name: string }>;
  teams: ReadonlyArray<{ id: string; name: string }>;
}

// A club profile wins over a player one, as in DashboardWorkspaceContent, which shows the club
// when both are selected.
export function openProfileTitle(
  state: DashboardProfileNavigationState,
  world: ProfileSubjects,
  /** The retired player fetched for the open profile, who is in no roster: the archive's person. */
  retiree: { id: string; match_name: string } | null = null,
): string | null {
  if (state.selectedTeamId !== null) {
    return world.teams.find((team) => team.id === state.selectedTeamId)?.name ?? null;
  }
  if (state.selectedPlayerId !== null) {
    const rostered = world.players.find((player) => player.id === state.selectedPlayerId);
    if (rostered) return rostered.match_name;
    return retiree?.id === state.selectedPlayerId ? retiree.match_name : null;
  }
  return null;
}
