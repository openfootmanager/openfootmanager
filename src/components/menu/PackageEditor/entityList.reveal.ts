import { useEffect } from "react";

/**
 * Hold on to a window that `capRows` stretched to keep a selected row visible.
 *
 * The stretch is computed fresh each render from `visibleCount`, so on its own
 * it does not accumulate: duplicate the last visible row often enough and the
 * selection eventually reaches `visibleCount + pageSize`, the stretch is
 * refused, and the list snaps back — hiding rows that were on screen a moment
 * earlier along with the copy just made. Folding each stretch back into the
 * count means the next one is measured from where the list actually ends.
 *
 * Only ever grows. Narrowing the list is the job of the reset in the search and
 * filter handlers, which is deliberate: a stretch should survive a keystroke in
 * the form, but not a change to what the list is showing.
 */
export function useKeepRevealed(
  shownCount: number,
  visibleCount: number,
  setVisibleCount: (count: number) => void,
): void {
  useEffect(() => {
    if (shownCount > visibleCount) {
      setVisibleCount(shownCount);
    }
    // setVisibleCount is a state setter, stable for the life of the component.
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [shownCount, visibleCount]);
}
