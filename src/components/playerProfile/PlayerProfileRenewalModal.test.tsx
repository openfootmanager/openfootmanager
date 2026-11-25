import { render, screen } from "@testing-library/react";
import { describe, expect, it, vi } from "vitest";

import PlayerProfileRenewalModal from "./PlayerProfileRenewalModal";

const t = (key: string) => key;

function renderModal(renewalIsTerminal: boolean) {
  return render(
    <PlayerProfileRenewalModal
      show
      playerName="John Smith"
      t={t}
      renewalWage="50000"
      renewalLength="2"
      renewalIsTerminal={renewalIsTerminal}
      isRenewalWageValid
      renewalViolatesSoftCap={false}
      renewalProjection={null}
      renewalStatusMessage={null}
      renewalStatusClassName=""
      renewalCooledOff={false}
      renewalFeedback={null}
      renewalSubmitDisabled={false}
      delegateRenewalDisabled={false}
      onWageChange={vi.fn()}
      onLengthChange={vi.fn()}
      onClose={vi.fn()}
      onDelegate={vi.fn()}
      onSubmit={vi.fn()}
    />,
  );
}

describe("PlayerProfileRenewalModal", () => {
  /**
   * Given a renewal that has reached its end (accepted or refused for good)
   * When the dialog shows
   * Then both inputs are disabled, and say so by more than colour: the disabled attribute and a
   *      dimmed look.
   */
  it("dims both inputs when the renewal is terminal", () => {
    renderModal(true);

    for (const name of ["playerProfile.renewalWage", "playerProfile.renewalLength"]) {
      const input = screen.getByLabelText(name);
      expect(input).toBeDisabled();
      expect(input).toHaveClass("disabled:opacity-50");
    }
  });

  /**
   * Given a renewal still open
   * When the dialog shows
   * Then both inputs take input.
   */
  it("leaves both inputs enabled while the renewal is open", () => {
    renderModal(false);

    expect(screen.getByLabelText("playerProfile.renewalWage")).toBeEnabled();
    expect(screen.getByLabelText("playerProfile.renewalLength")).toBeEnabled();
  });
});
