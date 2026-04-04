import { beforeEach, describe, expect, it, vi } from "vitest";
import { invoke } from "@tauri-apps/api/core";

import { exitToMenu, getActiveGame, saveGame, selectTeam } from "./gameService";

vi.mock("@tauri-apps/api/core", () => ({
    invoke: vi.fn(),
}));

const mockedInvoke = vi.mocked(invoke);

describe("gameService", () => {
    beforeEach(() => {
        mockedInvoke.mockReset();
    });

    it("loads the active game", async () => {
        const game = { manager: { id: "manager-1" } };
        mockedInvoke.mockResolvedValueOnce(game);

        await expect(getActiveGame()).resolves.toBe(game);
        expect(mockedInvoke).toHaveBeenCalledWith("get_active_game");
    });

    it("saves the active game", async () => {
        mockedInvoke.mockResolvedValueOnce(undefined);

        await expect(saveGame()).resolves.toBeUndefined();
        expect(mockedInvoke).toHaveBeenCalledWith("save_game");
    });

    it("selects a team for the active manager", async () => {
        const game = { manager: { id: "manager-1" }, teams: [] };
        mockedInvoke.mockResolvedValueOnce(game);

        await expect(selectTeam("team-1")).resolves.toBe(game);
        expect(mockedInvoke).toHaveBeenCalledWith("select_team", {
            teamId: "team-1",
        });
    });

    it("exits to the menu through the backend command", async () => {
        mockedInvoke.mockResolvedValueOnce(undefined);

        await expect(exitToMenu()).resolves.toBeUndefined();
        expect(mockedInvoke).toHaveBeenCalledWith("exit_to_menu");
    });
});