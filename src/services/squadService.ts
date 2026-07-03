import { invoke } from "@tauri-apps/api/core";

import type { GameStateData, PlayerData } from "../store/gameStore";
import type { KitPattern, PlayerRole, PlayerSquadRole, TacticsPhaseSettings } from "../store/types";

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

export async function setStartingXi(
    playerIds: string[],
): Promise<GameStateData> {
    return invoke<GameStateData>("set_starting_xi", { playerIds });
}

export async function setPlayerRole(
    playerId: string,
    role: PlayerRole | null,
): Promise<GameStateData> {
    return invoke<GameStateData>("set_player_role", {
        playerId,
        role: role ?? undefined,
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

export async function setTacticsPhase(
    patch: Partial<TacticsPhaseSettings>,
): Promise<GameStateData> {
    return invoke<GameStateData>("set_tactics_phase", {
        buildUpStyle: patch.build_up_style,
        width: patch.width,
        tempo: patch.tempo,
        defensiveLine: patch.defensive_line,
        pressingIntensity: patch.pressing_intensity,
        defensiveShape: patch.defensive_shape,
        markingStyle: patch.marking_style,
        counterPressDuration: patch.counter_press_duration,
        breakSpeed: patch.break_speed,
    });
}

export async function setTeamKitPattern(
    kitPattern: KitPattern,
): Promise<GameStateData> {
    return invoke<GameStateData>("set_team_kit_pattern", {
        kitPattern,
    });
}