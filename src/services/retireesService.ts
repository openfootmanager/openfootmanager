import { invoke } from "@tauri-apps/api/core";
import type { HallOfFameLegendData, PlayerData } from "../store/types";

export async function fetchHallOfFameLegends(): Promise<HallOfFameLegendData[]> {
  return invoke<HallOfFameLegendData[]>("get_hall_of_fame_legends");
}

export async function fetchRetiredPlayer(playerId: string): Promise<PlayerData> {
  return invoke<PlayerData>("get_retired_player", { playerId });
}
