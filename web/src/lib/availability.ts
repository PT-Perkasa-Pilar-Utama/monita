// How an availability figure reads, matching the old server page.

/** `99.987%`, or a dash for a period with nothing recorded. */
export function pct(a: number | null): string {
  return a === null ? "–" : `${a.toFixed(3)}%`;
}

/** Three nines and up is good, two is fair, below is poor. */
export function grade(a: number | null): "none" | "good" | "fair" | "poor" {
  if (a === null) return "none";
  if (a >= 99.9) return "good";
  if (a >= 99) return "fair";
  return "poor";
}

/** The bar's fill, 0 to 100: 90 % and below is empty, so the last nines stay visible. */
export function barWidth(a: number | null): number {
  return Math.min(100, Math.max(0, ((a ?? 0) - 90) * 10));
}
