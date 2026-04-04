import type { GameStateData } from "../store/gameStore";
import { invokeCommand } from "./tauriClient";
import type { SaveEntry, WorldDatabaseInfo } from "./menuTypes";

export interface StartNewGameInput {
    firstName: string;
    lastName: string;
    dob: string;
    nationality: string;
    worldSource?: string;
}

export async function listWorldDatabases(): Promise<WorldDatabaseInfo[]> {
    return invokeCommand<WorldDatabaseInfo[]>("list_world_databases");
}

export async function writeTempDatabase(json: string): Promise<string> {
    return invokeCommand<string>("write_temp_database", { json });
}

export async function startNewGame(
    input: StartNewGameInput,
): Promise<GameStateData> {
    return invokeCommand<GameStateData>("start_new_game", input);
}

export async function getSaves(): Promise<SaveEntry[]> {
    return invokeCommand<SaveEntry[]>("get_saves");
}

export async function loadGame(saveId: string): Promise<string> {
    return invokeCommand<string>("load_game", { saveId });
}

export async function deleteSave(saveId: string): Promise<boolean> {
    return invokeCommand<boolean>("delete_save", { saveId });
}