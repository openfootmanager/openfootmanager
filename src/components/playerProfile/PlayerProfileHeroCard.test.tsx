import { render, screen } from "@testing-library/react";
import { describe, expect, it, vi } from "vitest";

import { ratingTierClass } from "../../lib/ratingTier";
import { createPlayer } from "../../test-utils/factories";
import PlayerProfileHeroCard from "./PlayerProfileHeroCard";

vi.mock("../../hooks/useRatingTiers", async () => {
  const { cannedRatingTiersModule } = await import("../../test-utils/ratingTiers");
  return cannedRatingTiersModule();
});

function renderHero(ovr: number) {
  return render(
    <PlayerProfileHeroCard
      player={createPlayer()}
      ovr={ovr}
      primaryPosition="Forward"
      age={25}
      teamName="Club"
      footednessLabel="Right"
      weakFootValue={3}
      language="en"
      isOwnClub
      scoutAvailability={{
        scouts: [],
        availableScout: null,
        alreadyScouting: false,
        allBusy: false,
        canScout: false,
      }}
      scoutStatus="idle"
      scoutError={null}
      onScout={vi.fn()}
      t={(key: string) => key}
    />,
  );
}

describe("PlayerProfileHeroCard rating tiers", () => {
  /**
   * Given a player rated 76 and another rated 77, either side of the A-tier cut-off
   * When the profile hero shows the rating in its avatar frame
   * Then the frame takes the tint of the backend tier (B and A).
   */
  it("tints the rating frame by its backend tier", () => {
    const frameOf = (ovr: string) => screen.getByText(ovr).closest(".rounded-2xl");

    const { unmount } = renderHero(76);
    expect(frameOf("76")).toHaveClass(...ratingTierClass("B", "hero").split(" "));
    unmount();

    renderHero(77);
    expect(frameOf("77")).toHaveClass(...ratingTierClass("A", "hero").split(" "));
  });
});
