/** The rows of a list worth rendering, as `[start, end)`. */
export interface RowWindow {
  start: number;
  end: number;
}

interface WindowArgs {
  /** Rows in the whole list. */
  count: number;
  /** Distance from one row's top to the next: the row's height plus the gap below it. */
  step: number;
  /** How far the scroller is scrolled. */
  scrollTop: number;
  /** Where the list starts inside the scroller's content (a toolbar above it pushes it down). */
  listTop: number;
  /** The scroller's visible height. */
  viewportHeight: number;
  /** Rows kept rendered beyond the viewport on each side, so a fast scroll never shows a gap. */
  overscan: number;
}

/**
 * The rows to render for a scroller at `scrollTop`: those on screen and `overscan` more either side.
 * A list of any length costs the same, which is the point: appending every row of a 17,000-row list
 * to the page made 394,000 elements and a 5 GB web view.
 */
export function windowRange({
  count,
  step,
  scrollTop,
  listTop,
  viewportHeight,
  overscan,
}: WindowArgs): RowWindow {
  if (count <= 0) return { start: 0, end: 0 };
  const into = scrollTop - listTop;
  const first = Math.floor(into / step);
  const last = Math.ceil((into + viewportHeight) / step);
  const start = Math.min(count, Math.max(0, first - overscan));
  const end = Math.min(count, Math.max(start, last + overscan));
  return { start, end };
}

interface RevealArgs {
  index: number;
  step: number;
  /** The row's own height, without the gap below it. */
  rowHeight: number;
  listTop: number;
  scrollTop: number;
  viewportHeight: number;
}

/**
 * Where to scroll so row `index` is fully on screen: `null` if it already is, else the least
 * movement that brings it in (its top to the viewport's top when it is above, its bottom to the
 * viewport's bottom when it is below).
 */
export function scrollTopToReveal({
  index,
  step,
  rowHeight,
  listTop,
  scrollTop,
  viewportHeight,
}: RevealArgs): number | null {
  const top = listTop + index * step;
  const bottom = top + rowHeight;
  if (top < scrollTop) return top;
  if (bottom > scrollTop + viewportHeight) return bottom - viewportHeight;
  return null;
}
