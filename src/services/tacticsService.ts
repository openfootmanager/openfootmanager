import type { GameStateData, TeamMatchRolesData } from "../store/gameStore";
import { invokeCommand } from "./tauriClient";

export async function setStartingXI(playerIds: string[]): Promise<GameStateData> {
    return invokeCommand<GameStateData>("set_starting_xi", {
        playerIds,
    });
}

export async function setFormation(formation: string): Promise<GameStateData> {
    return invokeCommand<GameStateData>("set_formation", {
        formation,
    });
}

export async function setPlayStyle(playStyle: string): Promise<GameStateData> {
    return invokeCommand<GameStateData>("set_play_style", {
        playStyle,
    });
}

export async function setTeamMatchRoles(
    matchRoles: TeamMatchRolesData,
): Promise<GameStateData> {
    return invokeCommand<GameStateData>("set_team_match_roles", {
        matchRoles,
    });
}