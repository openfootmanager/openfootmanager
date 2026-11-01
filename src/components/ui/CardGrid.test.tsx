import { render, screen } from "@testing-library/react";
import { describe, expect, it } from "vitest";

import { CardGrid } from "./CardGrid";

describe("CardGrid", () => {
  /**
   * Given two cards in the grid
   * When it is drawn
   * Then they are the items of one list, and the grid is one column until the screen is wide
   *      enough for the 20rem columns (so a narrow container does not scroll sideways).
   */
  it("is a list that is one column on a narrow screen", () => {
    render(
      <CardGrid>
        <li>one</li>
        <li>two</li>
      </CardGrid>,
    );

    const list = screen.getByRole("list");
    expect(screen.getAllByRole("listitem")).toHaveLength(2);
    expect(list).toHaveClass("grid-cols-1");
    expect(list.className).toMatch(/sm:grid-cols-\[repeat\(auto-fill,minmax\(20rem,1fr\)\)\]/);
  });
});
