import { render, screen } from "@testing-library/react";
import { describe, expect, it } from "vitest";

import MatchScreenLayout from "./MatchScreenLayout";

describe("MatchScreenLayout", () => {
  /** Given a fixed-height match screen, when it has a footer, then content yields space to it. */
  it("keeps the footer from shrinking inside the fixed-height screen", () => {
    render(
      <MatchScreenLayout showThemeToggle={false} footer={<footer>Match actions</footer>}>
        <main>Match content</main>
      </MatchScreenLayout>,
    );

    const footer = screen.getByRole("contentinfo");
    const content = screen.getByRole("main");
    const screenColumn = footer.parentElement?.parentElement;

    // jsdom has no flex layout, so assert the constraints that preserve footer space.
    expect(screenColumn).toHaveClass("h-dvh", "overflow-hidden", "flex", "flex-col");
    expect(footer.parentElement).toHaveClass("shrink-0");
    expect(content.parentElement).toHaveClass("min-h-0", "flex-1");
    expect(content.parentElement?.parentElement).toBe(screenColumn);
  });
});
