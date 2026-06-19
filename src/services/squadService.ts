import { invoke } from "@tauri-apps/api/core";

import type { GameStateData, PlayerData } from "../store/gameStore";
import type { PlayerSquadRole } from "../store/types";

export async function getSquad(teamId: string): Promise<PlayerData[]> {
    return invoke<PlayerData[]>("get_squad", { teamId });
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