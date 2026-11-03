import { render, screen } from "@testing-library/react";
import { describe, expect, it, vi } from "vitest";

import type { TransferOfferData } from "../../store/gameStore";
import TransferNegotiationHistory from "./TransferNegotiationHistory";

vi.mock("react-i18next", () => ({
  useTranslation: () => ({ t: (key: string) => key }),
}));

const offer = {
  fee: 960500,
  last_manager_fee: 941250,
  suggested_counter_fee: null,
  negotiation_round: 2,
} as unknown as TransferOfferData;

describe("TransferNegotiationHistory", () => {
  /**
   * Given a negotiation still in its first round, whose bid was turned down
   * When the exchange is shown
   * Then the history is there from the first round, so a rejection streak is not invisible.
   */
  it("shows the exchange from the first round", () => {
    render(
      <TransferNegotiationHistory
        offer={{ ...offer, negotiation_round: 1 } as unknown as TransferOfferData}
        mode="outgoing"
      />,
    );

    expect(screen.getByText("€941,250")).toBeInTheDocument();
  });

  /**
   * Given a negotiation in its second round, with a bid of €941,250 and an offer of €960,500
   * When the exchange is shown
   * Then both fees are exact: a counter is decided on the digits, which "€941K" and "€961K" hide.
   */
  it("shows both sides of the exchange exactly", () => {
    render(<TransferNegotiationHistory offer={offer} mode="outgoing" />);

    expect(screen.getByText("€941,250")).toBeInTheDocument();
    expect(screen.getByText("€960,500")).toBeInTheDocument();
  });
});
