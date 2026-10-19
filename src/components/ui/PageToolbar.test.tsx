import { render, screen } from "@testing-library/react";
import { describe, expect, it } from "vitest";

import { PageToolbar } from "./PageToolbar";

describe("PageToolbar", () => {
  /**
   * Given tabs, filters and a primary action,
   * when the toolbar renders,
   * then all three are there and the action comes last, at the toolbar's end.
   */
  it("puts the page's primary action last", () => {
    render(
      <PageToolbar
        start={<span>tabs</span>}
        filters={
          <label>
            Position <input />
          </label>
        }
        action={<button type="button">Make bid</button>}
      />,
    );
    const toolbar = screen.getByTestId("page-toolbar");
    const order = [...toolbar.querySelectorAll("span, label, button")].map((el) =>
      el.textContent?.trim(),
    );
    expect(order).toEqual(["tabs", "Position", "Make bid"]);
  });

  /**
   * Given tabs wider than the toolbar,
   * then their slot may shrink to the toolbar's width, so the tablist scrolls inside it rather
   * than pushing the page sideways, and keeps a full toolbar row's height when the filters wrap
   * below it, so the underline still sits on the row's bottom edge.
   */
  it("lets the tabs' slot shrink to the toolbar's width and keep a row's height", () => {
    render(<PageToolbar start={<span>tabs</span>} />);
    const slot = screen.getByText("tabs").parentElement;
    expect(slot).toHaveClass("min-w-0", "max-w-full", "min-h-toolbar");
  });

  /**
   * Given filters and an action that wrap onto a new line,
   * then their group packs to the right, so the action stays at the toolbar's right end.
   */
  it("keeps the action at the right end when the filters wrap", () => {
    render(<PageToolbar action={<button type="button">Hire</button>} />);
    const group = screen.getByRole("button", { name: "Hire" }).parentElement;
    expect(group).toHaveClass("ml-auto", "justify-end");
  });

  /** Given only an action, then the toolbar renders it on its own. */
  it("renders with only an action", () => {
    render(<PageToolbar action={<button type="button">Hire</button>} />);
    expect(screen.getByRole("button", { name: "Hire" })).toBeInTheDocument();
  });
});
