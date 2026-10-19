// The release form: one input per change, so a long change that wraps is still one change.
import type { Release } from "@/lib/types";

/** Each kind of change and the form field that holds it, in the order a release shows them. */
export const CHANGE_BOXES = [
  ["Added", "added"],
  ["Changed", "changed"],
  ["Fixed", "fixed"],
  ["Removed", "removed"],
] as const;

export type ChangeKey = (typeof CHANGE_BOXES)[number][1];

export type ReleaseForm = { version: string; released_on: string; title: string } & Record<ChangeKey, string[]>;

/** The form for a release, or an empty one dated `today` with one blank Added change. */
export function releaseForm(r: Release | null, today: string): ReleaseForm {
  const items = (tag: string): string[] => r?.sections.find((s) => s.tag === tag)?.items ?? [];
  return {
    version: r?.version ?? "",
    released_on: r?.released_on ?? today,
    title: r?.title ?? "",
    added: r ? items("Added") : [""],
    changed: items("Changed"),
    fixed: items("Fixed"),
    removed: items("Removed"),
  };
}

/** What the server takes: each kind as lines, blank changes dropped. */
export function releaseBody(f: ReleaseForm): Record<string, string> {
  const lines = (xs: string[]): string =>
    xs
      .map((x) => x.trim())
      .filter((x) => x !== "")
      .join("\n");
  return { version: f.version, released_on: f.released_on, title: f.title, added: lines(f.added), changed: lines(f.changed), fixed: lines(f.fixed), removed: lines(f.removed) };
}

/** "3 changes" across every kind. */
export function changeCount(r: Release): string {
  const n = r.sections.reduce((sum, s) => sum + s.items.length, 0);
  return n === 1 ? "1 change" : `${n} changes`;
}
