import { beforeEach, describe, expect, it, vi } from "vitest";
import { invoke } from "@tauri-apps/api/core";

import {
    clearAllSaves,
    exportWorldDatabase,
    getSettings,
    saveSettings,
} from "./settingsService";

vi.mock("@tauri-apps/api/core", () => ({
    invoke: vi.fn(),
}));

const mockedInvoke = vi.mocked(invoke);

describe("settingsService", () => {
    beforeEach(() => {
        mockedInvoke.mockReset();
    });

    it("loads persisted settings", async () => {
        const settings = { language: "es" };
        mockedInvoke.mockResolvedValueOnce(settings);

        await expect(getSettings()).resolves.toBe(settings);
        expect(mockedInvoke).toHaveBeenCalledWith("get_settings");
    });

    it("persists settings", async () => {
        const settings = {
            theme: "dark",
            language: "en",
            currency: "EUR",
            default_match_mode: "live",
            auto_save: true,
            match_speed: "normal",
            show_match_commentary: true,
            confirm_advance: false,
            ui_scale: "normal",
            high_contrast: false,
        } as const;
        mockedInvoke.mockResolvedValueOnce(undefined);

        await expect(saveSettings(settings)).resolves.toBeUndefined();
        expect(mockedInvoke).toHaveBeenCalledWith("save_settings", { settings });
    });

    it("clears all saves", async () => {
        mockedInvoke.mockResolvedValueOnce(undefined);

        await expect(clearAllSaves()).resolves.toBeUndefined();
        expect(mockedInvoke).toHaveBeenCalledWith("clear_all_saves");
    });

    it("exports the world database", async () => {
        mockedInvoke.mockResolvedValueOnce("exported_world.json");

        await expect(exportWorldDatabase("exported_world.json")).resolves.toBe(
            "exported_world.json",
        );
        expect(mockedInvoke).toHaveBeenCalledWith("export_world_database", {
            exportPath: "exported_world.json",
        });
    });
});