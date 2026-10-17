import type { GameStateData } from "../store/gameStore";

/** Whether the season is over, as the backend decides it (the answer `advance_to_next_season` enforces). */
export function isSeasonComplete(gameState: GameStateData): boolean {
  return gameState.season_context.season_complete;
}

export function hasCompetitiveStandings(gameState: GameStateData): boolean {
  return gameState.season_context.phase !== "Preseason";
}
