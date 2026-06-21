import { invoke } from "@tauri-apps/api/core";

import type { GameStateData, PlayerData } from "../store/gameStore";
import type { KitPattern, PlayerSquadRole } from "../store/types";

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

export async function assignJerseyNumber(
    playerId: string,
    jerseyNumber: number | null,
): Promise<GameStateData> {
    return invoke<GameStateData>("assign_jersey_number", {
        playerId,
        jerseyNumber,
    });
}

export async function setTeamKitPattern(
    kitPattern: KitPattern,
): Promise<GameStateData> {
    return invoke<GameStateData>("set_team_kit_pattern", {
        kitPattern,
    });
}