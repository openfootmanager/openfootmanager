/** Which way a column runs: low to high (A to Z), or high to low. */
export type SortDirection = "asc" | "desc";

/** One known value a row sorts by: a number or a text. */
type SortScalar = number | string;

/**
 * What a row sorts by in one column: a number, a text, a tuple of them compared item by item (a
 * column ordered by several things in turn), or null when it isn't known.
 */
export type SortValue = SortScalar | readonly SortScalar[] | null;

export interface SortState<Key extends string> {
  key: Key;
  direction: SortDirection;
}

/** Two known values, low to high: a tuple's first differing item decides, a shorter one first. */
function compareKnown(
  a: SortScalar | readonly SortScalar[],
  b: SortScalar | readonly SortScalar[],
  locale: string,
): number {
  if (Array.isArray(a) && Array.isArray(b)) {
    for (let index = 0; index < Math.min(a.length, b.length); index++) {
      const order = compareKnown(a[index], b[index], locale);
      if (order !== 0) return order;
    }
    return a.length - b.length;
  }
  if (typeof a === "number" && typeof b === "number") return a - b;
  return String(a).localeCompare(String(b), locale, { sensitivity: "accent" });
}

/**
 * A list's rows in a column's order, for a table the client sorts (the backend sends the whole
 * list). Rows with no value go last in both directions, so a withheld figure ("–") never heads its
 * column; ties keep the order the rows arrived in. Text sorts in the alphabet of `locale`, the
 * game's language (not the machine's), ignoring case but not accents: "Malaga" and "Málaga" are
 * two names, in the language's order.
 */
export function sortRows<Row>(
  rows: readonly Row[],
  sortValueOf: (row: Row) => SortValue,
  direction: SortDirection,
  locale: string,
): Row[] {
  const sign = direction === "asc" ? 1 : -1;
  return [...rows].sort((left, right) => {
    const a = sortValueOf(left);
    const b = sortValueOf(right);
    if (a === null || b === null) return a === b ? 0 : a === null ? 1 : -1;
    return sign * compareKnown(a, b, locale);
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
