// Search inside a list the page already holds. Every list the server sends is capped, so the
// page filters in memory.

/** True when `query` is empty or any of `texts` contains it, ignoring case and outer spaces. */
export function matches(query: string, ...texts: (string | null | undefined)[]): boolean {
  const q = query.trim().toLowerCase();
  if (!q) return true;
  return texts.some((t) => (t ?? "").toLowerCase().includes(q));
}

/** A list's search and its two selects; an empty value matches every row. */
/** `from` and `to` are a day range (`YYYY-MM-DD`) for a list the server filters by time. */
export type ListFilter = { q: string; state: string; severity: string; from?: string; to?: string };

/** True when the search matches any text and each set select equals the row's value. */
export function keeps(f: ListFilter, texts: (string | null | undefined)[], state: string, severity: string): boolean {
  return matches(f.q, ...texts) && (!f.state || f.state === state) && (!f.severity || f.severity === severity);
}

/** "4 monitors", or "2 of 4 monitors" when the filter hides some. */
export function countLabel(shown: number, total: number, noun: string): string {
  const plural = total === 1 ? noun : `${noun}s`;
  return shown === total ? `${total} ${plural}` : `${shown} of ${total} ${plural}`;
}
