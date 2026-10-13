import { describe, expect, it } from "vitest";
import { formatVal } from "../../lib/helpers";
import type { MessageAction, MessageData } from "../../store/gameStore";
import {
  formatSignedAmount,
  isChooseOptionAction,
  isPendingSponsorOffer,
} from "./FinancesTab.helpers";

describe("formatSignedAmount", () => {
  it("renders a negative amount with a leading minus", () => {
    expect(formatSignedAmount(-5_000_000).startsWith("-")).toBe(true);
    expect(formatSignedAmount(5_000_000).startsWith("-")).toBe(false);
  });

  it("defers sign placement to formatVal", () => {
    // Pins the reason the abs()-then-prepend-"-" version was dropped: formatVal
    // already renders the sign, so the two agree for every input.
    for (const value of [-5_000_000, -1_250, 0, 1_250, 5_000_000]) {
      expect(formatSignedAmount(value)).toBe(formatVal(value));
    }
  });
});

describe("isChooseOptionAction", () => {
  it("recognises a choice action", () => {
    const actionType: MessageAction["action_type"] = {
      ChooseOption: { options: [{ id: "accept", label: "Accept", description: "" }] },
    };

    expect(isChooseOptionAction(actionType)).toBe(true);
  });

  it("rejects a string action type", () => {
    expect(isChooseOptionAction("Acknowledge" as MessageAction["action_type"])).toBe(false);
  });
});

describe("isPendingSponsorOffer", () => {
  function sponsorMessage(overrides: Partial<MessageData> = {}): MessageData {
    return {
      id: "sponsor_offer_1",
      category: "Finance",
      actions: [
        {
          resolved: false,
          action_type: {
            ChooseOption: { options: [{ id: "accept", label: "Accept", description: "" }] },
          },
        },
      ],
      ...overrides,
    } as MessageData;
  }

  it("accepts an unresolved sponsor choice", () => {
    expect(isPendingSponsorOffer(sponsorMessage())).toBe(true);
  });

  it("rejects an offer that has already been answered", () => {
    const answered = sponsorMessage();
    answered.actions[0].resolved = true;

    expect(isPendingSponsorOffer(answered)).toBe(false);
  });

  it("rejects a finance message that is not a sponsor offer", () => {
    expect(isPendingSponsorOffer(sponsorMessage({ id: "board_warning_1" }))).toBe(false);
  });

  it("rejects a sponsor-shaped message filed under another category", () => {
    expect(isPendingSponsorOffer(sponsorMessage({ category: "Transfer" }))).toBe(false);
  });
});
