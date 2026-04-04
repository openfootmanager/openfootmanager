import { beforeEach, describe, expect, it, vi } from "vitest";
import { invoke } from "@tauri-apps/api/core";

import { advanceToNextSeason } from "./seasonTransitionService";

vi.mock("@tauri-apps/api/core", () => ({
    invoke: vi.fn(),
}));

const mockedInvoke = vi.mocked(invoke);

describe("seasonTransitionService", () => {
    beforeEach(() => {
        mockedInvoke.mockReset();
    });

    it("advances to the next season", async () => {
        const response = { game: { manager: { id: "manager-1" } }, summary: { season: 2026 } };
        mockedInvoke.mockResolvedValueOnce(response);

        await expect(advanceToNextSeason()).resolves.toBe(response);
        expect(mockedInvoke).toHaveBeenCalledWith("advance_to_next_season");
    });
});