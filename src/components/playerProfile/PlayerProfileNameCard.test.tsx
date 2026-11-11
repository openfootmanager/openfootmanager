import { fireEvent, render, screen, within } from "@testing-library/react";
import { beforeAll, describe, expect, it, vi } from "vitest";

import { i18nReady } from "../../i18n";
import { ratingTierClass } from "../../lib/ratingTier";
import type { StaffData } from "../../store/gameStore";
import { createPlayer, createTeam } from "../../test-utils/factories";
import PlayerProfileNameCard from "./PlayerProfileNameCard";

vi.mock("../../hooks/useRatingTiers", async () => {
  const { cannedRatingTiersModule } = await import("../../test-utils/ratingTiers");
  return cannedRatingTiersModule();
});

beforeAll(async () => {
  await i18nReady;
});

function t(key: string, options?: Record<string, string | number>): string {
  if (key === "playerProfile.overall") return "Overall";
  if (key === "common.age") return "Age";
  return options ? `${key}:${JSON.stringify(options)}` : key;
}

const NO_SCOUT = {
  scouts: [],
  availableScout: null,
  alreadyScouting: false,
  allBusy: false,
  canScout: false,
};

function renderCard(
  overrides: {
    ovr?: number | null;
    face?: string;
    isOwnClub?: boolean;
    /** A scout is free to be sent: the Scout button shows, not the "no scouts" hint. */
    scoutFree?: boolean;
    alternates?: string[];
    onSelectTeam?: (id: string) => void;
  } = {},
) {
  const {
    ovr = 77,
    face,
    isOwnClub = true,
    scoutFree = false,
    alternates = [],
    onSelectTeam = () => {},
  } = overrides;
  const scout = { id: "scout-1" } as StaffData;
  const availability = scoutFree
    ? {
        scouts: [scout],
        availableScout: scout,
        alreadyScouting: false,
        allBusy: false,
        canScout: true,
      }
    : NO_SCOUT;
  const player = createPlayer({
    ovr: 77,
    media: face ? { face } : undefined,
    alternate_positions: alternates,
    team_id: "team-1",
  });
  return render(
    <PlayerProfileNameCard
      player={player}
      ovr={ovr}
      primaryPosition="Midfielder"
      age={22}
      teamName="Brussels FC"
      footednessLabel="Right"
      weakFootValue={4}
      language="en"
      isOwnClub={isOwnClub}
      scoutAvailability={availability}
      scoutStatus="idle"
      scoutError={null}
      onScout={() => {}}
      onSelectTeam={onSelectTeam}
      team={createTeam({ id: "team-1", colors: { primary: "#991b1b", secondary: "#ffffff" } })}
      actions={<button type="button">Actions</button>}
      t={t}
    />,
  );
}

