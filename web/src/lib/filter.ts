// Search inside a list the page already holds. Every list the server sends is capped, so the
// page filters in memory.

/** True when `query` is empty or any of `texts` contains it, ignoring case and outer spaces. */
export function matches(query: string, ...texts: (string | null | undefined)[]): boolean {
  const q = query.trim().toLowerCase();
  if (!q) return true;
  return texts.some((t) => (t ?? "").toLowerCase().includes(q));
}
