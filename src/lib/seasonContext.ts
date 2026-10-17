import type {
  GameStateData,
  SeasonContextData,
  TransferWindowContextData,
} from "../store/gameStore";

const DEFAULT_TRANSFER_WINDOW: TransferWindowContextData = {
  status: "Closed",
  opens_on: null,
  closes_on: null,
  days_until_opens: null,
  days_remaining: null,
};

const DEFAULT_SEASON_CONTEXT: SeasonContextData = {
  phase: "Preseason",
  season_complete: false,
  season_start: null,
  season_end: null,
  days_until_season_start: null,
  transfer_window: DEFAULT_TRANSFER_WINDOW,
};

export function resolveSeasonContext(gameState: GameStateData): SeasonContextData {
  if (gameState.season_context) {
    return normaliseSeasonContext(gameState.season_context);
  }

  // The backend always sends it; a state without one is not worked out from fixtures here.
  return DEFAULT_SEASON_CONTEXT;
}

/** Whether the season is over, as the backend decides it (the answer `advance_to_next_season` enforces). */
export function isSeasonComplete(gameState: GameStateData): boolean {
  return resolveSeasonContext(gameState).season_complete === true;
}

export function hasCompetitiveStandings(gameState: GameStateData): boolean {
  return resolveSeasonContext(gameState).phase !== "Preseason";
}

function normaliseSeasonContext(context: SeasonContextData): SeasonContextData {
  return {
    phase: context.phase,
    season_complete: context.season_complete ?? false,
    season_start: context.season_start ?? null,
    season_end: context.season_end ?? null,
    days_until_season_start: context.days_until_season_start ?? null,
    transfer_window: {
      status: context.transfer_window?.status ?? "Closed",
      opens_on: context.transfer_window?.opens_on ?? null,
      closes_on: context.transfer_window?.closes_on ?? null,
      days_until_opens: context.transfer_window?.days_until_opens ?? null,
      days_remaining: context.transfer_window?.days_remaining ?? null,
    },
  };
}
