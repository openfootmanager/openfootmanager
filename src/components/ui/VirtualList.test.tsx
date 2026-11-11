import { useMemo } from "react";
import { fireEvent, render, screen, waitFor } from "@testing-library/react";
import { describe, expect, it, vi } from "vitest";

import { VirtualList } from "./VirtualList";

// jsdom has no layout: the scroller reports no height, so the list assumes a viewport of 800 px
// (13 rows at a 64 px step) and the rows around it.
const STEP = 64;

interface HarnessProps {
  count?: number;
  revealIndex?: number | null;
  resetKey?: unknown;
  onActivate?: (index: number) => void;
}

function Harness({ count = 10_000, revealIndex, resetKey, onActivate }: HarnessProps) {
  const items = useMemo(() => Array.from({ length: count }, (_, i) => i), [count]);
  return (
    <div data-testid="scroller" data-scroll-container="" style={{ height: 400, overflow: "auto" }}>
      <VirtualList
        items={items}
        rowHeight={56}
        gap={8}
        itemKey={(n) => n}
        renderItem={(n) => <button type="button">row {n}</button>}
        aria-label="Rows"
        revealIndex={revealIndex}
        resetKey={resetKey}
        onActivate={onActivate}
      />
    </div>
  );
}

const scroller = () => screen.getByTestId("scroller");
const item = (index: number) =>
  document.querySelector<HTMLElement>(`[data-vindex="${index}"]`) as HTMLElement;

async function scrollToRow(row: number) {
  scroller().scrollTop = row * STEP;
  fireEvent.scroll(scroller());
  await waitFor(() => expect(item(row)).not.toBeNull());
}

