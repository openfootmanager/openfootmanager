import type { GameStateData } from "../store/gameStore";
import { invokeCommand } from "./tauriClient";

export interface TrainingGroupData {
  id: string;
  name: string;
  focus: string;
  player_ids: string[];
}

export async function setTraining(
  focus: string,
  intensity: string,
): Promise<GameStateData> {
  return invokeCommand<GameStateData>("set_training", {
    focus,
    intensity,
  });
}

export async function setTrainingSchedule(
  schedule: string,
): Promise<GameStateData> {
  return invokeCommand<GameStateData>("set_training_schedule", {
    schedule,
  });
}

export async function setTrainingGroups(
  groups: TrainingGroupData[],
): Promise<GameStateData> {
  return invokeCommand<GameStateData>("set_training_groups", {
    groups,
  });
}

export async function setPlayerTrainingFocus(
  playerId: string,
  focus: string | null,
): Promise<GameStateData> {
  return invokeCommand<GameStateData>("set_player_training_focus", {
    playerId,
    focus,
  });
}