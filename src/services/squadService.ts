import { invoke } from "@tauri-apps/api/core";

import type { GameStateData } from "../store/gameStore";
import type { PlayerSquadRole } from "../store/types";

export async function setPlayerSquadRole(
    playerId: string,
    squadRole: PlayerSquadRole,
): Promise<GameStateData> {
    return invoke<GameStateData>("set_player_squad_role", {
        playerId,
        squadRole,
    });
}