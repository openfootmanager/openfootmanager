import { invoke } from "@tauri-apps/api/core";

import type { GameStateData, ScoutedPlayerView } from "../store/gameStore";

export interface StartYouthScoutingInput {
  scoutId: string;
  region?: string | null;
  objective?: string | null;
  targetPosition?: string | null;
}

/** What the manager's club knows about a player, as `ofm_core::player_knowledge` decides it. */
export type PlayerKnowledge =
  | { kind: "own" }
  | { kind: "known" }
  | { kind: "scouted"; scouted_on: string; current: boolean }
  | { kind: "unknown" };

export async function getPlayerKnowledge(
  playerIds: string[],
): Promise<Record<string, PlayerKnowledge>> {
  return invoke<Record<string, PlayerKnowledge>>("get_player_knowledge", { playerIds });
}

/** The weekly wage the club may see, as `ofm_core::player_knowledge::visible_wage` decides it. */
export type VisibleWage =
  | { kind: "exact"; weekly: number }
  | { kind: "approximate"; weekly: number }
  | { kind: "hidden" };

export async function getVisibleWage(playerId: string): Promise<VisibleWage | null> {
  return invoke<VisibleWage | null>("get_visible_wage", { playerId });
}

export async function getScoutedReport(playerId: string): Promise<ScoutedPlayerView | null> {
  return invoke<ScoutedPlayerView | null>("get_scouted_report", { playerId });
}

export async function sendScout(scoutId: string, playerId: string): Promise<GameStateData> {
  return invoke<GameStateData>("send_scout", {
    scoutId,
    playerId,
  });
}

export async function startYouthScouting(input: StartYouthScoutingInput): Promise<GameStateData> {
  return invoke<GameStateData>("start_youth_scouting", {
    scoutId: input.scoutId,
    region: input.region ?? null,
    objective: input.objective ?? null,
    targetPosition: input.targetPosition ?? null,
  });
}

export async function cancelYouthScouting(assignmentId: string): Promise<GameStateData> {
  return invoke<GameStateData>("cancel_youth_scouting", {
    assignmentId,
  });
}

export async function reassignYouthScouting(
  assignmentId: string,
  scoutId: string,
): Promise<GameStateData> {
  return invoke<GameStateData>("reassign_youth_scouting", {
    assignmentId,
    scoutId,
  });
}
