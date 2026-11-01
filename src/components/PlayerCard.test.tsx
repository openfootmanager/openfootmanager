import { fireEvent, render, screen, within } from "@testing-library/react";
import { describe, expect, it, vi } from "vitest";

import PlayerCard from "./PlayerCard";

vi.mock("react-i18next", () => ({
  useTranslation: () => ({
    t: (key: string, params?: Record<string, string | number>) =>
      params ? `${key}:${Object.values(params).join(",")}` : key,
    i18n: { language: "en" },
  }),
}));

const PORTRAIT = { id: "p1", full_name: "Diego Gonzalez", match_name: "Gonzalez" };

function renderCard(props: Partial<Parameters<typeof PlayerCard>[0]> = {}) {
  return render(
    <PlayerCard
      portrait={PORTRAIT}
      name="Diego Gonzalez"
      position="RightBack"
      nationality="AR"
      age={24}
      teamName="Rovers"
      ovr={79}
      tier="A"
      kit={{ primary: "#16a34a", secondary: "#ffffff" }}
      facts={[
        { label: "Value", value: "£2.5M" },
        { label: "Wage", value: "£10K" },
      ]}
      onOpen={vi.fn()}
      {...props}
    />,
  );
}

describe("PlayerCard", () => {
  /**
   * Given a player with a known rating
   * When his card is drawn
   * Then the card is named for him, its name opens his profile, and the meta line carries his
   *      position, his age and his club.
   */
  it("names the card for the player and carries position, age and club", () => {
    const onOpen = vi.fn();
    renderCard({ onOpen });
    const card = screen.getByRole("article", { name: "Diego Gonzalez" });
    fireEvent.click(within(card).getByRole("button", { name: "Diego Gonzalez" }));
    expect(onOpen).toHaveBeenCalledTimes(1);
    expect(within(card).getByText("Rovers")).toBeInTheDocument();
    expect(within(card).getByText("24")).toBeInTheDocument();
    // The bare number is named for a screen reader: "Age 24", not just "24".
    expect(within(card).getByText("common.age")).toBeInTheDocument();
  });

  /**
   * Given a rating the club is not allowed to see (null)
   * When the card is drawn
   * Then the hero is the hidden-rating dash and no rating chip is drawn.
   */
  it("shows a dash, not a chip, for a rating the club does not know", () => {
    renderCard({ ovr: null, tier: null });
    const card = screen.getByRole("article", { name: "Diego Gonzalez" });
    expect(within(card).getByText("–")).toBeInTheDocument();
    expect(within(card).queryByTitle(/common\.overallRating/)).toBeNull();
  });

  /**
   * Given a known rating in tier A
   * When the card is drawn
   * Then the hero is a rating chip that says the rating and its tier.
   */
  it("draws a known rating as a chip that names its tier", () => {
    renderCard({ ovr: 79, tier: "A" });
    const card = screen.getByRole("article", { name: "Diego Gonzalez" });
    expect(within(card).getByTitle(/^common\.overallRatingTier/)).toBeInTheDocument();
  });

  /**
   * Given a free agent (no club)
   * When the card is drawn
   * Then no club button is drawn; with a club and a handler, the club's name is a button that
   *      selects the club without opening the player.
   */
  it("makes the club a button that does not open the player", () => {
    const onOpen = vi.fn();
    const onSelectTeam = vi.fn();
    const { unmount } = renderCard({ onOpen, onSelectTeam });
    fireEvent.click(screen.getByRole("button", { name: "Rovers" }));
    expect(onSelectTeam).toHaveBeenCalledTimes(1);
    expect(onOpen).not.toHaveBeenCalled();
    unmount();

    renderCard({ teamName: null, onSelectTeam });
    expect(screen.queryByRole("button", { name: "Rovers" })).toBeNull();
  });

  /**
   * Given the two facts the list chose
   * When the card is drawn
   * Then both appear with their labels in the facts list.
   */
  it("shows the two facts the list passes", () => {
    renderCard();
    expect(screen.getByText("Wage")).toBeInTheDocument();
    expect(screen.getByText("£10K")).toBeInTheDocument();
  });

  /**
   * Given a card whose portrait is a square image
   * When it is drawn
   * Then the portrait is clipped to its circle (overflow hidden on the rounded wrapper).
   */
  it("clips the portrait to its circle", () => {
    renderCard();
    const circle = screen
      .getByRole("article", { name: "Diego Gonzalez" })
      .querySelector(".rounded-full");
    expect(circle).toHaveClass("overflow-hidden");
  });

  /**
   * Given the card, whose only nationality signal is the flag
   * When a screen reader reads it
   * Then the flag is named for its country, not hidden as decoration.
   */
  it("names the flag for his country", () => {
    renderCard({ nationality: "AR" });
    const card = screen.getByRole("article", { name: "Diego Gonzalez" });
    expect(within(card).getByRole("img", { name: "Argentina" })).toBeInTheDocument();
  });

  /**
   * Given the club button in a card whose header wears the kit's wash
   * When it takes keyboard focus
   * Then its ring sits off the wash, with the panel-coloured offset the name button has.
   */
  it("gives the club button a focus ring offset from the wash", () => {
    renderCard({ onSelectTeam: vi.fn() });
    expect(screen.getByRole("button", { name: "Rovers" })).toHaveClass(
      "focus-visible:ring-offset-2",
      "focus-visible:ring-offset-panel",
    );
  });
});
