import { fireEvent, render, screen } from "@testing-library/react";
import { describe, expect, it, vi } from "vitest";

import { createPlayer } from "../../test-utils/factories";
import TacticsCompareDialog from "./TacticsCompareDialog";

vi.mock("react-i18next", () => ({
  initReactI18next: { type: "3rdParty", init: () => undefined },
  useTranslation: () => ({
    t: (key: string, options?: Record<string, unknown>) =>
      options?.name ? `${key}|${String(options.name)}` : key,
    i18n: { language: "en" },
  }),
}));

// The rating tiers and position groups are asked of the backend; none answers in a test.
vi.mock("@tauri-apps/api/core", () => ({ invoke: vi.fn(() => Promise.resolve([])) }));

describe("TacticsCompareDialog", () => {
  /**
   * Given two players picked for a swap on the tactics board
   * When the comparison opens, and the player presses Escape
   * Then it is a named dialog with the focus inside it (it used to leave the focus on the board
   *      behind it, with no Escape and no Tab trap), and Escape closes it
   */
  it("is a named dialog that takes the focus and closes on Escape", () => {
    const onClose = vi.fn();
    render(
      <TacticsCompareDialog
        selectedPlayer={createPlayer({ id: "a", match_name: "A. One" })}
        comparePlayer={createPlayer({ id: "b", match_name: "B. Two" })}
        canConfirmSwap
        onConfirmSwap={vi.fn()}
        onClose={onClose}
      />,
    );

    const dialog = screen.getByRole("dialog", { name: "tactics.inspector" });
    expect(dialog).toHaveAttribute("aria-modal", "true");
    expect(dialog).toContainElement(document.activeElement as HTMLElement);

    fireEvent.keyDown(window, { key: "Escape" });
    expect(onClose).toHaveBeenCalledOnce();
  });

  /**
   * Given the comparison is open
   * When the player looks for its close button
   * Then it is named, though it is only an icon
   */
  it("names its icon-only close button", () => {
    render(
      <TacticsCompareDialog
        selectedPlayer={createPlayer({ id: "a" })}
        comparePlayer={createPlayer({ id: "b" })}
        canConfirmSwap={false}
        onConfirmSwap={vi.fn()}
        onClose={vi.fn()}
      />,
    );

    expect(screen.getByRole("button", { name: "common.close" })).toBeInTheDocument();
  });

  /**
   * Given a swap the backend rules out because the bench player is injured or suspended
   * When the comparison is open
   * Then Confirm swap is off, and the reason beside it names that player, tied to the button
   */
  it("says why Confirm swap is off for an unavailable bench player", () => {
    render(
      <TacticsCompareDialog
        selectedPlayer={createPlayer({ id: "a", match_name: "A. One" })}
        comparePlayer={createPlayer({ id: "b", match_name: "B. Two" })}
        canConfirmSwap={false}
        swapBlockedReason={{ kind: "unavailable", playerId: "b" }}
        onConfirmSwap={vi.fn()}
        onClose={vi.fn()}
      />,
    );

    const confirm = screen.getByRole("button", { name: "tactics.confirmSwap" });
    expect(confirm).toBeDisabled();
    const reason = screen.getByText("tactics.swapBlocked.unavailable|B. Two");
    expect(confirm).toHaveAccessibleDescription("tactics.swapBlocked.unavailable|B. Two");
    expect(reason).toBeInTheDocument();
    expect(screen.queryByText("tactics.compareSelectionHint")).not.toBeInTheDocument();
  });

  /**
   * Given a swap that would change nothing, and one whose bench player's verdict has not come yet
   * When each comparison is open
   * Then each says its own reason, and neither says the player is unavailable
   */
  it.each([
    [{ kind: "unchanged" } as const, "tactics.swapBlocked.unchanged"],
    [{ kind: "unchecked", playerId: "a" } as const, "tactics.swapBlocked.unchecked|A. One"],
  ])("says the reason %j for a swap that is off", (reason, text) => {
    render(
      <TacticsCompareDialog
        selectedPlayer={createPlayer({ id: "a", match_name: "A. One" })}
        comparePlayer={createPlayer({ id: "b", match_name: "B. Two" })}
        canConfirmSwap={false}
        swapBlockedReason={reason}
        onConfirmSwap={vi.fn()}
        onClose={vi.fn()}
      />,
    );

    expect(screen.getByText(text)).toBeInTheDocument();
    expect(screen.queryByText(/swapBlocked.unavailable/)).not.toBeInTheDocument();
  });

  /**
   * Given a swap that can be confirmed
   * When the comparison is open
   * Then the usual hint stands, no reason is shown, and Confirm swap is on
   */
  it("shows the hint, not a reason, when the swap can be confirmed", () => {
    render(
      <TacticsCompareDialog
        selectedPlayer={createPlayer({ id: "a" })}
        comparePlayer={createPlayer({ id: "b" })}
        canConfirmSwap
        swapBlockedReason={null}
        onConfirmSwap={vi.fn()}
        onClose={vi.fn()}
      />,
    );

    expect(screen.getByText("tactics.compareSelectionHint")).toBeInTheDocument();
    expect(screen.queryByText(/swapBlocked/)).not.toBeInTheDocument();
    expect(screen.getByRole("button", { name: "tactics.confirmSwap" })).toBeEnabled();
  });
});
