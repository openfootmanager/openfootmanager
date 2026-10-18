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

  /** Given only an action, then the toolbar renders it on its own. */
  it("renders with only an action", () => {
    render(<PageToolbar action={<button type="button">Hire</button>} />);
    expect(screen.getByRole("button", { name: "Hire" })).toBeInTheDocument();
  });
});
