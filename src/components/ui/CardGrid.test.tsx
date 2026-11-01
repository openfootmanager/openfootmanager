import { render, screen } from "@testing-library/react";
import { describe, expect, it } from "vitest";

import { CardGrid } from "./CardGrid";

describe("CardGrid", () => {
  /**
   * Given two cards in the grid
   * When it is drawn
   * Then they are the items of one list, laid out in auto-filled tracks whose minimum is the
   *      smaller of 20rem and the grid's own width, so a container narrower than 20rem gets one
   *      full-width column instead of a sideways scroll.
   */
  it("is a list whose tracks never exceed a narrow container", () => {
    render(
      <CardGrid>
        <li>one</li>
        <li>two</li>
      </CardGrid>,
    );

    const list = screen.getByRole("list");
    expect(screen.getAllByRole("listitem")).toHaveLength(2);
    expect(list.className).toContain("grid-cols-[repeat(auto-fill,minmax(min(20rem,100%),1fr))]");
  });

  /**
   * Given the grid's layout rule
   * When it is read
   * Then it does not switch on the viewport (a `sm:` rule): the viewport is not the grid's width.
   */
  it("does not depend on the viewport width", () => {
    render(
      <CardGrid>
        <li>one</li>
      </CardGrid>,
    );

    expect(screen.getByRole("list").className).not.toMatch(/(?:^|\s)(?:sm|md|lg|xl):/);
  });
});
