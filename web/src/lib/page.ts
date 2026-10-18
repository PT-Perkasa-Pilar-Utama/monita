// Paging rules for a list the server reads one page at a time.

/** How many pages `total` rows fill; an empty list still has page 1. */
export function pageCount(total: number, pageSize: number): number {
  return Math.max(1, Math.ceil(total / pageSize));
}

/** The page number in a URL: a missing, bad or lower value is page 1. */
export function pageParam(raw: string | null): number {
  const n = Number.parseInt(raw ?? "", 10);
  return Number.isFinite(n) && n > 1 ? n : 1;
}

/** "Showing 21 to 31 of 31 projects." for the rows on `page`. */
export function rangeLabel(page: number, pageSize: number, total: number, noun: string): string {
  const plural = total === 1 ? noun : `${noun}s`;
  if (total === 0) return `No ${noun}s.`;
  const from = (page - 1) * pageSize + 1;
  const to = Math.min(page * pageSize, total);
  return `Showing ${from} to ${to} of ${total} ${plural}.`;
}

/** The query string for a search and a page; an empty search and page 1 are left out. */
export function listQuery(q: string, page: number): string {
  const out = new URLSearchParams();
  if (q.trim()) out.set("q", q.trim());
  if (page > 1) out.set("page", String(page));
  const s = out.toString();
  return s ? `?${s}` : "";
}
