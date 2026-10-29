import { render, screen } from "@testing-library/react";
import { describe, expect, it } from "vitest";

import { UnreadBadge } from "./UnreadBadge";

describe("UnreadBadge", () => {
  /**
   * Given three unread messages
   * When the badge is drawn
   * Then it is a circle: a fixed height, a minimum width equal to it, and tight side padding, so
   * one digit is as wide as it is tall and more digits widen it only by their own width.
   */
  it("is a circle for one digit", () => {
    render(<UnreadBadge count={3} />);
    const badge = screen.getByText("3");
    expect(badge).toHaveClass("h-5", "min-w-5", "px-1", "rounded-full");
  });

  /** Given more than 99 unread, then the badge reads 99+ instead of growing without end. */
  it("caps at 99+", () => {
    render(<UnreadBadge count={120} />);
    expect(screen.getByText("99+")).toBeInTheDocument();
  });

  /** Given a mark rather than a count (a match today), then it shows the mark in the same circle. */
  it("shows a mark in the same circle", () => {
    render(<UnreadBadge count="!" />);
    expect(screen.getByText("!")).toHaveClass("h-5", "min-w-5");
  });
});
