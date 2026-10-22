import { render, screen } from "@testing-library/react";
import { describe, expect, it, vi } from "vitest";

import { RatingChip } from "./RatingChip";

vi.mock("react-i18next", () => ({
  useTranslation: () => ({
    t: (key: string, params?: Record<string, string | number>) => {
      if (key === "common.overallRatingTier")
        return `Overall ${params?.value}, tier ${params?.tier}`;
      if (key === "common.overallRating") return `Overall ${params?.value}`;
      if (key.startsWith("common.ratingTier.")) return key.slice("common.ratingTier.".length);
      return key;
    },
  }),
}));

describe("RatingChip", () => {
  /**
   * Given a rating of 79 that the backend puts in tier A
   * When the chip is drawn
   * Then it shows the number alone, painted in tier A's tone, and says the rating and its tier
   * to a screen reader and as a tooltip.
   */
  it("shows the number in its tier's tone, and names the rating and the tier", () => {
    render(<RatingChip value={79} tier="A" />);
    const chip = screen.getByText("Overall 79, tier A").closest("[title]");
    expect(chip).toHaveAttribute("title", "Overall 79, tier A");
    expect(chip).toHaveClass("bg-tone-good-tint", "text-tone-good");
    expect(chip?.querySelector("[aria-hidden='true']")).toHaveTextContent(/^79$/);
  });

  /** Given a rating below every tier, then the chip is neutral and names the rating alone. */
  it("is neutral and names no tier for a rating below the tiers", () => {
    render(<RatingChip value={58} tier="Rest" />);
    const chip = screen.getByText("Overall 58").closest("[title]");
    expect(chip).toHaveAttribute("title", "Overall 58");
    expect(chip).toHaveClass("bg-sunken", "text-fg-muted");
  });

  /** Given a fractional rating, then the chip shows the whole number the backend's tiers use. */
  it("shows a rating as a whole number", () => {
    render(<RatingChip value={78.6} tier="A" />);
    expect(screen.getByText("Overall 79, tier A")).toBeInTheDocument();
  });

  /** Given the larger size, then the chip is the taller one; the default is the row's size. */
  it("comes in two sizes", () => {
    const { rerender } = render(<RatingChip value={79} tier="A" />);
    expect(screen.getByTitle("Overall 79, tier A")).toHaveClass("h-5");
    rerender(<RatingChip value={79} tier="A" size="md" />);
    expect(screen.getByTitle("Overall 79, tier A")).toHaveClass("h-6");
  });
});