describe("PlayerProfileNameCard", () => {
  /**
   * Given a player whose rating the club does not know
   * When the name card is drawn
   * Then the rating reads "–" with no tier tone, and no number of his appears
   */
  it("shows a dash for a rating the club does not know", () => {
    renderCard({ ovr: null });

    expect(screen.getByText("–")).toBeInTheDocument();
    expect(screen.queryByText("77")).not.toBeInTheDocument();
  });

  /**
   * Given a player whose rating the club does not know
   * When a screen reader reads the name card
   * Then it is told why there is no number, as text and not only as a tooltip, and the dash alone
   *      is not read out
   */
  it("says in words, not only in a tooltip, that the club does not know the rating", () => {
    renderCard({ ovr: null });

    const explanation = screen.getByText("playerProfile.ratingUnknown");
    expect(explanation).toHaveClass("sr-only");
    expect(screen.getByText("–")).toHaveAttribute("aria-hidden", "true");
  });

  /**
   * Given a player with a face picture, and one without
   * When the name card is drawn for each
   * Then his rating is on screen once, labelled "Overall", beside the picture or the initials
   */
  it("shows the rating once, labelled, with or without a face picture", () => {
    const { unmount } = renderCard({ face: "assets/faces/fraser.png" });
    expect(screen.getByRole("img", { name: "John Smith" })).toBeInTheDocument();
    expect(screen.getAllByText("77")).toHaveLength(1);
    expect(screen.getByText("Overall")).toBeInTheDocument();
    unmount();

    renderCard();
    expect(screen.getAllByText("77")).toHaveLength(1);
  });

  /**
   * Given a player rated 76 and another rated 77, either side of the A-tier cut-off
   * When the name card shows the rating
   * Then the chip takes the tone of the backend's tier (B, then A)
   */
  it("tints the rating chip by its backend tier", () => {
    const chipOf = (ovr: string) => screen.getByText(ovr).closest("span[title]");

    const { unmount } = renderCard({ ovr: 76 });
    expect(chipOf("76")).toHaveClass(...ratingTierClass("B", "badge").split(" "));
    unmount();

    renderCard({ ovr: 77 });
    expect(chipOf("77")).toHaveClass(...ratingTierClass("A", "badge").split(" "));
  });

  /**
   * Given a midfielder who can also play wide and up front
   * When the name card is drawn
   * Then his natural position is a badge and each alternate is another, outlined, beside the name
   */
  it("shows the natural position and each alternate as badges", () => {
    renderCard({ alternates: ["Forward", "Defender"] });

    const card = screen.getByRole("region", { name: "John Smith" });
    // The badge inside each titled wrapper: the natural one is filled with its group's colour, an
    // alternate is the outlined variant.
    const badgeOf = (name: string) => within(card).getByTitle(name).firstElementChild;
    expect(badgeOf("Midfielder")).not.toHaveClass("bg-transparent");
    expect(badgeOf("Forward")).toHaveClass("bg-transparent");
    expect(badgeOf("Defender")).toHaveClass("bg-transparent");
    expect(within(card).queryByTitle("Goalkeeper")).not.toBeInTheDocument();
  });

  /**
   * Given the name card of a player the manager can scout, in a narrow window
   * When the card is drawn
   * Then the scouting button, the Actions menu and the rating are one group: they wrap together
   *      under the identity, so the rating never drops to the left under the avatar on its own
   *      (the 960×640 shot of a rival's profile)
   */
  it("keeps the scouting button, the Actions menu and the rating in one group", () => {
    renderCard({ isOwnClub: false, scoutFree: true });

    const group = screen.getByRole("button", { name: "Actions" }).parentElement;
    expect(group).toContainElement(screen.getByRole("button", { name: "Scout" }));
    expect(group).toContainElement(screen.getByText("Overall"));
    expect(group).not.toContainElement(screen.getByRole("heading", { name: "John Smith" }));
  });

  /**
   * Given an active player at a club the manager can open
   * When the meta line is read
   * Then his club is separated from his age by the same dot as the rest ("Brussels FC · Age 22"),
   *      not run into it
   */
  it("separates a linked club from his age", () => {
    renderCard({ onSelectTeam: () => {} });

    const meta = screen.getByRole("button", { name: "Brussels FC" }).closest("p");
    expect(meta).toHaveTextContent(/Brussels FC\s*·\s*Age 22/);
  });

  /**
   * Given a player at a club the manager can open
   * When his club's name is pressed
   * Then the club opens
   */
  it("opens the player's club from the meta line", () => {
    const onSelectTeam = vi.fn();
    renderCard({ onSelectTeam });

    const club = screen.getByRole("button", { name: "Brussels FC" });
    // Its ring is offset from the kit wash, so it shows on any club's colours.
    expect(club).toHaveClass("focus-visible:ring-offset-2", "focus-visible:ring-offset-page");
    fireEvent.click(club);

    expect(onSelectTeam).toHaveBeenCalledWith("team-1");
  });

  /**
   * Given a player of the manager's own club, and one of a rival's whom he cannot yet scout
   * When the name card is drawn
   * Then no band of his condition, morale, value or wage is under it
   */
  it("carries no status band of condition, morale, value or wage", () => {
    renderCard();

    for (const key of ["common.condition", "common.morale", "common.value", "common.wage"]) {
      expect(screen.queryByText(key)).not.toBeInTheDocument();
    }
  });

  /**
   * Given the club's kit colours
   * When the name card is drawn
   * Then the kit is the card's stripe and wash, through its variables
   */
  it("takes its stripe and wash from the club's kit", () => {
    renderCard();

    const card = screen.getByRole("region", { name: "John Smith" });
    expect(card.style.getPropertyValue("--kit")).toBe("#991b1b");
    expect(card).toHaveClass("border-l-kit");
  });
});
