import { fireEvent, render, screen } from "@testing-library/react";
import { describe, expect, it, vi } from "vitest";

import PlayersPagination from "./PlayersPagination";

vi.mock("react-i18next", () => ({
  useTranslation: () => ({
    t: (key: string, params?: Record<string, string | number>) =>
      params ? `${key}:${Object.values(params).join(",")}` : key,
  }),
}));

describe("PlayersPagination", () => {
  /**
   * Given a pager with several pages
   * When it is drawn
   * Then its text and border take role tokens (so high contrast and dark mode reach them), with
   *      no fixed gray or navy palette colour.
   */
  it("draws its text and border with role tokens, not palette colours", () => {
    const { container } = render(
      <PlayersPagination page={2} pageSize={30} total={90} totalPages={3} onPage={vi.fn()} />,
    );
    const html = container.innerHTML;

    expect(container.firstElementChild).toHaveClass("border-line");
    expect(html).toContain("text-fg-muted");
    expect(html).not.toMatch(/(?:text|border|bg)-(?:gray|navy)-\d/);
    // Each of the four buttons shows where keyboard focus is.
    for (const button of screen.getAllByRole("button")) {
      expect(button).toHaveClass("focus-visible:ring-2", "focus-visible:ring-focus");
    }
  });

  /**
   * Given the middle page of three
   * When each of the four buttons is chosen
   * Then it asks for the first, previous, next and last page.
   */
  it("asks for the page each button names", () => {
    const onPage = vi.fn();
    render(<PlayersPagination page={2} pageSize={30} total={90} totalPages={3} onPage={onPage} />);

    fireEvent.click(screen.getByRole("button", { name: "common.firstPage" }));
    fireEvent.click(screen.getByRole("button", { name: "common.previousPage" }));
    fireEvent.click(screen.getByRole("button", { name: "common.nextPage" }));
    fireEvent.click(screen.getByRole("button", { name: "common.lastPage" }));

    expect(onPage.mock.calls.map(([page]) => page)).toEqual([1, 1, 3, 3]);
  });

  /**
   * Given a single page
   * When the pager is asked to draw
   * Then it draws nothing.
   */
  it("draws nothing for a single page", () => {
    const { container } = render(
      <PlayersPagination page={1} pageSize={30} total={5} totalPages={1} onPage={vi.fn()} />,
    );
    expect(container).toBeEmptyDOMElement();
  });

  /**
   * Given a pager on the page (the cards view) and one inside a panel (the table's card)
   * When their buttons are drawn
   * Then each focus ring is offset onto the colour it sits on: the page, or the panel.
   */
  it("offsets its focus ring onto the surface it sits on", () => {
    const { rerender } = render(
      <PlayersPagination page={2} pageSize={30} total={90} totalPages={3} onPage={vi.fn()} />,
    );
    for (const button of screen.getAllByRole("button")) {
      expect(button).toHaveClass("focus-visible:ring-offset-page");
    }

    rerender(
      <PlayersPagination
        page={2}
        pageSize={30}
        total={90}
        totalPages={3}
        onPage={vi.fn()}
        surface="panel"
      />,
    );
    for (const button of screen.getAllByRole("button")) {
      expect(button).toHaveClass("focus-visible:ring-offset-panel");
    }
  });
});
