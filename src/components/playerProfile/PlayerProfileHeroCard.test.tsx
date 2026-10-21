import { render, screen } from "@testing-library/react";
import { describe, expect, it } from "vitest";

import { createPlayer } from "../../test-utils/factories";
import PlayerProfileHeroCard from "./PlayerProfileHeroCard";

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
      traits={[]}
      showsPrivateState
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
