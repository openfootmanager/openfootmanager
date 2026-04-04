import type { AppSettings } from "../store/settingsTypes";
import { invokeCommand, invokeVoidCommand } from "./tauriClient";

export async function getSettings(): Promise<Partial<AppSettings>> {
    return invokeCommand<Partial<AppSettings>>("get_settings");
}

export async function saveSettings(settings: AppSettings): Promise<void> {
    await invokeVoidCommand("save_settings", { settings });
}

export async function clearAllSaves(): Promise<void> {
    await invokeVoidCommand("clear_all_saves");
}

export async function exportWorldDatabase(
    exportPath: string,
): Promise<string> {
    return invokeCommand<string>("export_world_database", { exportPath });
}