/** Monitors and heartbeats picked on a list, for the row menu and the batch actions. */
export type SourceKind = "monitor" | "heartbeat";

export type Picked = { id: number; name: string; state: string; paused: boolean };

/** "monitor" or "monitors" for `n`. */
export function noun(kind: SourceKind, n: number): string {
  return n === 1 ? kind : `${kind}s`;
}

/** The maintenance form with these sources already picked. */
export function maintenancePath(slug: string, kind: SourceKind, ids: number[]): string {
  return `/projects/${slug}/maintenance/new?${ids.map((id) => `item=${kind}:${id}`).join("&")}`;
}

/** The names a delete dialog spells out, and how many more it only counts. */
export function shownNames(names: string[], max = 10): { shown: string[]; more: number } {
  return { shown: names.slice(0, max), more: Math.max(0, names.length - max) };
}

/** What a batch delete makes the user type, so a slip of the mouse deletes nothing. */
export function confirmPhrase(n: number): string {
  return `delete ${n}`;
}
