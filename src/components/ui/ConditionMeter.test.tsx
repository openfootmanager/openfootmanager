import { render, screen } from "@testing-library/react";
import { describe, expect, it, vi } from "vitest";

import { ConditionMeter } from "./ConditionMeter";

vi.mock("react-i18next", () => ({
  useTranslation: () => ({
    t: (key: string) =>
      ({ "be.condition.tiring": "Tiring", "common.condition": "Condition" })[key] ?? key,
  }),
}));

describe("ConditionMeter", () => {
  /**
   * Given a player's condition and the backend's band for it
   * When it is drawn
   * Then it shows the band's word in the band's tone, beside a condition bar and the percentage.
   */
  it("shows the band's word in its tone beside the bar and the percentage", () => {
    render(
      <ConditionMeter
        value={64}
        status={{ band: "Tiring", label_key: "be.condition.tiring", tone: "fair" }}
      />,
    );
    expect(screen.getByText("Tiring")).toHaveClass("text-tone-fair");
    // The word is the band's non-colour signal, so a long one ("Cansándose") is never cut short.
    expect(screen.getByText("Tiring")).not.toHaveClass("truncate");
    expect(screen.getByText("Tiring")).toHaveClass("whitespace-nowrap");
    const bar = screen.getByRole("progressbar", { name: "Condition" });
    expect(bar).toHaveAttribute("aria-valuenow", "64");
    expect(bar.firstElementChild).toHaveClass("bg-tone-fair-solid");
    // The number as table C draws it, without "%"; the bar already gives it to a screen reader.
    expect(screen.getByText("64")).toHaveAttribute("aria-hidden", "true");
    expect(screen.queryByText("64%")).toBeNull();
  });

  /**
   * Given a condition whose band the backend hasn't sent yet (the squad's projection not fetched)
   * When it is drawn
   * Then the bar is neutral and no word is guessed from the number.
   */
  it("draws a neutral bar and no word without the backend's band", () => {
    const { container } = render(<ConditionMeter value={30} status={null} />);
    // No band word at all, not just not "Critical": only the number is drawn as text.
    expect(container.textContent).toBe("30");
    const bar = screen.getByRole("progressbar", { name: "Condition" });
    expect(bar.firstElementChild).toHaveClass("bg-fg-subtle");
    expect(screen.getByText("30")).toBeInTheDocument();
  });
});
