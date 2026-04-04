import { beforeEach, describe, expect, it, vi } from "vitest";
import { invoke } from "@tauri-apps/api/core";

import {
    setFormation,
    setPlayStyle,
    setStartingXI,
    setTeamMatchRoles,
} from "./tacticsService";

vi.mock("@tauri-apps/api/core", () => ({
    invoke: vi.fn(),
}));

const mockedInvoke = vi.mocked(invoke);

describe("tacticsService", () => {
    beforeEach(() => {
        mockedInvoke.mockReset();
    });

    it("persists the starting XI", async () => {
        const response = { manager: { id: "manager-1" } };
        mockedInvoke.mockResolvedValueOnce(response);

        await expect(setStartingXI(["player-1", "player-2"])).resolves.toBe(response);
        expect(mockedInvoke).toHaveBeenCalledWith("set_starting_xi", {
            playerIds: ["player-1", "player-2"],
        });
    });

    it("persists the formation", async () => {
        const response = { manager: { id: "manager-1" } };
        mockedInvoke.mockResolvedValueOnce(response);

        await expect(setFormation("4-3-3")).resolves.toBe(response);
        expect(mockedInvoke).toHaveBeenCalledWith("set_formation", {
            formation: "4-3-3",
        });
    });

    it("persists the play style", async () => {
        const response = { manager: { id: "manager-1" } };
        mockedInvoke.mockResolvedValueOnce(response);

        await expect(setPlayStyle("Attacking")).resolves.toBe(response);
        expect(mockedInvoke).toHaveBeenCalledWith("set_play_style", {
            playStyle: "Attacking",
        });
    });

    it("persists team match roles", async () => {
        const response = { manager: { id: "manager-1" } };
        const matchRoles = {
            captain: "player-1",
            vice_captain: "player-2",
            penalty_taker: "player-3",
            free_kick_taker: "player-4",
            corner_taker: "player-5",
        };
        mockedInvoke.mockResolvedValueOnce(response);

        await expect(setTeamMatchRoles(matchRoles)).resolves.toBe(response);
        expect(mockedInvoke).toHaveBeenCalledWith("set_team_match_roles", {
            matchRoles,
        });
    });
});