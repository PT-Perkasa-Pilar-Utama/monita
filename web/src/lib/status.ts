// Wording and grading for the public status page. Kept pure so lib/status.test.ts checks it.
import { duration, utc } from "@/lib/time";
import type { PublicIncident, PublicItem, StateKind, UptimeDay } from "@/lib/types";

/** Operational systems drawn with bars; the rest fold into a list, so 200 systems stay readable. */
export const BARS = 12;

/** Down systems, then issues, then the first `BARS` others keep their bars; the rest are listed. */
export function splitSystems(items: PublicItem[]): { barred: PublicItem[]; rest: PublicItem[] } {
  const calm = items.filter((i) => i.state !== "down" && i.state !== "issue");
  const problems = [...items.filter((i) => i.state === "down"), ...items.filter((i) => i.state === "issue")];
  return { barred: [...problems, ...calm.slice(0, BARS)], rest: calm.slice(BARS) };
}

/** One sentence for the whole page: which system is down, or that all run. */
export function headline(overall: StateKind, items: PublicItem[]): string {
  const down = items.filter((i) => i.state === "down");
  if (overall === "down") return down.length === 1 ? `${down[0]!.name} is down` : `${down.length} systems are down`;
  if (overall === "issue") return "Some systems have issues";
  return "All systems operational";
}

/** A day's bar: no data, all up, under 1% down, or more. */
export function barTone(d: UptimeDay): "none" | "up" | "partial" | "down" {
  if (d.uptime === null) return "none";
  if (d.downtime_s === 0) return "up";
  return d.uptime >= 99 ? "partial" : "down";
}

export function barTitle(d: UptimeDay): string {
  if (d.uptime === null) return `${d.day}: no data`;
  const pct = `${d.uptime.toFixed(2)}%`;
  return d.downtime_s === 0 ? `${d.day}: ${pct}` : `${d.day}: ${pct}, down ${duration(d.downtime_s)}`;
}

/** "Since 2026-10-17 10:38 UTC", or the start and how long it took. */
export function when(i: PublicIncident): string {
  return i.resolved_at === null ? `Since ${utc(i.started_at)}` : `${utc(i.started_at)}, resolved after ${duration(i.resolved_at - i.started_at)}`;
}

/** Resolved, Issue for a reported incident (never downtime), or Down. */
export function incidentState(i: PublicIncident): [StateKind, string] {
  if (i.resolved_at !== null) return ["up", "Resolved"];
  return i.origin === "manual" ? ["issue", "Issue"] : ["down", "Down"];
}

const MONTHS = ["January", "February", "March", "April", "May", "June", "July", "August", "September", "October", "November", "December"];

/** "October 2026" for the UTC month a time falls in. */
export function month(ts: number): string {
  const d = new Date(ts * 1000);
  return `${MONTHS[d.getUTCMonth()]} ${d.getUTCFullYear()}`;
}
