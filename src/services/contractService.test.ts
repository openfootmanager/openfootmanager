import { beforeEach, describe, expect, it, vi } from "vitest";
import { invoke } from "@tauri-apps/api/core";

import {
    clearContractExitIntent,
    previewContractTermination,
    setContractExitIntent,
    terminateContractNow,
} from "./contractService";

vi.mock("@tauri-apps/api/core", () => ({
    invoke: vi.fn(),
}));

const mockedInvoke = vi.mocked(invoke);

describe("contractService", () => {
    beforeEach(() => {
        mockedInvoke.mockReset();
    });

    it("marks a contract to expire", async () => {
        const response = { game: { manager: { id: "manager-1" } } };
        mockedInvoke.mockResolvedValueOnce(response);

        await expect(
            setContractExitIntent("player-1", "manager_profile_action"),
        ).resolves.toBe(response);
        expect(mockedInvoke).toHaveBeenCalledWith("set_contract_exit_intent", {
            playerId: "player-1",
            reason: "manager_profile_action",
        });
    });

    it("clears a planned expiry intent", async () => {
        const response = { game: { manager: { id: "manager-1" } } };
        mockedInvoke.mockResolvedValueOnce(response);

        await expect(clearContractExitIntent("player-1")).resolves.toBe(response);
        expect(mockedInvoke).toHaveBeenCalledWith("clear_contract_exit_intent", {
            playerId: "player-1",
        });
    });

    it("previews immediate termination", async () => {
        const response = { preview: { severance_cost: 132000 } };
        mockedInvoke.mockResolvedValueOnce(response);

        await expect(previewContractTermination("player-1")).resolves.toBe(response);
        expect(mockedInvoke).toHaveBeenCalledWith("preview_contract_termination", {
            playerId: "player-1",
        });
    });

    it("terminates a contract immediately", async () => {
        const response = { game: { manager: { id: "manager-1" } } };
        mockedInvoke.mockResolvedValueOnce(response);

        await expect(terminateContractNow("player-1")).resolves.toBe(response);
        expect(mockedInvoke).toHaveBeenCalledWith("terminate_contract_now", {
            playerId: "player-1",
        });
    });
});

