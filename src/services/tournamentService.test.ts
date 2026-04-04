import { beforeEach, describe, expect, it, vi } from "vitest";
import { invoke } from "@tauri-apps/api/core";

import { getSeasonAwards } from "./tournamentService";

vi.mock("@tauri-apps/api/core", () => ({
    invoke: vi.fn(),
}));

const mockedInvoke = vi.mocked(invoke);

describe("tournamentService", () => {
    beforeEach(() => {
        mockedInvoke.mockReset();
    });

    it("loads season awards", async () => {
        const awards = { golden_boot: [] };
        mockedInvoke.mockResolvedValueOnce(awards);

        await expect(getSeasonAwards()).resolves.toBe(awards);
        expect(mockedInvoke).toHaveBeenCalledWith("get_season_awards");
    });
});