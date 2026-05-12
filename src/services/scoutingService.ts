import { invoke } from "@tauri-apps/api/core";

import type { GameStateData } from "../store/gameStore";

export interface StartYouthScoutingInput {
  scoutId: string;
  region?: string | null;
  objective?: string | null;
  targetPosition?: string | null;
}

export async function sendScout(
  scoutId: string,
  playerId: string,
): Promise<GameStateData> {
  return invoke<GameStateData>("send_scout", {
    scoutId,
    playerId,
  });
}

export async function startYouthScouting(
  input: StartYouthScoutingInput,
): Promise<GameStateData> {
  return invoke<GameStateData>("start_youth_scouting", {
    scoutId: input.scoutId,
    region: input.region ?? null,
    objective: input.objective ?? null,
    targetPosition: input.targetPosition ?? null,
  });
}

export async function cancelYouthScouting(
  assignmentId: string,
): Promise<GameStateData> {
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