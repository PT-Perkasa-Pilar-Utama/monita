/** A table's order: which column and which way. `null` keeps the list's own order. */
export type Sort = { by: string; dir: "asc" | "desc" } | null;

type Key = string | number | null | undefined;

/** A heading's clicks run ascending, descending, then back to the list's own order. */
export function nextSort(current: Sort, by: string): Sort {
  if (current?.by !== by) return { by, dir: "asc" };
  return current.dir === "asc" ? { by, dir: "desc" } : null;
}

/** The order read back from `?sort=&dir=`; anything else is no order. */
export function parseSort(by: string | null, dir: string | null, allowed: readonly string[]): Sort {
  if (!by || !allowed.includes(by)) return null;
  return { by, dir: dir === "desc" ? "desc" : "asc" };
}

/**
 * The rows in `sort` order, by the key each column names. Empty keys go last either way, and
 * equal keys keep their list order. Text compares as people read it: "Svc 2" before "Svc 10".
 */
export function sortRows<T>(rows: readonly T[], sort: Sort, keys: Record<string, (row: T) => Key>): T[] {
  const key = sort ? keys[sort.by] : undefined;
  if (!sort || !key) return [...rows];
  const sign = sort.dir === "asc" ? 1 : -1;
  return rows
    .map((row, i) => ({ row, i, k: key(row) }))
    .sort((a, b) => {
      const ae = a.k === null || a.k === undefined || a.k === "";
      const be = b.k === null || b.k === undefined || b.k === "";
      if (ae || be) return ae === be ? a.i - b.i : ae ? 1 : -1;
      const c = typeof a.k === "number" && typeof b.k === "number" ? a.k - b.k : String(a.k).localeCompare(String(b.k), undefined, { numeric: true, sensitivity: "base" });
      return c === 0 ? a.i - b.i : sign * c;
    })
    .map((x) => x.row);
}
