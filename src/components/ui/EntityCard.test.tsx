import { fireEvent, render, screen, within } from "@testing-library/react";
import { describe, expect, it, vi } from "vitest";

import { EntityCard } from "./EntityCard";

const KIT = { primary: "#16a34a", secondary: "#ffffff" };

function renderCard(props: Partial<Parameters<typeof EntityCard>[0]> = {}) {
  return render(
    <EntityCard
      kit={KIT}
      media={<span role="img" aria-label="Portrait of Gonzalez" />}
      name="Gonzalez"
      meta={<span role="img" aria-label="Australia" />}
      hero={<span>79</span>}
      status={<button type="button">Renew</button>}
      facts={[
        { label: "Value", value: "£2.5M" },
        { label: "Morale", value: "75" },
      ]}
      onOpen={vi.fn()}
      {...props}
    />,
  );
}

describe("EntityCard", () => {
  /**
   * Given a player's card
   * When it is drawn
   * Then it is an article named for him, whose name is the button that opens his profile; the
   * face, the meta line, the hero number, the status and the facts are all outside that button,
   * so a screen reader reads each of them instead of one long button name.
   */
  it("names the card by its name button and keeps everything else readable outside it", () => {
    const onOpen = vi.fn();
    renderCard({ onOpen });
    const card = screen.getByRole("article", { name: "Gonzalez" });
    const open = within(card).getByRole("button", { name: "Gonzalez" });
    fireEvent.click(open);
    expect(onOpen).toHaveBeenCalledTimes(1);

    for (const name of ["Portrait of Gonzalez", "Australia"]) {
      expect(within(card).getByRole("img", { name }).closest("button")).toBeNull();
    }
    expect(within(card).getByText("Value")).toBeInTheDocument();
    expect(within(card).getByText("£2.5M")).toBeInTheDocument();
  });

  /**
   * Given a card with a control of its own in its status (a renewal button)
   * When the player clicks the card's body, or that control
   * Then a click on the body opens the profile, as a pointer convenience, and a click on the
   * control is the control's alone.
   */
  it("opens from a click on the card, but not from a control inside it", () => {
    const onOpen = vi.fn();
    renderCard({ onOpen });
    fireEvent.click(screen.getByText("£2.5M"));
    expect(onOpen).toHaveBeenCalledTimes(1);
    fireEvent.click(screen.getByRole("button", { name: "Renew" }));
    expect(onOpen).toHaveBeenCalledTimes(1);
  });

  /**
   * Given a card whose status holds a control that is one by ARIA role, not a native button
   * When the player clicks that control
   * Then the click is the control's alone, and the card does not open.
   */
  it("keeps a click on a role-based control from opening the card", () => {
    const onOpen = vi.fn();
    renderCard({
      onOpen,
      status: (
        <div role="button" tabIndex={0}>
          Shortlist
        </div>
      ),
    });
    fireEvent.click(screen.getByRole("button", { name: "Shortlist" }));
    expect(onOpen).not.toHaveBeenCalled();
  });

  /** Given a hero number of 0 (a new player's rating, a count), then it is drawn, not dropped. */
  it("draws a hero of 0", () => {
    renderCard({ hero: 0 });
    expect(screen.getByText("0")).toBeInTheDocument();
  });

  /** Given a card that opens nothing, then its name is a heading's plain text, not a button. */
  it("draws no button when the card opens nothing", () => {
    renderCard({ onOpen: undefined });
    expect(screen.queryByRole("button", { name: "Gonzalez" })).toBeNull();
    expect(screen.getByRole("heading", { name: "Gonzalez" })).toBeInTheDocument();
  });

  /**
   * Given the compact size (a message, a side panel)
   * When it is drawn
   * Then it keeps the identity row only (face, name, meta, hero), without the status or facts.
   */
  it("draws only the identity row when compact", () => {
    renderCard({ size: "compact" });
    const card = screen.getByRole("article", { name: "Gonzalez" });
    expect(within(card).getByText("79")).toBeInTheDocument();
    expect(within(card).queryByText("Value")).toBeNull();
    expect(within(card).queryByRole("button", { name: "Renew" })).toBeNull();
    // A compact name is lead-sized text, so it keeps its own case; only a heading-size name is set
    // in capitals.
    expect(within(card).getByRole("heading", { name: "Gonzalez" })).not.toHaveClass("uppercase");
  });

  /**
   * Given a full-size card that opens
   * When its name button takes keyboard focus
   * Then it shows the full focus ring, offset from the kit-washed header behind it, and the name
   * is set in capitals at heading size.
   */
  it("draws the full focus ring on the name button, clear of the kit wash", () => {
    renderCard({ onOpen: vi.fn() });
    const open = screen.getByRole("button", { name: "Gonzalez" });
    expect(open).toHaveClass(
      "focus-visible:ring-2",
      "focus-visible:ring-focus",
      "focus-visible:ring-offset-2",
      "focus-visible:ring-offset-panel",
    );
    expect(screen.getByRole("heading", { name: "Gonzalez" })).toHaveClass("uppercase");
  });

  /**
   * Given a card whose meta line or hero carries text only a screen reader reads ("Age",
   *       "Position"), which `sr-only` places absolutely
   * When the card is drawn
   * Then the card itself is the positioned box that text sits in, in both sizes, so the text stays
   *      inside the card's clipped bounds. Otherwise it falls back to the nearest positioned
   *      ancestor, outside the card, and stretches the page: a 1280×800 Teams page grew to 852px
   *      tall and scrolled. jsdom does no layout, so the class is what can be checked here.
   */
  it("is the positioned box its screen-reader-only text sits in, in both sizes", () => {
    renderCard({ hero: <span className="sr-only">Position </span> });
    expect(screen.getByRole("article", { name: "Gonzalez" })).toHaveClass(
      "relative",
      "overflow-hidden",
    );
    renderCard({ name: "Compact", size: "compact" });
    expect(screen.getByRole("article", { name: "Compact" })).toHaveClass(
      "relative",
      "overflow-hidden",
    );
  });

  /**
   * Given a club's kit colours
   * When the card is drawn
   * Then it carries them as the --kit variables the kit tokens read, and its stripe, header wash
   * and hairline use those tokens; without a kit it carries none, and falls back to neutral.
   */
  it("paints the club's kit through the kit tokens, and nothing without a kit", () => {
    const { unmount } = renderCard();
    const card = screen.getByRole("article", { name: "Gonzalez" });
    expect(card.style.getPropertyValue("--kit")).toBe("#16a34a");
    expect(card.style.getPropertyValue("--kit-2")).toBe("#ffffff");
    expect(card).toHaveClass("border-l-kit");
    expect(card.querySelector("header")).toHaveClass("bg-kit-wash", "kit-hairline", "border-b-kit");
    unmount();

    renderCard({ kit: null });
    expect(screen.getByRole("article", { name: "Gonzalez" }).style.getPropertyValue("--kit")).toBe(
      "",
    );
  });
});
