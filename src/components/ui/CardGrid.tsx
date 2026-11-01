import type { ReactNode } from "react";

interface CardGridProps {
  /** The cards, each wrapped in its own `<li>`; the grid is the list they belong to. */
  children: ReactNode;
}

/**
 * The grid every card view lays its cards out in: one column on a narrow screen, then as many
 * 20rem columns as fit. The single column is explicit because a 20rem minimum alone would make
 * the grid wider than a narrow container, and the page would scroll sideways.
 */
export function CardGrid({ children }: CardGridProps) {
  return (
    <ul className="grid list-none grid-cols-1 gap-3 p-0 sm:grid-cols-[repeat(auto-fill,minmax(20rem,1fr))]">
      {children}
    </ul>
  );
}
