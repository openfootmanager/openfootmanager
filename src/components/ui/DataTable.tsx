import { ChevronDown, ChevronUp } from "lucide-react";
import { Fragment, type MouseEvent, type ReactNode, useId, useState } from "react";
import { useTranslation } from "react-i18next";

export type SortDirection = "asc" | "desc";

/** A row's left accent: `positive` for a starter, `danger` for a player who can't play. */
type RowAccent = "positive" | "danger";

export interface DataTableColumn<Row> {
  key: string;
  /** The visible header, translated. Sentence case: table headers aren't uppercase. */
  header: string;
  /** The header's accessible name, when the visible header is a symbol (`#`). */
  headerLabel?: string;
  cell: (row: Row) => ReactNode;
  /** Right-aligned, with tabular figures. */
  numeric?: boolean;
  /** Its header asks to sort by it (the table never sorts the rows itself). */
  sortable?: boolean;
  /**
   * The cell that names the row: it holds the row's open button and its details toggle. With none
   * marked (or the marked one folding away), the first column that never folds does.
   */
  primary?: boolean;
  /**
   * Folded away in a narrow table (under 48rem), and shown in the row's details instead. Never on
   * the first column, which carries the row's accent: there it is ignored.
   */
  drawer?: boolean;
}

interface DataTableProps<Row> {
  /** Names the table for a screen reader: what the list is. */
  label: string;
  columns: readonly DataTableColumn<Row>[];
  rows: readonly Row[];
  rowKey: (row: Row) => string;
  /** The row's name: its open button's, and the subject of its details toggle. */
  rowName: (row: Row) => string;
  sort?: { key: string; direction: SortDirection };
  onSortChange?: (key: string) => void;
  /** Opens the row (its profile): from the name's button, Enter on it, or a click on the row. */
  onOpen?: (row: Row) => void;
  /** A last column of controls (a menu); its header is for screen readers only. */
  rowAction?: { header: string; cell: (row: Row) => ReactNode };
  rowAccent?: (row: Row) => RowAccent | null;
  loading?: boolean;
  /** Shown in place of the rows when there are none; the default says there are no results. */
  empty?: ReactNode;
}

/** The cell's and the header's own controls are inset-ringed: the table scrolls inside a box. */
const CONTROL_FOCUS =
  "rounded focus:outline-none focus-visible:ring-2 focus-visible:ring-inset focus-visible:ring-focus";

/** Hidden under the narrow breakpoint (the drawer column) and above it (the drawer itself). */
const FOLDED = "@max-3xl:hidden";
const UNFOLDED_ONLY = "@3xl:hidden";

/** Every first cell has the accent's width, transparent unless accented, so names line up. */
const ACCENT_EDGE = "border-l-3";
const ACCENT: Record<RowAccent | "none", string> = {
  none: "border-l-transparent",
  positive: "border-l-positive-solid",
  danger: "border-l-danger-solid",
};

/** A click on one of the row's own controls is that control's, not a request to open the row. */
function isOnControl(event: MouseEvent<HTMLElement>): boolean {
  return (event.target as Element).closest("button, a, input, select, textarea") !== null;
}

function SortableHeader({
  column,
  direction,
  onSortChange,
}: {
  column: DataTableColumn<unknown>;
  direction: SortDirection | null;
  onSortChange: (key: string) => void;
}) {
  const Arrow = direction === "asc" ? ChevronUp : ChevronDown;
  return (
    <button
      type="button"
      onClick={() => onSortChange(column.key)}
      aria-label={column.headerLabel}
      className={`inline-flex items-center gap-1 ${column.numeric ? "flex-row-reverse" : ""} ${CONTROL_FOCUS}`}
    >
      {column.header}
      {direction ? <Arrow aria-hidden="true" className="h-3 w-3" /> : null}
    </button>
  );
}

/**
 * The table view of every list (table C): column definitions, one sortable header, rows the
 * height of a row token, and a row that opens its profile. In a narrow table the `drawer` columns
 * fold into the row's details, opened from the name cell. It draws no panel of its own.
 */
