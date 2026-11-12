import { invoke } from "@tauri-apps/api/core";

import type { PlayerCareer } from "../store/generated/PlayerCareer";

/**
 * One player's career for his profile, as `ofm_core::player_career` builds it: his seasons at each
 * club newest first with each move written where it happened, the same rows summed by club, and his
 * totals; an archived retiree's is his saved seasons. Null for an id neither an active player nor a retiree has. The screen draws it as given.
 */
export async function getPlayerCareer(playerId: string): Promise<PlayerCareer | null> {
  return invoke<PlayerCareer | null>("get_player_career", { playerId });
}
