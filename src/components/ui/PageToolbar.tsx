import type { ReactNode } from "react";

interface PageToolbarProps {
  /** Usually the page's Tabs. */
  start?: ReactNode;
  /** Search and filter controls, placed before the action. */
  filters?: ReactNode;
  /** The page's one primary action. It always sits at the toolbar's right end. */
  action?: ReactNode;
}

/**
 * The row under the header where a page keeps its sections, its filters and its one primary
 * action, in the same place on every page. It wraps onto a second line when the window is narrow
 * instead of scrolling sideways.
 */
export function PageToolbar({ start, filters, action }: PageToolbarProps) {
  return (
    <div
      data-testid="page-toolbar"
      // An inset ring, not a border: it takes no space, so a tabs row of min-h-toolbar fills the
      // toolbar exactly instead of making it two pixels taller.
      className="flex min-h-toolbar flex-wrap items-center gap-x-3 gap-y-2 rounded-lg bg-panel px-3 ring-1 ring-inset ring-line"
    >
      {start ? (
        <div className="flex min-h-toolbar min-w-0 max-w-full items-stretch self-stretch">
          {start}
        </div>
      ) : null}
      <div className="ml-auto flex min-w-0 flex-wrap items-center gap-2 py-2">
        {filters}
        {action}
      </div>
    </div>
  );
}
