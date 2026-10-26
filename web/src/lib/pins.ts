// Pins (decision Q48): each user's favourite projects, monitors and heartbeats lead their lists.

/** How many of a kind one user pins: projects in all, monitors or heartbeats per project. */
export const PIN_MAX = 6;

/** The pinned rows first, then the rest, each group in the order it came: the list's own sort
 * orders the pins among themselves. */
export function pinnedFirst<T extends { pinned: boolean }>(rows: T[]): T[] {
  return [...rows.filter((r) => r.pinned), ...rows.filter((r) => !r.pinned)];
}
