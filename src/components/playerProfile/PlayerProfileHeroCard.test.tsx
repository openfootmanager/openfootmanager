import { render, screen } from "@testing-library/react";
import { describe, expect, it, vi } from "vitest";

import { ratingTierClass } from "../../lib/ratingTier";
import { createPlayer } from "../../test-utils/factories";
import PlayerProfileHeroCard from "./PlayerProfileHeroCard";

vi.mock("../../hooks/useRatingTiers", async () => {
  const { cannedRatingTiersModule } = await import("../../test-utils/ratingTiers");
  return cannedRatingTiersModule();
});

function t(key: string): string {
  return key === "common.ovr" ? "OVR" : key;
}

function renderHero(face?: string) {
  const player = createPlayer({ ovr: 77, media: face ? { face } : undefined });

  render(
    <PlayerProfileHeroCard
      player={player}
      ovr={77}
      primaryPosition="Midfielder"
      age={22}
      teamName="Brussels FC"
      footednessLabel="Right"
      weakFootValue={4}
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
      onScout={() => {}}
      t={t}
    />,
  );
}

describe("PlayerProfileHeroCard", () => {
  /**
   * Given a player whose face picture is shown in the hero
   * When his profile opens
   * Then his OVR is still on screen, labelled, and not hidden behind the picture
   */
  it("shows the OVR of a player who has a face picture", () => {
    renderHero("assets/faces/fraser.png");

    expect(screen.getByRole("img", { name: "John Smith" })).toBeInTheDocument();
    expect(screen.getByText("OVR")).toBeInTheDocument();
    expect(screen.getByText("77")).toBeInTheDocument();
  });

  /**
   * Given a player with no face picture
   * When his profile opens
   * Then his OVR appears once, in its own labelled tile, and the avatar does not repeat it
   */
  it("shows the OVR once for a player without a face picture", () => {
    renderHero();

    expect(screen.getByText("OVR")).toBeInTheDocument();
    expect(screen.getAllByText("77")).toHaveLength(1);
  });
});

function renderHeroAtOvr(ovr: number) {
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

    const { unmount } = renderHeroAtOvr(76);
    expect(frameOf("76")).toHaveClass(...ratingTierClass("B", "hero").split(" "));
    unmount();

    renderHeroAtOvr(77);
    expect(frameOf("77")).toHaveClass(...ratingTierClass("A", "hero").split(" "));
  });
});
