// Wording and grading for the public status page. Kept pure so lib/status.test.ts checks it.
import { day, duration, hhmm, stamp, zone } from "@/lib/time";
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
  return i.resolved_at === null ? `Since ${stamp(i.started_at)}` : `${stamp(i.started_at)}, resolved after ${duration(i.resolved_at - i.started_at)}`;
}

/** Resolved, Issue for a reported incident (never downtime), or Down. */
export function incidentState(i: PublicIncident): [StateKind, string] {
  if (i.resolved_at !== null) return ["up", "Resolved"];
  return i.origin === "manual" ? ["issue", "Issue"] : ["down", "Down"];
}

const MONTHS = ["January", "February", "March", "April", "May", "June", "July", "August", "September", "October", "November", "December"];

/** "October 2026" for the month a time falls in, in the admin's offset. */
export function month(ts: number): string {
  const [y, m] = day(ts).split("-");
  return `${MONTHS[Number(m) - 1]} ${y}`;
}

/** "October 2026" for "2026-10". */
export function monthName(ym: string): string {
  const [y, m] = ym.split("-");
  return `${MONTHS[Number(m) - 1]} ${y}`;
}

/** "August 2026 to October 2026" for a window of months, in any order. */
export function windowLabel(months: string[]): string {
  const sorted = [...months].sort();
  if (sorted.length === 0) return "";
  const [first, last] = [sorted[0]!, sorted[sorted.length - 1]!];
  return first === last ? monthName(first) : `${monthName(first)} to ${monthName(last)}`;
}

/** Empty squares before the 1st in a calendar whose weeks start on Sunday. */
export function blanksBefore(firstDay: string): number {
  const [y, m, d] = firstDay.split("-").map(Number);
  return new Date(Date.UTC(y!, m! - 1, d!)).getUTCDay();
}

/** "Oct 15" for a day string. */
function shortDay(d: string): string {
  return `${MONTHS[Number(d.slice(5, 7)) - 1]!.slice(0, 3)} ${Number(d.slice(8, 10))}`;
}

/** "Oct 15, 18:31 to 18:56 UTC+7", with the end day when it ends on another, or "since" while open. */
export function span(i: PublicIncident): string {
  const start = `${shortDay(day(i.started_at))}, ${hhmm(i.started_at)}`;
  if (i.resolved_at === null) return `Since ${start} ${zone()}`;
  const sameDay = day(i.started_at) === day(i.resolved_at);
  const end = sameDay ? hhmm(i.resolved_at) : `${shortDay(day(i.resolved_at))}, ${hhmm(i.resolved_at)}`;
  return `${start} to ${end} ${zone()}`;
}

/** The mean of the bars that have data, as "99.87%", or null when none has. */
export function meanUptime(days: UptimeDay[]): string | null {
  const known = days.map((d) => d.uptime).filter((u): u is number => u !== null);
  if (known.length === 0) return null;
  return `${(known.reduce((a, b) => a + b, 0) / known.length).toFixed(2)}%`;
}
