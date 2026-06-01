import { create } from 'zustand';
import type { GameStateData } from './types';

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
  isDirty: boolean;
  showFiredModal: boolean;
  setGameActive: (active: boolean, managerName?: string) => void;
  setGameState: (state: GameStateData) => void;
  markClean: () => void;
  setShowFiredModal: (show: boolean) => void;
  clearGame: () => void;
}

export const useGameStore = create<GameStore>((set) => ({
  hasActiveGame: false,
  managerName: null,
  gameState: null,
  isDirty: false,
  showFiredModal: false,
  setGameActive: (active, managerName) => set({
    hasActiveGame: active,
    managerName: managerName || null
  }),
  setGameState: (state) => set({
    gameState: normalizeGameStateNationalities(state),
    isDirty: true,
  }),
  markClean: () => set({ isDirty: false }),
  setShowFiredModal: (show) => set({ showFiredModal: show }),
  clearGame: () => set({
    hasActiveGame: false,
    managerName: null,
    gameState: null,
    isDirty: false,
    showFiredModal: false,
  }),
}));