describe("VirtualList", () => {
  /**
   * Given a list of 10,000 rows
   * When it is rendered
   * Then only the rows around the viewport are in the page, not all of them
   */
  it("renders a window of the rows, not the whole list", () => {
    render(<Harness />);

    const rendered = screen.getAllByRole("listitem").length;
    expect(rendered).toBeGreaterThan(10);
    expect(rendered).toBeLessThan(40);
    expect(screen.getByText("row 0")).toBeInTheDocument();
    expect(screen.queryByText("row 9999")).toBeNull();
  });

  /**
   * Given a list of 10,000 rows with only some in the page
   * When a screen reader reads a row
   * Then it is told its place in the whole list: row N of 10,000
   */
  it("tells assistive technology each row's place in the whole list", () => {
    render(<Harness />);

    expect(screen.getByRole("list", { name: "Rows" })).toBeInTheDocument();
    expect(item(0)).toHaveAttribute("aria-setsize", "10000");
    expect(item(0)).toHaveAttribute("aria-posinset", "1");
    expect(item(3)).toHaveAttribute("aria-posinset", "4");
  });

  /**
   * Given the list scrolled far down
   * When the scroller moves
   * Then the rows there are rendered and the rows left behind are gone
   */
  it("moves the window with the scroll", async () => {
    render(<Harness />);

    await scrollToRow(5_000);

    expect(screen.getByText("row 5000")).toBeInTheDocument();
    expect(screen.queryByText("row 0")).toBeNull();
    expect(screen.getAllByRole("listitem").length).toBeLessThan(40);
  });

  /**
   * Given a row with keyboard focus
   * When the list is scrolled until that row is far outside the window
   * Then the row stays in the page and keeps focus: scrolling must not drop the user's place
   */
  it("keeps the focused row mounted while it is scrolled out of the window", async () => {
    render(<Harness />);
    const focused = item(3);
    focused.focus();
    expect(document.activeElement).toBe(focused);

    await scrollToRow(5_000);

    expect(item(3)).toBe(focused);
    expect(document.activeElement).toBe(focused);
  });

  /**
   * Given a long list
   * When it is read for its tab stops
   * Then exactly one row is in the tab order: arrow keys move among the rest
   */
  it("has one tab stop, which follows the focused row", () => {
    render(<Harness />);
    expect(screen.getAllByRole("listitem").filter((row) => row.tabIndex === 0)).toHaveLength(1);

    fireEvent.keyDown(item(0), { key: "ArrowDown" });

    expect(item(1).tabIndex).toBe(0);
    expect(item(0).tabIndex).toBe(-1);
  });

  /**
   * Given the first row focused
   * When the arrow keys are pressed
   * Then focus moves a row at a time, up and down
   */
  it("moves focus between rows with the arrow keys", () => {
    render(<Harness />);
    item(0).focus();

    fireEvent.keyDown(item(0), { key: "ArrowDown" });
    expect(document.activeElement).toBe(item(1));
    fireEvent.keyDown(item(1), { key: "ArrowDown" });
    expect(document.activeElement).toBe(item(2));
    fireEvent.keyDown(item(2), { key: "ArrowUp" });
    expect(document.activeElement).toBe(item(1));
  });

  /**
   * Given a row focused near the edge of the window
   * When ArrowDown takes focus to a row that is not rendered yet
   * Then the list scrolls it into the window and focuses it
   */
  it("brings an unrendered row into the window to focus it", async () => {
    render(<Harness count={200} />);
    await scrollToRow(150);
    item(150).focus();

    fireEvent.keyDown(item(150), { key: "ArrowDown" });

    await waitFor(() => expect(document.activeElement).toBe(item(151)));
  });

  /**
   * Given the first row focused in a long list
   * When End is pressed
   * Then the last row is focused, though it was nowhere near the window
   */
  it("jumps to the last row with End and the first with Home", async () => {
    render(<Harness />);
    item(0).focus();

    fireEvent.keyDown(item(0), { key: "End" });
    await waitFor(() => expect(document.activeElement).toBe(item(9_999)));

    fireEvent.keyDown(item(9_999), { key: "Home" });
    await waitFor(() => expect(document.activeElement).toBe(item(0)));
  });

  /**
   * Given a row focused
   * When Enter or Space is pressed on the row itself
   * Then the list reports that row as activated; the same key inside a control in the row is that
   *      control's own business
   */
  it("activates the focused row with Enter, but not a key pressed inside a button in it", () => {
    const onActivate = vi.fn();
    render(<Harness onActivate={onActivate} />);

    fireEvent.keyDown(item(4), { key: "Enter" });
    fireEvent.keyDown(item(5), { key: " " });
    fireEvent.keyDown(screen.getByText("row 6"), { key: "Enter" });

    expect(onActivate.mock.calls).toEqual([[4], [5]]);
  });

  /**
   * Given a row far outside the window (the one a search found, or the one a form opened)
   * When the list is told to reveal it
   * Then the row is scrolled into view and rendered
   */
  it("scrolls a revealed row into view", async () => {
    const { rerender } = render(<Harness revealIndex={null} />);

    rerender(<Harness revealIndex={9_000} />);

    await waitFor(() => expect(screen.getByText("row 9000")).toBeInTheDocument());
    expect(scroller().scrollTop).toBeGreaterThan(0);
  });

  /**
   * Given a list scrolled far down
   * When what the list shows changes (a new search)
   * Then it is back at its first row, not left among rows that are no longer there
   */
  it("returns to the top when the reset key changes", async () => {
    const { rerender } = render(<Harness resetKey="a" />);
    await scrollToRow(5_000);

    rerender(<Harness resetKey="b" />);

    await waitFor(() => expect(item(0)).not.toBeNull());
    expect(scroller().scrollTop).toBe(0);
  });

  /**
   * Given no rows
   * When the list is rendered
   * Then it renders no row and does not fail
   */
  it("renders nothing for an empty list", () => {
    render(<Harness count={0} />);

    expect(screen.queryAllByRole("listitem")).toHaveLength(0);
  });

  /**
   * Given a list shorter than the viewport
   * When it is rendered
   * Then every row is in the page
   */
  it("renders a short list whole", () => {
    render(<Harness count={4} />);

    expect(screen.getAllByRole("listitem")).toHaveLength(4);
  });
});
