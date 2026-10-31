import { beforeEach, describe, expect, it, vi } from "vitest";
import { invoke } from "@tauri-apps/api/core";

import {
  getFreeAgentOpeningOffer,
  offerFreeAgentContract,
  previewFreeAgentContractImpact,
} from "./freeAgentService";

vi.mock("@tauri-apps/api/core", () => ({
  invoke: vi.fn(),
}));

const mockedInvoke = vi.mocked(invoke);

describe("freeAgentService", () => {
  beforeEach(() => {
    mockedInvoke.mockReset();
  });

  it("submits a free-agent contract offer", async () => {
    const response = { outcome: "accepted", game: { manager: { id: "manager-1" } } };
    mockedInvoke.mockResolvedValueOnce(response);

    await expect(offerFreeAgentContract("player-1", 4000, 3)).resolves.toBe(response);
    expect(mockedInvoke).toHaveBeenCalledWith("offer_free_agent_contract", {
      playerId: "player-1",
      weeklyWage: 4000,
      contractYears: 3,
    });
  });

  it("asks the backend for the opening offer to a free agent", async () => {
    const offer = { weekly_wage: 7000, contract_years: 4 };
    mockedInvoke.mockResolvedValueOnce(offer);

    await expect(getFreeAgentOpeningOffer("player-1")).resolves.toBe(offer);
    expect(mockedInvoke).toHaveBeenCalledWith("get_free_agent_opening_offer", {
      playerId: "player-1",
    });
  });

  it("passes on the backend's refusal of an opening offer", async () => {
    mockedInvoke.mockRejectedValueOnce("be.error.contracts.playerNotFreeAgent");

    await expect(getFreeAgentOpeningOffer("player-1")).rejects.toBe(
      "be.error.contracts.playerNotFreeAgent",
    );
  });

  it("previews a free-agent contract impact", async () => {
    const response = { projection: { projected_annual_wage_bill: 4000 } };
    mockedInvoke.mockResolvedValueOnce(response);

    await expect(previewFreeAgentContractImpact("player-1", 4000)).resolves.toBe(response);
    expect(mockedInvoke).toHaveBeenCalledWith("preview_free_agent_contract_impact", {
      playerId: "player-1",
      weeklyWage: 4000,
    });
  });
});
