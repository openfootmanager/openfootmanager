/**
 * Where a key moves the selection in a row of `count` items (a tab list, a radio group): the arrows
 * one step, wrapping at both ends unless `wrap` is off, and Home and End to the ends. Up and down
 * move too when `upDown` is set, as in a WAI-ARIA radio group; a tab list leaves them to the page.
 * Null for any other key.
 */
export function rovingTarget(
  key: string,
  index: number,
  count: number,
  { upDown = false, wrap = true }: { upDown?: boolean; wrap?: boolean } = {},
): number | null {
  const step: Record<string, number> = { ArrowRight: 1, ArrowLeft: -1 };
  if (upDown) {
    step.ArrowDown = 1;
    step.ArrowUp = -1;
  }
  if (key === "Home") return 0;
  if (key === "End") return count - 1;
  if (!(key in step)) return null;
  const next = index + step[key];
  if (!wrap) return Math.min(Math.max(next, 0), count - 1);
  return (next + count) % count;
}
