import { fireEvent, render, screen } from "@testing-library/react";
import { describe, expect, it, vi } from "vitest";

import { formatExactMoney } from "../../lib/helpers";
import type { VisibleWage } from "../../services/scoutingService";
import { createPlayer, createTeam } from "../../test-utils/factories";
import LoanOfferModal from "./LoanOfferModal";

// Writes a key with the wage it was given, so a test can read which wage reached the sentence.
vi.mock("react-i18next", () => ({
  useTranslation: () => ({
    t: (key: string, options?: { wage?: string }) =>
      options?.wage === undefined ? key : `${key}(${options.wage})`,
    i18n: { language: "en" },
  }),
}));

function renderLoanForm(visibleWage: VisibleWage, wageContributionPct: number) {
  render(
    <LoanOfferModal
      loanTarget={createPlayer({ wage: 12000 })}
      visibleWage={visibleWage}
      teams={[createTeam()]}
      periodId=""
      periodOptions={[]}
      selectedEndDate="2027-01-01"
      onPeriodChange={vi.fn()}
      wageContributionPct={wageContributionPct}
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
}

describe("the wage share a loan offer would cover", () => {
  /**
   * Given another club's player on 12,000 a week whom the club has not scouted
   * When the loan form says what share of his wage the club would cover
   * Then it writes "–" for the amount, and nothing derived from his real wage.
   */
  it("hides the amount when the club may not see his wage", () => {
    renderLoanForm({ kind: "hidden" }, 50);

    expect(screen.getByText("transfers.loanWageSummary(–)")).toBeInTheDocument();
    expect(screen.queryByText(new RegExp(formatExactMoney(6000)))).not.toBeInTheDocument();
  });

  /**
   * Given a scouted player whose wage the club sees as about 12,000 a week
   * When the loan form says what half of it would cost
   * Then the amount is half the visible wage, written as "about".
   */
  it("writes the share of a scouted player's wage as about", () => {
    renderLoanForm({ kind: "approximate", weekly: 12000 }, 50);

    expect(
      screen.getByText(
        `transfers.loanWageSummary(playerProfile.approximateWage(${formatExactMoney(6000)}))`,
      ),
    ).toBeInTheDocument();
  });

  /**
   * Given a player whose exact wage the club sees, 12,000 a week
   * When the loan form says what 75% of it would cost
   * Then the amount is the exact share, 9,000.
   */
  it("writes the exact share when the club sees his exact wage", () => {
    renderLoanForm({ kind: "exact", weekly: 12000 }, 75);

    expect(
      screen.getByText(`transfers.loanWageSummary(${formatExactMoney(9000)})`),
    ).toBeInTheDocument();
  });

  /**
   * Given a player whose wage the club may not see
   * When the loan form offers to cover none of it, 0%
   * Then the amount is zero: nothing of a hidden wage is still nothing, and it tells nothing.
   */
  it("writes a zero share at 0% even when the wage is hidden", () => {
    renderLoanForm({ kind: "hidden" }, 0);

    expect(
      screen.getByText(`transfers.loanWageSummary(${formatExactMoney(0)})`),
    ).toBeInTheDocument();
  });
});

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
        visibleWage={{ kind: "hidden" }}
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

  /**
   * Given the dialog is open over a button that had focus
   * When it opens, Tab is pressed on its last control, and then Escape
   * Then focus starts inside it, Tab wraps instead of leaving it, and Escape asks to close.
   */
  it("takes focus, traps Tab and closes on Escape", () => {
    const onClose = vi.fn();
    const opener = document.createElement("button");
    document.body.appendChild(opener);
    opener.focus();

    render(
      <LoanOfferModal
        loanTarget={createPlayer()}
        visibleWage={{ kind: "hidden" }}
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
        onClose={onClose}
      />,
    );

    const dialog = screen.getByRole("dialog");
    expect(dialog).toHaveFocus();
    const controls = dialog.querySelectorAll<HTMLElement>(
      "button:not([disabled]), input:not([disabled]), select:not([disabled])",
    );
    const last = controls[controls.length - 1];
    last.focus();
    fireEvent.keyDown(last, { key: "Tab" });
    expect(dialog.contains(document.activeElement)).toBe(true);
    expect(document.activeElement).toBe(controls[0]);

    fireEvent.keyDown(window, { key: "Escape" });

    expect(onClose).toHaveBeenCalledTimes(1);
    opener.remove();
  });
});
