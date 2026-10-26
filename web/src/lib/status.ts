// Wording and grading for the public status page. Kept pure so lib/status.test.ts checks it.
import { day, duration, hhmm, stamp, zone } from "@/lib/time";
import type { PublicIncident, PublicItem, StateKind, UptimeDay } from "@/lib/types";

/** Operational systems drawn with bars; the rest fold into a list, so 200 systems stay readable. */
export const BARS = 12;

const LOUD: StateKind[] = ["down", "issue", "maintenance"];

/** Down systems, then issues, then maintenance, then the first `BARS` others keep their bars; the rest are listed. */
export function splitSystems(items: PublicItem[]): { barred: PublicItem[]; rest: PublicItem[] } {
  const calm = items.filter((i) => !LOUD.includes(i.state));
  const problems = LOUD.flatMap((s) => items.filter((i) => i.state === s));
  return { barred: [...problems, ...calm.slice(0, BARS)], rest: calm.slice(BARS) };
}

/** One sentence for the whole page: which system is down, that all run, or that it cannot tell
 * yet. With no public system, or none checked, it never says all systems work. */
export function headline(overall: StateKind, items: PublicItem[]): string {
  const down = items.filter((i) => i.state === "down");
  if (overall === "down") return down.length === 1 ? `${down[0]!.name} is down` : `${down.length} systems are down`;
  if (overall === "issue") return "Some systems have issues";
  if (overall === "maintenance") return "Maintenance in progress";
  if (overall === "up") return "All systems operational";
  return items.length === 0 ? "No systems to show yet" : "Waiting for the first checks";
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
  if (i.resolved_at === null) return `Since ${shortDay(day(i.started_at))}, ${hhmm(i.started_at)} ${zone()}`.trimEnd();
  return between(i.started_at, i.resolved_at);
}

/** "Oct 15, 18:31 to 18:56 UTC+7", with the end day when it ends on another. */
export function between(from: number, to: number): string {
  const start = `${shortDay(day(from))}, ${hhmm(from)}`;
  const end = day(from) === day(to) ? hhmm(to) : `${shortDay(day(to))}, ${hhmm(to)}`;
  return `${start} to ${end} ${zone()}`.trimEnd();
}

/** The mean of the bars that have data, as "99.87%", or null when none has. */
export function meanUptime(days: UptimeDay[]): string | null {
  const known = days.map((d) => d.uptime).filter((u): u is number => u !== null);
  if (known.length === 0) return null;
  return `${(known.reduce((a, b) => a + b, 0) / known.length).toFixed(2)}%`;
}

/** Runs of incidents that start in the same month, in the order given, as "October 2026". */
export function byMonth(incidents: PublicIncident[]): { month: string; incidents: PublicIncident[] }[] {
  const groups: { month: string; incidents: PublicIncident[] }[] = [];
  for (const i of incidents) {
    const m = month(i.started_at);
    const last = groups.at(-1);
    if (last?.month === m) last.incidents.push(i);
    else groups.push({ month: m, incidents: [i] });
  }
  return groups;
}

/** The incidents that have ended; open ones show under Happening now instead. */
export function resolvedOnly(incidents: PublicIncident[]): PublicIncident[] {
  return incidents.filter((i) => i.resolved_at !== null);
}
