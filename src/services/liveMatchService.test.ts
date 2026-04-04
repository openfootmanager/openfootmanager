import { beforeEach, describe, expect, it, vi } from "vitest";
import { invoke } from "@tauri-apps/api/core";

import {
    finishLiveMatch,
    getMatchSnapshot,
    startLiveMatchSession,
} from "./liveMatchService";

vi.mock("@tauri-apps/api/core", () => ({
    invoke: vi.fn(),
}));

const mockedInvoke = vi.mocked(invoke);

describe("liveMatchService", () => {
    beforeEach(() => {
        mockedInvoke.mockReset();
    });

    it("loads the current live match snapshot", async () => {
        const snapshot = { phase: "FirstHalf" };
        mockedInvoke.mockResolvedValueOnce(snapshot);

        await expect(getMatchSnapshot()).resolves.toBe(snapshot);
        expect(mockedInvoke).toHaveBeenCalledWith("get_match_snapshot");
    });

    it("starts or restores a live match session", async () => {
        const snapshot = { phase: "PreMatch" };
        mockedInvoke.mockResolvedValueOnce(snapshot);

        await expect(
            startLiveMatchSession({
                allowsExtraTime: false,
                fixtureIndex: 7,
                mode: "spectator",
            }),
        ).resolves.toBe(snapshot);

        expect(mockedInvoke).toHaveBeenCalledWith("start_live_match", {
            allowsExtraTime: false,
            fixtureIndex: 7,
            mode: "spectator",
        });
    });

    it("finalizes the live match", async () => {
        const response = { game: { manager: { id: "manager-1" } } };
        mockedInvoke.mockResolvedValueOnce(response);

        await expect(finishLiveMatch()).resolves.toBe(response);
        expect(mockedInvoke).toHaveBeenCalledWith("finish_live_match");
    });
});