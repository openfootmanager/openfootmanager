import type { GameStateData } from "../store/gameStore";
import { invokeCommand, invokeVoidCommand } from "./tauriClient";

export async function getActiveGame(): Promise<GameStateData> {
    return invokeCommand<GameStateData>("get_active_game");
}

export async function saveGame(): Promise<void> {
    await invokeVoidCommand("save_game");
}

export async function selectTeam(teamId: string): Promise<GameStateData> {
    return invokeCommand<GameStateData>("select_team", { teamId });
}

export async function exitToMenu(): Promise<void> {
    await invokeVoidCommand("exit_to_menu");
}