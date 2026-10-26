/** A project's primitives: the tabs that each get a footer illustration. */
export type Primitive = "monitors" | "heartbeats" | "incidents" | "maintenance" | "tickets" | "changelog";

const PRIMITIVES: readonly Primitive[] = ["monitors", "heartbeats", "incidents", "maintenance", "tickets", "changelog"];

/** The primitive whose tab `pathname` is in, under the project at `base`; null for Overview and
 * Settings. */
export function footerFor(pathname: string, base: string): Primitive | null {
  if (!pathname.startsWith(`${base}/`)) return null;
  const first = pathname.slice(base.length + 1).split("/")[0];
  return PRIMITIVES.find((p) => p === first) ?? null;
}
