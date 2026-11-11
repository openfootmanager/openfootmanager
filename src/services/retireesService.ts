import { invoke } from "@tauri-apps/api/core";
import type { HallOfFameLegendData, PastChampionData, PlayerData } from "../store/types";

export async function fetchHallOfFameLegends(): Promise<HallOfFameLegendData[]> {
  return invoke<HallOfFameLegendData[]>("get_hall_of_fame_legends");
}

/** Every league title in the world's past, latest first; the backend decides what is a title. */
export async function fetchHallOfFameChampions(): Promise<PastChampionData[]> {
  return invoke<PastChampionData[]>("get_hall_of_fame_champions");
}

export async function fetchRetiredPlayer(playerId: string): Promise<PlayerData> {
  return invoke<PlayerData>("get_retired_player", { playerId });
}
