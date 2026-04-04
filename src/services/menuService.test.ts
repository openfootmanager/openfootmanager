import { beforeEach, describe, expect, it, vi } from "vitest";
import { invoke } from "@tauri-apps/api/core";

import {
    deleteSave,
    getSaves,
    listWorldDatabases,
    loadGame,
    startNewGame,
    writeTempDatabase,
} from "./menuService";

vi.mock("@tauri-apps/api/core", () => ({
    invoke: vi.fn(),
}));

const mockedInvoke = vi.mocked(invoke);

describe("menuService", () => {
    beforeEach(() => {
        mockedInvoke.mockReset();
    });

    it("lists available world databases", async () => {
        const databases = [{ id: "random" }];
        mockedInvoke.mockResolvedValueOnce(databases);

        await expect(listWorldDatabases()).resolves.toBe(databases);
        expect(mockedInvoke).toHaveBeenCalledWith("list_world_databases");
    });

    it("writes an imported world database to temporary storage", async () => {
        mockedInvoke.mockResolvedValueOnce("temp-path.json");

        await expect(writeTempDatabase("{\"hello\":\"world\"}")).resolves.toBe(
            "temp-path.json",
        );
        expect(mockedInvoke).toHaveBeenCalledWith("write_temp_database", {
            json: "{\"hello\":\"world\"}",
        });
    });

    it("starts a new game", async () => {
        const game = { manager: { id: "manager-1" } };
        mockedInvoke.mockResolvedValueOnce(game);

        await expect(
            startNewGame({
                dob: "1980-01-01",
                firstName: "Ada",
                lastName: "Lovelace",
                nationality: "GB",
                worldSource: "random",
            }),
        ).resolves.toBe(game);

        expect(mockedInvoke).toHaveBeenCalledWith("start_new_game", {
            dob: "1980-01-01",
            firstName: "Ada",
            lastName: "Lovelace",
            nationality: "GB",
            worldSource: "random",
        });
    });

    it("loads available saves", async () => {
        const saves = [{ id: "save-1" }];
        mockedInvoke.mockResolvedValueOnce(saves);

        await expect(getSaves()).resolves.toBe(saves);
        expect(mockedInvoke).toHaveBeenCalledWith("get_saves");
    });

    it("loads a saved game", async () => {
        mockedInvoke.mockResolvedValueOnce("Ada Lovelace");

        await expect(loadGame("save-1")).resolves.toBe("Ada Lovelace");
        expect(mockedInvoke).toHaveBeenCalledWith("load_game", {
            saveId: "save-1",
        });
    });

    it("deletes a saved game", async () => {
        mockedInvoke.mockResolvedValueOnce(true);

        await expect(deleteSave("save-1")).resolves.toBe(true);
        expect(mockedInvoke).toHaveBeenCalledWith("delete_save", {
            saveId: "save-1",
        });
    });
});