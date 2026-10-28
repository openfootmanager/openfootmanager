import { render, screen } from "@testing-library/react";
import { describe, expect, it, vi } from "vitest";

import { ratingTierClass } from "../../lib/ratingTier";
import { createPlayer } from "../../test-utils/factories";
import TacticsPlayerFocusPanel from "./TacticsPlayerFocusPanel";

vi.mock("../../hooks/useRatingTiers", async () => {
  const { cannedRatingTiersModule } = await import("../../test-utils/ratingTiers");
  return cannedRatingTiersModule();
});

vi.mock("react-i18next", () => ({
  useTranslation: () => ({ t: (key: string) => key, i18n: { language: "en" } }),
}));

describe("TacticsPlayerFocusPanel rating tiers", () => {
  /**
   * Given a player whose pace is 77 and stamina 76, either side of the A-tier cut-off, and every
   *       other attribute far below
   * When the inspector shows his attributes
   * Then each value takes the colour of its backend tier, in the number and in its bar.
   */
  it("colours each attribute by its backend tier", () => {
    const base = createPlayer();
    const player = createPlayer({
      attributes: {
        ...Object.fromEntries(Object.keys(base.attributes).map((attr) => [attr, 30])),
        pace: 77,
        stamina: 76,
      } as typeof base.attributes,
    });

    render(
      <TacticsPlayerFocusPanel
        canConfirmSwap={false}
        comparePlayer={null}
        onConfirmSwap={vi.fn()}
        selectedPlayer={player}
      />,
    );

    expect(screen.getByText("77")).toHaveClass(...ratingTierClass("A", "text").split(" "));
    expect(screen.getByText("76")).toHaveClass(...ratingTierClass("B", "text").split(" "));
    const barOf = (value: string) =>
      screen.getByText(value).parentElement?.querySelector(".h-full");
    expect(barOf("77")).toHaveClass(ratingTierClass("A", "bar"));
    expect(barOf("76")).toHaveClass(...ratingTierClass("B", "bar").split(" "));
  });

  /**
   * Given a selected player whose pace is 77 and a compared player whose pace is 76, every other
   *       attribute far below
   * When the inspector compares them
   * Then each side's value takes the colour of its backend tier, in the number and in its bar.
   */
  it("colours both sides of a comparison by their backend tier", () => {
    const base = createPlayer();
    const withPace = (id: string, pace: number) =>
      createPlayer({
        id,
        attributes: {
          ...Object.fromEntries(Object.keys(base.attributes).map((attr) => [attr, 30])),
          pace,
        } as typeof base.attributes,
      });

    render(
      <TacticsPlayerFocusPanel
        canConfirmSwap={false}
        comparePlayer={withPace("compared", 76)}
        onConfirmSwap={vi.fn()}
        selectedPlayer={withPace("selected", 77)}
      />,
    );

    expect(screen.getByText("77")).toHaveClass(...ratingTierClass("A", "text").split(" "));
    expect(screen.getByText("76")).toHaveClass(...ratingTierClass("B", "text").split(" "));
    expect(
      screen.getByText("77").parentElement?.parentElement?.querySelector(".h-full"),
    ).toHaveClass(ratingTierClass("A", "bar"));
    expect(
      screen.getByText("76").parentElement?.parentElement?.querySelector(".h-full"),
    ).toHaveClass(...ratingTierClass("B", "bar").split(" "));
  });
});
