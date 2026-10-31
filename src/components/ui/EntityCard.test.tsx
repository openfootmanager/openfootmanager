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
