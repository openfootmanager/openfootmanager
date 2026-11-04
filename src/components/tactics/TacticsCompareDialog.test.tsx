import { fireEvent, render, screen } from "@testing-library/react";
import { describe, expect, it, vi } from "vitest";

import { createPlayer } from "../../test-utils/factories";
import TacticsCompareDialog from "./TacticsCompareDialog";

vi.mock("react-i18next", () => ({
  useTranslation: () => ({ t: (key: string) => key, i18n: { language: "en" } }),
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
});
