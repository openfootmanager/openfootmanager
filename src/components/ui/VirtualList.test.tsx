import { useMemo } from "react";
import { fireEvent, render, screen, waitFor } from "@testing-library/react";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";

import { installScrollGeometry } from "../../test-utils/scrollGeometry";
import { VirtualList } from "./VirtualList";

// jsdom has no layout; the geometry fixture gives the scroller a 640 px viewport and puts the list
// 96 px down its content (a toolbar above it), moving the list's top with the scroll as a browser does.
const STEP = 64;
const VIEWPORT = 640;
const LIST_OFFSET = 96;

let restoreGeometry: () => void;
beforeEach(() => {
  restoreGeometry = installScrollGeometry({ viewportHeight: VIEWPORT, listOffset: LIST_OFFSET });
});
afterEach(() => restoreGeometry());

interface HarnessProps {
  count?: number;
  /** The rows' identities, in order; defaults to 0..count-1. */
  ids?: number[];
  revealIndex?: number | null;
  resetKey?: unknown;
  onActivate?: (index: number) => void;
}

function Harness({ count = 10_000, ids, revealIndex, resetKey, onActivate }: HarnessProps) {
  const items = useMemo(() => ids ?? Array.from({ length: count }, (_, i) => i), [count, ids]);
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
  scroller().scrollTop = LIST_OFFSET + row * STEP;
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
    expect(rendered).toBeLessThan(25);
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
    expect(screen.getAllByRole("listitem").length).toBeLessThan(25);
    // The row at the top of the viewport and the rows below it are all there.
    expect(screen.getByText("row 5009")).toBeInTheDocument();
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
   * Given the record at row 101 focused, then scrolled far away
   * When a row is inserted above it (a duplicate of row 0), and then removed again (undo)
   * Then the focused record keeps focus and its node each time: focus follows the record's identity,
   *      not the index it had, so a shift cannot hand it to the wrong row or unmount it
   */
  it("keeps focus on the same record when rows are inserted or removed above it", async () => {
    const original = Array.from({ length: 300 }, (_, i) => i);
    const inserted = [-1, ...original];
    const { rerender } = render(<Harness ids={original} />);
    await scrollToRow(101);
    item(101).focus();
    const focused = item(101);
    await scrollToRow(250);
    expect(document.activeElement).toBe(focused);

    rerender(<Harness ids={inserted} />);

    // The record is now row 102; it is the same node, still focused, and the tab stop.
    expect(item(102)).toBe(focused);
    expect(document.activeElement).toBe(focused);
    expect(focused.tabIndex).toBe(0);
    expect(focused.textContent).toBe("row 101");

    rerender(<Harness ids={original} />);

    expect(item(101)).toBe(focused);
    expect(document.activeElement).toBe(focused);
    expect(focused.textContent).toBe("row 101");
  });

  /**
   * Given the focused record removed from the list
   * When the list re-renders
   * Then nothing throws and the list has a tab stop again
   */
  it("keeps a tab stop when the focused record is removed", () => {
    const ids = Array.from({ length: 40 }, (_, i) => i);
    const { rerender } = render(<Harness ids={ids} />);
    item(5).focus();

    rerender(<Harness ids={ids.filter((id) => id !== 5)} />);

    expect(screen.getAllByRole("listitem").filter((row) => row.tabIndex === 0)).toHaveLength(1);
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
