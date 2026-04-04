import { beforeEach, describe, expect, it, vi } from "vitest";
import { invoke } from "@tauri-apps/api/core";

import { getPlayerMatchHistory, getPlayerStatsOverview } from "./playerProfileService";

vi.mock("@tauri-apps/api/core", () => ({
    invoke: vi.fn(),
}));

const mockedInvoke = vi.mocked(invoke);

describe("playerProfileService", () => {
    beforeEach(() => {
        mockedInvoke.mockReset();
    });

    it("loads player stats overview", async () => {
        const response = { percentileEligible: false, metrics: {} };
        mockedInvoke.mockResolvedValueOnce(response);

        await expect(getPlayerStatsOverview("player-1")).resolves.toBe(response);
        expect(mockedInvoke).toHaveBeenCalledWith("get_player_stats_overview", {
            playerId: "player-1",
        });
    });

    it("loads recent player match history", async () => {
        const response = [{ fixture_id: "fixture-1" }];
        mockedInvoke.mockResolvedValueOnce(response);

        await expect(getPlayerMatchHistory("player-1", 5)).resolves.toBe(response);
        expect(mockedInvoke).toHaveBeenCalledWith("get_player_match_history", {
            playerId: "player-1",
            limit: 5,
        });
    });
});