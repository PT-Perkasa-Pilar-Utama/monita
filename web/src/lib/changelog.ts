// The release form and its four boxes of changes, one change per line.
import type { Release } from "@/lib/types";

/** Each kind of change and the form field that holds it, in the order a release shows them. */
export const CHANGE_BOXES = [
  ["Added", "added"],
  ["Changed", "changed"],
  ["Fixed", "fixed"],
  ["Removed", "removed"],
] as const;

export type ReleaseForm = { version: string; released_on: string; title: string; added: string; changed: string; fixed: string; removed: string };

/** The form for a release, or an empty one dated `today`. */
export function releaseForm(r: Release | null, today: string): ReleaseForm {
  const lines = (tag: string): string => r?.sections.find((s) => s.tag === tag)?.items.join("\n") ?? "";
  return {
    version: r?.version ?? "",
    released_on: r?.released_on ?? today,
    title: r?.title ?? "",
    added: lines("Added"),
    changed: lines("Changed"),
    fixed: lines("Fixed"),
    removed: lines("Removed"),
  };
}

/** "3 changes" across every kind. */
export function changeCount(r: Release): string {
  const n = r.sections.reduce((sum, s) => sum + s.items.length, 0);
  return n === 1 ? "1 change" : `${n} changes`;
}
