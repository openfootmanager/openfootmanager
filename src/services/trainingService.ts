import { invoke } from "@tauri-apps/api/core";

import type { GameStateData } from "../store/gameStore";
import type { TrainingGroupData } from "../store/types";

// Lives in `store/types.ts` with the rest of the wire shape — `TeamData.training_groups` needs
// it, and a type the store depends on cannot sit in a service the store does not import.
// Re-exported here so existing imports keep working.
export type { TrainingGroupData } from "../store/types";

export async function setTraining(focus: string, intensity: string): Promise<GameStateData> {
  return invoke<GameStateData>("set_training", {
    focus,
    intensity,
  });
}

export async function setTrainingSchedule(schedule: string): Promise<GameStateData> {
  return invoke<GameStateData>("set_training_schedule", {
    schedule,
  });
}

export async function setTrainingGroups(groups: TrainingGroupData[]): Promise<GameStateData> {
  return invoke<GameStateData>("set_training_groups", {
    groups,
  });
}

export async function setPlayerTrainingFocus(
  playerId: string,
  focus: string | null,
): Promise<GameStateData> {
  return invoke<GameStateData>("set_player_training_focus", {
    playerId,
    focus,
  });
}
