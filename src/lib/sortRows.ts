/** Which way a column runs: low to high (A to Z), or high to low. */
export type SortDirection = "asc" | "desc";

/** What a row sorts by in one column: a number, a text, or null when it isn't known. */
export type SortValue = number | string | null;

export interface SortState<Key extends string> {
  key: Key;
  direction: SortDirection;
}

/**
 * A list's rows in a column's order, for a table the client sorts (the backend sends the whole
 * list). Rows with no value go last in both directions, so a withheld figure ("–") never heads its
 * column; ties keep the order the rows arrived in. Text sorts alphabetically, ignoring case.
 */
export function sortRows<Row>(
  rows: readonly Row[],
  sortValueOf: (row: Row) => SortValue,
  direction: SortDirection,
): Row[] {
  const sign = direction === "asc" ? 1 : -1;
  return [...rows].sort((left, right) => {
    const a = sortValueOf(left);
    const b = sortValueOf(right);
    if (a === null || b === null) return a === b ? 0 : a === null ? 1 : -1;
    if (typeof a === "number" && typeof b === "number") return sign * (a - b);
    return sign * String(a).localeCompare(String(b), undefined, { sensitivity: "base" });
  });
}

/**
 * The sort after a header is pressed: the same column turns round; another column starts in its
 * own first direction (`firstDirection`), high to low for most figures, A to Z for names.
 */
export function nextSort<Key extends string>(
  current: SortState<Key>,
  key: Key,
  firstDirection: SortDirection,
): SortState<Key> {
  if (current.key === key) {
    return { key, direction: current.direction === "asc" ? "desc" : "asc" };
  }
  return { key, direction: firstDirection };
}
