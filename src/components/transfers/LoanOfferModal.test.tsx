import { render, screen } from "@testing-library/react";
import { describe, expect, it, vi } from "vitest";

import { createPlayer, createTeam } from "../../test-utils/factories";
import LoanOfferModal from "./LoanOfferModal";

vi.mock("react-i18next", () => ({
  useTranslation: () => ({
    t: (key: string) => key,
    i18n: { language: "en" },
  }),
}));

describe("LoanOfferModal", () => {
  /**
   * Given a loan target whose `position` is the legacy bucket and whose natural position is granular
   * When the loan modal opens
   * Then the badge reads the granular position.
   */
  it("badges the player by their granular natural position", () => {
    render(
      <LoanOfferModal
        loanTarget={createPlayer({ position: "Defender", natural_position: "LeftBack" })}
        teams={[createTeam()]}
        periodId=""
        periodOptions={[]}
        selectedEndDate="2027-01-01"
        onPeriodChange={vi.fn()}
        wageContributionPct={50}
        onWageContributionChange={vi.fn()}
        buyOptionEnabled={false}
        buyOptionFee=""
        onBuyOptionEnabledChange={vi.fn()}
        onBuyOptionFeeChange={vi.fn()}
        result={null}
        error={null}
        loading={false}
        submitDisabled={false}
        onSubmit={vi.fn()}
        onClose={vi.fn()}
      />,
    );

    expect(screen.getByText("common.posAbbr.LeftBack")).toBeInTheDocument();
    expect(screen.queryByText("common.posAbbr.Defender")).not.toBeInTheDocument();
  });
});
