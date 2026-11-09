import { fireEvent, render, screen } from "@testing-library/react";
import { describe, expect, it, vi } from "vitest";

import { FiredLetter } from "./FiredModal";

vi.mock("react-i18next", () => ({
  initReactI18next: { type: "3rdParty", init: () => undefined },
  useTranslation: () => ({ t: (key: string) => key, i18n: { language: "en" } }),
}));

describe("FiredLetter", () => {
  /**
   * Given the board has sacked the manager
   * When its letter opens, and the player presses Escape
   * Then it is a dialog named by its heading with the focus inside it (it used to leave the focus
   *      on the page behind, which a keyboard player could tab through but not see), and Escape
   *      does what Continue does
   */
  it("is a named dialog that takes the focus, and Escape continues", () => {
    const onContinue = vi.fn();
    render(<FiredLetter teamName="Glasgow FC" onContinue={onContinue} />);

    const dialog = screen.getByRole("dialog", { name: "sacked.title" });
    expect(dialog).toHaveAttribute("aria-modal", "true");
    expect(dialog).toContainElement(document.activeElement as HTMLElement);

    fireEvent.keyDown(window, { key: "Escape" });
    expect(onContinue).toHaveBeenCalledOnce();
  });

  /**
   * Given the letter is open
   * When the player presses Continue
   * Then the letter is dismissed the same way
   */
  it("continues from its button", () => {
    const onContinue = vi.fn();
    render(<FiredLetter teamName="Glasgow FC" onContinue={onContinue} />);

    fireEvent.click(screen.getByRole("button", { name: "dashboard.continue" }));
    expect(onContinue).toHaveBeenCalledOnce();
  });
});
