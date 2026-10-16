import { render, screen } from "@testing-library/react";
import { describe, expect, it, vi } from "vitest";

import type { PlayerData, TransferOfferData } from "../../store/gameStore";
import { TransferOfferRecord } from "./TransfersPlayerOffers";
import type { TransfersTabController } from "./useTransfersTabController";

vi.mock("react-i18next", () => ({
  useTranslation: () => ({ t: (key: string) => key }),
}));

describe("TransferOfferRecord", () => {
  /**
   * Given a pending offer of €941,250 for a player
   * When the offer row is shown in the offers list
   * Then the fee is exact, matching the same offer in its history.
   */
  it("shows the offered fee exactly", () => {
    const state = {
      gameState: { teams: [{ id: "buyer", name: "Buyer FC" }] },
      t: (key: string) => key,
      userTeamId: "mine",
      handleRespondOffer: vi.fn(),
      openCounterNegotiation: vi.fn(),
    } as unknown as Pick<
      TransfersTabController,
      "gameState" | "t" | "userTeamId" | "handleRespondOffer" | "openCounterNegotiation"
    >;
    const offer = {
      id: "o1",
      from_team_id: "buyer",
      fee: 941250,
      status: "Pending",
    } as unknown as TransferOfferData;

    render(
      <TransferOfferRecord state={state} offer={offer} player={{ team_id: "x" } as PlayerData} />,
    );

    expect(screen.getByText(/€941,250/)).toBeInTheDocument();
  });
});
