import { create } from 'zustand';
import type { GameStateData, SeasonContextData } from './types';
import type { SessionState, UserCompetitionSummary, StandingRow } from '../services/sessionService';

type FootballIdentityCarrier = {
  nationality: string;
  football_nation?: string | null;
};

function normalizeNationality<T extends FootballIdentityCarrier>(entity: T): T {
  const footballNation = entity.football_nation?.trim();
  if (!footballNation || footballNation === entity.nationality) {
    return entity;
  }

  return {
    ...entity,
    nationality: footballNation,
  };
}

function normalizeNationalityList<T extends FootballIdentityCarrier>(entities: T[]): T[] {
  let changed = false;
  const normalized = entities.map((entity) => {
    const next = normalizeNationality(entity);
    changed ||= next !== entity;
    return next;
  });

  return changed ? normalized : entities;
}

function normalizeGameStateNationalities(state: GameStateData): GameStateData {
  const manager = normalizeNationality(state.manager);
  const managers = state.managers
    ? normalizeNationalityList(state.managers)
    : state.managers;
  const players = normalizeNationalityList(state.players);
  const staff = normalizeNationalityList(state.staff);

  if (
    manager === state.manager
    && managers === state.managers
    && players === state.players
    && staff === state.staff
  ) {
    return state;
  }

  return {
    ...state,
    manager,
    managers,
    players,
    staff,
  };
}

const DEFAULT_SEASON_CONTEXT: SeasonContextData = {
  phase: 'Preseason',
  season_start: null,
  season_end: null,
  days_until_season_start: null,
  transfer_window: {
    status: 'Closed',
    opens_on: null,
    closes_on: null,
    days_until_opens: null,
    days_remaining: null,
  },
};

function deriveSessionState(state: GameStateData): SessionState {
  const teamId = state.manager.team_id;
  const team = teamId
    ? (state.teams.find((t) => t.id === teamId) ?? null)
    : null;

  const userComp = teamId
    ? (state.competitions ?? []).find((c) =>
        (c.participant_ids ?? []).includes(teamId)
      )
    : undefined;

  let user_competition: UserCompetitionSummary | null = null;
  if (userComp && teamId) {
    const teamNameMap = new Map(state.teams.map((t) => [t.id, t.name]));
    const standings: StandingRow[] = (userComp.standings ?? []).map((s) => ({
      ...s,
      team_name: teamNameMap.get(s.team_id) ?? '',
    }));
    const allFixtures = userComp.fixtures ?? [];
    const upcoming = allFixtures
      .filter(
        (f) =>
          f.status === 'Scheduled' &&
          (f.home_team_id === teamId || f.away_team_id === teamId),
      )
      .sort((a, b) => a.date.localeCompare(b.date))
      .slice(0, 3);
    const recent = allFixtures
      .filter(
        (f) =>
          f.status === 'Completed' &&
          (f.home_team_id === teamId || f.away_team_id === teamId),
      )
      .sort((a, b) => b.date.localeCompare(a.date))
      .slice(0, 2);

    user_competition = {
      competition_id: userComp.id,
      competition_name: userComp.name,
      name_key: userComp.name_key ?? null,
      standings,
      upcoming_fixtures: upcoming,
      recent_fixtures: recent,
    };
  }

  return {
    clock: state.clock,
    manager: state.manager,
    team,
    season_context: state.season_context ?? DEFAULT_SEASON_CONTEXT,
    board_objectives: state.board_objectives ?? [],
    scouting_assignments: state.scouting_assignments ?? [],
    youth_scouting_assignments: state.youth_scouting_assignments ?? [],
    active_competition_ids: state.active_competition_ids ?? [],
    unread_news_count: (state.news ?? []).filter((a) => !a.read).length,
    unread_messages_count: (state.messages ?? []).filter((m) => !m.read).length,
    user_competition,
  };
}

// Re-export all types so existing imports from gameStore keep working
export type {
  TeamColors,
  TeamSeasonRecord,
  TeamMatchRolesData,
  TeamData,
  PlayerSeasonStats,
  CareerEntry,
  ContractExitIntentData,
  ContractRenewalStateData,
  PlayerMoraleCoreData,
  PlayerData,
  TransferOfferData,
  StaffData,
  MessageAction,
  MessageActionOption,
  MessageContext,
  DelegatedRenewalCaseMessageData,
  DelegatedRenewalReportMessageData,
  PlayerSelectionOptions,
  ScoutReportData,
  MessageData,
  ManagerCareerStats,
  ManagerCareerEntry,
  ManagerData,
  CompletedTransferData,
  TransferRumourData,
  FixtureData,
  StandingData,
  LeagueData,
  NationalTeamData,
  WorldRegionData,
  SeasonPhase,
  TransferWindowStatus,
  TransferWindowContextData,
  SeasonContextData,
  SeasonAwardEntryData,
  SeasonManagerAwardEntryData,
  SeasonAwardsData,
  NewsMatchScore,
  NewsArticle,
  BoardObjective,
  ScoutingAssignment,
  YouthScoutingAssignment,
  GameStateData,
} from './types';

interface GameStore {
  hasActiveGame: boolean;
  managerName: string | null;
  gameState: GameStateData | null;
  sessionState: SessionState | null;
  isDirty: boolean;
  showFiredModal: boolean;
  setGameActive: (active: boolean, managerName?: string) => void;
  setGameState: (state: GameStateData) => void;
  setMessages: (messages: MessageData[]) => void;
  setSessionState: (state: SessionState) => void;
  markClean: () => void;
  setShowFiredModal: (show: boolean) => void;
  clearGame: () => void;
}

export const useGameStore = create<GameStore>((set, get) => ({
  hasActiveGame: false,
  managerName: null,
  gameState: null,
  sessionState: null,
  isDirty: false,
  showFiredModal: false,
  setGameActive: (active, managerName) => set({
    hasActiveGame: active,
    managerName: managerName || null
  }),
  setGameState: (state) => {
    const normalized = normalizeGameStateNationalities(state);
    set({ gameState: normalized, sessionState: deriveSessionState(normalized), isDirty: true });
  },
  // Lightweight patch for inbox-only mutations that return just the message
  // slice (not the whole game). Patching messages here re-derives sessionState
  // so the sidebar unread badge updates immediately, without round-tripping the
  // entire world on every read/delete.
  setMessages: (messages) => {
    const current = get().gameState;
    if (!current) return;
    const next = { ...current, messages };
    set({ gameState: next, sessionState: deriveSessionState(next), isDirty: true });
  },
  setSessionState: (state) => set({ sessionState: state }),
  markClean: () => set({ isDirty: false }),
  setShowFiredModal: (show) => set({ showFiredModal: show }),
  clearGame: () => set({
    hasActiveGame: false,
    managerName: null,
    gameState: null,
    sessionState: null,
    isDirty: false,
    showFiredModal: false,
  }),
}));