export function DataTable<Row>({
  label,
  columns: requestedColumns,
  rows,
  rowKey,
  rowName,
  sort,
  onSortChange,
  onOpen,
  rowAction,
  rowAccent,
  loading = false,
  empty,
}: DataTableProps<Row>) {
  const { t } = useTranslation();
  const idBase = useId();
  const [openDetails, setOpenDetails] = useState<ReadonlySet<string>>(new Set());
  // The first column never folds: its edge carries the accent, and with every column folding it
  // is the one left to hold the row's name, so a narrow row is never out of reach.
  const columns = requestedColumns.map((column, index) =>
    index === 0 && column.drawer ? { ...column, drawer: false } : column,
  );
  const folded = columns.filter((column) => column.drawer);
  // The column holding the row's open button and details toggle: the primary one, or the first
  // that never folds, so a keyboard can always open a row and reach what folded away.
  const nameColumnKey = (
    columns.find((column) => column.primary && !column.drawer) ??
    columns.find((column) => !column.drawer)
  )?.key;
  const span = columns.length + (rowAction ? 1 : 0);

  function toggleDetails(key: string) {
    setOpenDetails((current) => {
      const next = new Set(current);
      if (next.has(key)) next.delete(key);
      else next.add(key);
      return next;
    });
  }

  const cellClass = "h-row whitespace-nowrap border-b border-line px-2 align-middle first:pl-3.5";

  return (
    <div className="@container w-full overflow-x-auto">
      <table
        aria-label={label}
        aria-busy={loading || undefined}
        className="w-full border-separate border-spacing-0 text-body text-fg"
      >
        <thead>
          <tr>
            {columns.map((column) => {
              const direction = sort?.key === column.key ? sort.direction : null;
              return (
                <th
                  key={column.key}
                  scope="col"
                  aria-label={column.headerLabel}
                  aria-sort={
                    direction ? (direction === "asc" ? "ascending" : "descending") : undefined
                  }
                  className={`h-8 whitespace-nowrap border-b border-line-strong bg-panel px-2 text-caption first:border-l-3 first:border-l-transparent first:pl-3.5 ${
                    column.numeric ? "text-right" : "text-left"
                  } ${direction ? "font-semibold text-fg" : "font-medium text-fg-subtle"} ${
                    column.drawer ? FOLDED : ""
                  }`}
                >
                  {column.sortable && onSortChange ? (
                    <SortableHeader
                      column={column as DataTableColumn<unknown>}
                      direction={direction}
                      onSortChange={onSortChange}
                    />
                  ) : (
                    column.header
                  )}
                </th>
              );
            })}
            {rowAction ? (
              <th scope="col" className="h-8 border-b border-line-strong bg-panel px-2">
                <span className="sr-only">{rowAction.header}</span>
              </th>
            ) : null}
          </tr>
        </thead>
        <tbody>
          {rows.length === 0 ? (
            <tr>
              <td colSpan={span} className="px-3.5 py-6 text-center text-body text-fg-muted">
                {loading ? t("common.loading") : (empty ?? t("common.noResults"))}
              </td>
            </tr>
          ) : null}
          {rows.map((row) => {
            const key = rowKey(row);
            const name = rowName(row);
            const accent = rowAccent?.(row) ?? null;
            const detailsId = `${idBase}-${key}-details`;
            const detailsOpen = openDetails.has(key);
            return (
              <Fragment key={key}>
                {/* A pointer convenience only: the keyboard opens the row from the name's button. */}
                <tr
                  data-accent={accent ?? undefined}
                  onClick={
                    onOpen
                      ? (event) => {
                          if (!isOnControl(event)) onOpen(row);
                        }
                      : undefined
                  }
                  className={`hover:bg-hover ${onOpen ? "cursor-pointer" : ""}`}
                >
                  {columns.map((column, index) => (
                    <td
                      key={column.key}
                      className={`${cellClass} ${column.numeric ? "text-right tabular-nums" : ""} ${
                        column.drawer ? FOLDED : ""
                      } ${index === 0 ? `${ACCENT_EDGE} ${ACCENT[accent ?? "none"]}` : ""}`}
                    >
                      {column.key === nameColumnKey ? (
                        <span className="inline-flex min-w-0 items-center gap-1">
                          {onOpen ? (
                            <button
                              type="button"
                              aria-label={name}
                              onClick={() => onOpen(row)}
                              className={`min-w-0 text-left ${CONTROL_FOCUS}`}
                            >
                              {column.cell(row)}
                            </button>
                          ) : (
                            column.cell(row)
                          )}
                          {folded.length > 0 ? (
                            <button
                              type="button"
                              aria-expanded={detailsOpen}
                              aria-controls={detailsId}
                              aria-label={t("common.detailsFor", { name })}
                              onClick={() => toggleDetails(key)}
                              className={`${UNFOLDED_ONLY} grid h-7 w-7 shrink-0 place-items-center text-fg-muted hover:bg-sunken hover:text-fg ${CONTROL_FOCUS}`}
                            >
                              <ChevronDown
                                aria-hidden="true"
                                className={`h-4 w-4 transition-transform motion-reduce:transition-none ${
                                  detailsOpen ? "rotate-180" : ""
                                }`}
                              />
                            </button>
                          ) : null}
                        </span>
                      ) : (
                        column.cell(row)
                      )}
                    </td>
                  ))}
                  {rowAction ? (
                    // The action cell is its control's, padding and all: whatever it renders (a
                    // menu item, a portal), a click in it never opens the row.
                    <td
                      onClick={(event) => event.stopPropagation()}
                      className={`${cellClass} w-0 text-right`}
                    >
                      {rowAction.cell(row)}
                    </td>
                  ) : null}
                </tr>
                {/* Rendered while closed too, hidden, so the toggle's aria-controls always resolves. */}
                {folded.length > 0 ? (
                  <tr id={detailsId} hidden={!detailsOpen} className={UNFOLDED_ONLY}>
                    <td colSpan={span} className="border-b border-line bg-sunken px-3.5 py-2.5">
                      <dl className="flex flex-wrap gap-x-7 gap-y-2 text-caption">
                        {folded.map((column) => (
                          <div key={column.key} className="flex flex-col gap-1">
                            <dt className="text-fg-subtle">
                              {column.headerLabel ?? column.header}
                            </dt>
                            <dd>{column.cell(row)}</dd>
                          </div>
                        ))}
                      </dl>
                    </td>
                  </tr>
                ) : null}
              </Fragment>
            );
          })}
        </tbody>
      </table>
    </div>
  );
}
