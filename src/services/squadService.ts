import { invoke } from "@tauri-apps/api/core";

import type { GameStateData, PlayerData } from "../store/gameStore";
import type { PlayerRole, PlayerSquadRole, TacticsPhaseSettings } from "../store/types";

/** The backend's tier names, in the order a rating climbs through them. */
export type RatingTier = "S" | "A" | "B" | "Rest";

/** The tier of every rating from 0 to 99, indexed by rating: the backend's one rule, no game needed. */
export async function getRatingTiers(): Promise<RatingTier[]> {
  return invoke<RatingTier[]>("get_rating_tiers");
}

export async function getSquad(teamId: string): Promise<PlayerData[]> {
  return invoke<PlayerData[]>("get_squad", { teamId });
}

/** The takers the match would use for these players when none are assigned, as the backend picks them. */
export interface AutoSetPieces {
  captain: string | null;
  penalty_taker: string | null;
  free_kick_taker: string | null;
  corner_taker: string | null;
}

export async function autoSelectSetPieces(playerIds: string[]): Promise<AutoSetPieces> {
  return invoke<AutoSetPieces>("auto_select_set_pieces", { playerIds });
}

export async function setPlayerSquadRole(
  playerId: string,
  squadRole: PlayerSquadRole,
): Promise<GameStateData> {
  return invoke<GameStateData>("set_player_squad_role", {
    playerId,
    squadRole,
  });
}

export async function setStartingXi(playerIds: string[]): Promise<GameStateData> {
  return invoke<GameStateData>("set_starting_xi", { playerIds });
}

export async function setPlayerRole(
  playerId: string,
  role: PlayerRole | null,
): Promise<GameStateData> {
  return invoke<GameStateData>("set_player_role", {
    playerId,
    role: role ?? undefined,
  });
}

/**
 * Pick a whole tactic preset: formation, play style and the phase blueprint
 * that style implies, in one atomic backend call.
 */
export async function applyTacticPreset(
  formation: string,
  playStyle: string,
): Promise<GameStateData> {
  return invoke<GameStateData>("apply_tactic_preset", { formation, playStyle });
}

/** Whether the user's phase dials differ from the blueprint their play style implies. */
export async function getTacticsBlueprintStatus(): Promise<{ modified: boolean }> {
  return invoke<{ modified: boolean }>("get_tactics_blueprint_status");
}

export async function setTacticsPhase(
  patch: Partial<TacticsPhaseSettings>,
): Promise<GameStateData> {
  return invoke<GameStateData>("set_tactics_phase", {
    buildUpStyle: patch.build_up_style,
    width: patch.width,
    tempo: patch.tempo,
    defensiveLine: patch.defensive_line,
    pressingIntensity: patch.pressing_intensity,
    defensiveShape: patch.defensive_shape,
    markingStyle: patch.marking_style,
    counterPressDuration: patch.counter_press_duration,
    breakSpeed: patch.break_speed,
  });
}
