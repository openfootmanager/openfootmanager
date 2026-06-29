import { beforeEach, describe, expect, it, vi } from "vitest";
import { invoke } from "@tauri-apps/api/core";

import {
  counterLoanOffer,
  counterOffer,
  exerciseLoanBuyOption,
  makeLoanOffer,
  makeTransferBid,
  previewTransferBidFinancialImpact,
  respondToLoanOffer,
  respondToOffer,
} from "./transfersService";

vi.mock("@tauri-apps/api/core", () => ({
  invoke: vi.fn(),
}));

const mockedInvoke = vi.mocked(invoke);

describe("transfersService", () => {
  beforeEach(() => {
    mockedInvoke.mockReset();
  });

  it("calls the make transfer bid backend command", async () => {
    const response = { decision: "accepted" };
    mockedInvoke.mockResolvedValueOnce(response);

    await expect(makeTransferBid("player-1", 1500000)).resolves.toBe(response);
    expect(mockedInvoke).toHaveBeenCalledWith("make_transfer_bid", {
      playerId: "player-1",
      fee: 1500000,
    });
  });

  it("calls the respond to offer backend command", async () => {
    const response = { manager: { id: "manager-1" } };
    mockedInvoke.mockResolvedValueOnce(response);

    await expect(respondToOffer("player-1", "offer-1", true)).resolves.toBe(response);
    expect(mockedInvoke).toHaveBeenCalledWith("respond_to_offer", {
      playerId: "player-1",
      offerId: "offer-1",
      accept: true,
    });
  });

  it("calls the make loan offer backend command", async () => {
    const response = { decision: "accepted" };
    mockedInvoke.mockResolvedValueOnce(response);

    await expect(makeLoanOffer("player-1", "2027-01-01", 75)).resolves.toBe(response);
    expect(mockedInvoke).toHaveBeenCalledWith("make_loan_offer", {
      playerId: "player-1",
      endDate: "2027-01-01",
      wageContributionPct: 75,
      buyOptionFee: null,
    });
  });

  it("calls the exercise loan buy option backend command", async () => {
    const response = { manager: { id: "manager-1" } };
    mockedInvoke.mockResolvedValueOnce(response);

    await expect(exerciseLoanBuyOption("player-1")).resolves.toBe(response);
    expect(mockedInvoke).toHaveBeenCalledWith("exercise_loan_buy_option", {
      playerId: "player-1",
    });
  });

  it("calls the respond to loan offer backend command", async () => {
    const response = { manager: { id: "manager-1" } };
    mockedInvoke.mockResolvedValueOnce(response);

    await expect(respondToLoanOffer("player-1", "loan-offer-1", true)).resolves.toBe(response);
    expect(mockedInvoke).toHaveBeenCalledWith("respond_to_loan_offer", {
      playerId: "player-1",
      offerId: "loan-offer-1",
      accept: true,
    });
  });

  it("calls the counter loan offer backend command", async () => {
    const response = { decision: "counter_offer" };
    mockedInvoke.mockResolvedValueOnce(response);

    await expect(
      counterLoanOffer("player-1", "loan-offer-1", "2027-01-01", 85, 1200000),
    ).resolves.toBe(response);
    expect(mockedInvoke).toHaveBeenCalledWith("counter_loan_offer", {
      playerId: "player-1",
      offerId: "loan-offer-1",
      endDate: "2027-01-01",
      wageContributionPct: 85,
      buyOptionFee: 1200000,
    });
  });

  it("calls the counter offer backend command", async () => {
    const response = { decision: "counter_offer" };
    mockedInvoke.mockResolvedValueOnce(response);

    await expect(counterOffer("player-1", "offer-1", 1800000)).resolves.toBe(response);
    expect(mockedInvoke).toHaveBeenCalledWith("counter_offer", {
      playerId: "player-1",
      offerId: "offer-1",
      requestedFee: 1800000,
    });
  });

  it("calls the transfer bid projection backend command", async () => {
    const response = { projection: { transfer_budget_before: 0 } };
    mockedInvoke.mockResolvedValueOnce(response);

    await expect(previewTransferBidFinancialImpact("player-1", 1000000)).resolves.toBe(response);
    expect(mockedInvoke).toHaveBeenCalledWith("preview_transfer_bid_financial_impact", {
      playerId: "player-1",
      fee: 1000000,
    });
  });
});
