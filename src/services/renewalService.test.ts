import { beforeEach, describe, expect, it, vi } from "vitest";
import { invoke } from "@tauri-apps/api/core";

import {
    delegateRenewals,
    previewRenewalFinancialImpact,
    proposeRenewal,
} from "./renewalService";

vi.mock("@tauri-apps/api/core", () => ({
    invoke: vi.fn(),
}));

const mockedInvoke = vi.mocked(invoke);

describe("renewalService", () => {
    beforeEach(() => {
        mockedInvoke.mockReset();
    });

    it("previews renewal impact", async () => {
        const response = { projection: { policy_allows: true } };
        mockedInvoke.mockResolvedValueOnce(response);

        await expect(previewRenewalFinancialImpact("player-1", 10000)).resolves.toBe(response);
        expect(mockedInvoke).toHaveBeenCalledWith("preview_renewal_financial_impact", {
            playerId: "player-1",
            weeklyWage: 10000,
        });
    });

    it("proposes a renewal", async () => {
        const response = { outcome: "accepted", game: { manager: { id: "manager-1" } } };
        mockedInvoke.mockResolvedValueOnce(response);

        await expect(proposeRenewal("player-1", 10000, 3)).resolves.toBe(response);
        expect(mockedInvoke).toHaveBeenCalledWith("propose_renewal", {
            playerId: "player-1",
            weeklyWage: 10000,
            contractYears: 3,
        });
    });

    it("delegates renewals", async () => {
        const response = { game: { manager: { id: "manager-1" } }, report: { success_count: 1, failure_count: 0, stalled_count: 0, cases: [] } };
        mockedInvoke.mockResolvedValueOnce(response);

        await expect(delegateRenewals({
            playerIds: ["player-1"],
            maxWageIncreasePct: 35,
            maxContractYears: 3,
        })).resolves.toBe(response);
        expect(mockedInvoke).toHaveBeenCalledWith("delegate_renewals", {
            playerIds: ["player-1"],
            maxWageIncreasePct: 35,
            maxContractYears: 3,
        });
    });
});