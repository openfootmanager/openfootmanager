import { beforeEach, describe, expect, it, vi } from "vitest";
import { invoke } from "@tauri-apps/api/core";

import { upgradeFacility } from "./facilityService";

vi.mock("@tauri-apps/api/core", () => ({
    invoke: vi.fn(),
}));

const mockedInvoke = vi.mocked(invoke);

describe("facilityService", () => {
    beforeEach(() => {
        mockedInvoke.mockReset();
    });

    it("upgrades a facility", async () => {
        const response = { manager: { id: "manager-1" } };
        mockedInvoke.mockResolvedValueOnce(response);

        await expect(upgradeFacility("Training")).resolves.toBe(response);
        expect(mockedInvoke).toHaveBeenCalledWith("upgrade_facility", {
            facility: "Training",
        });
    });
});