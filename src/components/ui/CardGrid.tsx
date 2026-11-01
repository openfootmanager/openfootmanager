import type { ReactNode } from "react";

interface CardGridProps {
  /** The cards, each wrapped in its own `<li>`; the grid is the list they belong to. */
  children: ReactNode;
}

/**
 * The grid every card view lays its cards out in: as many 20rem columns as fit, and one full-width
 * column when its container is narrower than that. The track minimum is capped at the container's
 * own width (`min(20rem, 100%)`) because the container, not the viewport, is what can be too
 * narrow: with the sidebar open a wide screen can still hold a narrow grid, and a plain 20rem
 * minimum would make it scroll sideways.
 */
export function CardGrid({ children }: CardGridProps) {
  return (
    <ul className="grid list-none grid-cols-[repeat(auto-fill,minmax(min(20rem,100%),1fr))] gap-3 p-0">
      {children}
    </ul>
  );
}
