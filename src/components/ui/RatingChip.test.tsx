import { render, screen } from "@testing-library/react";
import { describe, expect, it, vi } from "vitest";

import { RatingChip } from "./RatingChip";

vi.mock("react-i18next", () => ({
  useTranslation: () => ({
    t: (key: string, params?: Record<string, string | number>) => {
      if (key === "common.overallRatingTier")
        return `Overall ${params?.value}, tier ${params?.tier}`;
      if (key === "common.overallRating") return `Overall ${params?.value}`;
      if (key === "common.namedRatingTier")
        return `${params?.name} ${params?.value}, tier ${params?.tier}`;
      if (key === "common.namedRating") return `${params?.name} ${params?.value}`;
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

  /**
   * Given tiers that no cut-off of the chip's own could give (60 put in S, 90 left in the rest:
   * the backend's table decides, e.g. for a league's own scale)
   * When the chips are drawn
   * Then each paints and names the tier it was given, not one worked out from its number.
   */
  it("paints and names the tier it is given, whatever its number", () => {
    render(
      <>
        <RatingChip value={60} tier="S" />
        <RatingChip value={90} tier="Rest" />
      </>,
    );
    const high = screen.getByText("Overall 60, tier S").closest("[title]");
    expect(high).toHaveClass("bg-tone-top-tint", "text-tone-top");
    const plain = screen.getByText("Overall 90").closest("[title]");
    expect(plain).toHaveClass("bg-sunken", "text-fg-muted");
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

  /**
   * Given a shooting rating of 74 in tier A, and a composure of 50 below every tier
   * When each chip is drawn with the attribute's name
   * Then it names that attribute, not the overall rating, with the tier where there is one.
   */
  it("names the rating it shows when it is not the overall", () => {
    render(
      <>
        <RatingChip value={74} tier="A" name="Shooting" />
        <RatingChip value={50} tier="Rest" name="Composure" />
      </>,
    );
    expect(screen.getByTitle("Shooting 74, tier A")).toHaveClass("bg-tone-good-tint");
    expect(screen.getByText("Shooting 74, tier A")).toBeInTheDocument();
    expect(screen.getByTitle("Composure 50")).toHaveClass("bg-sunken");
    expect(screen.queryByText(/Overall/)).not.toBeInTheDocument();
  });

  /** Given the larger size, then the chip is the taller one; the default is the row's size. */
  it("comes in two sizes", () => {
    const { rerender } = render(<RatingChip value={79} tier="A" />);
    expect(screen.getByTitle("Overall 79, tier A")).toHaveClass("h-5");
    rerender(<RatingChip value={79} tier="A" size="md" />);
    expect(screen.getByTitle("Overall 79, tier A")).toHaveClass("h-6");
  });
});
