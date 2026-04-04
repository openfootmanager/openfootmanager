import { beforeEach, describe, expect, it, vi } from "vitest";
import { invoke } from "@tauri-apps/api/core";

import { toggleLoanList, toggleTransferList } from "./squadService";

vi.mock("@tauri-apps/api/core", () => ({
    invoke: vi.fn(),
}));

const mockedInvoke = vi.mocked(invoke);

describe("squadService", () => {
    beforeEach(() => {
        mockedInvoke.mockReset();
    });

    it("toggles a player on the transfer list", async () => {
        const gameState = { manager: { id: "manager-1" } };
        mockedInvoke.mockResolvedValueOnce(gameState);

        await expect(toggleTransferList("player-1")).resolves.toBe(gameState);
        expect(mockedInvoke).toHaveBeenCalledWith("toggle_transfer_list", {
            playerId: "player-1",
        });
    });

    it("toggles a player on the loan list", async () => {
        const gameState = { manager: { id: "manager-1" } };
        mockedInvoke.mockResolvedValueOnce(gameState);

        await expect(toggleLoanList("player-2")).resolves.toBe(gameState);
        expect(mockedInvoke).toHaveBeenCalledWith("toggle_loan_list", {
            playerId: "player-2",
        });
    });
});