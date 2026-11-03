import { render, screen } from "@testing-library/react";
import { describe, expect, it } from "vitest";

import { InjuryBadge } from "./InjuryBadge";

function badgeFor(days: number): HTMLElement {
  render(<InjuryBadge injury={{ name: "hamstring_strain", days_remaining: days }} />);
  return screen.getByRole("img");
}

describe("InjuryBadge", () => {
  /**
   * Given injuries of each severity (3 days or less, a week, two weeks, longer),
   * when the badge renders,
   * then each is painted with status tokens, warning for the short ones and danger for the long
   * ones, with a stronger border for the worse of each pair.
   */
  it.each([
    [2, ["bg-warning-tint", "text-warning", "border-line-strong"]],
    [5, ["bg-warning-tint", "text-warning", "border-warning-solid"]],
    [10, ["bg-danger-tint", "text-danger", "border-line-strong"]],
    [30, ["bg-danger-tint", "text-danger", "border-danger-solid"]],
  ])("paints a %i-day injury with status tokens", (days, classes) => {
    const badge = badgeFor(days);
    for (const name of classes) expect(badge).toHaveClass(name);
  });

  /** Given four injuries, one of each severity, then no two badges look the same. */
  it("gives every severity its own look", () => {
    const looks = [2, 5, 10, 30].map((days) => {
      const { unmount } = render(
        <InjuryBadge injury={{ name: "hamstring_strain", days_remaining: days }} />,
      );
      const look = screen.getByRole("img").className;
      unmount();
      return look;
    });
    expect(new Set(looks).size).toBe(4);
  });

  /** Given an injury, then the badge's name says what it is and how long is left, not only its colour. */
  it("names the injury and the days left", () => {
    // The test setup's t() returns keys, so the name shows which strings it is built from.
    expect(badgeFor(5)).toHaveAccessibleName("hamstring_strain - playerProfile.daysRemaining");
  });
});
