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
      className="flex min-h-toolbar flex-wrap items-center gap-x-3 gap-y-2 rounded-lg border border-line bg-panel px-3"
    >
      {start ? <div className="flex min-w-0 items-stretch self-stretch">{start}</div> : null}
      <div className="ml-auto flex min-w-0 flex-wrap items-center gap-2 py-2">
        {filters}
        {action}
      </div>
    </div>
  );
}
