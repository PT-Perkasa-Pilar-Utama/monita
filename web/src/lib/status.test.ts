import { expect, test } from "bun:test";
import { barTone, blanksBefore, headline, incidentState, month, monthName, span, splitSystems, when, windowLabel } from "@/lib/status";
import { setOffset } from "@/lib/time";
import type { PublicIncident, PublicItem } from "@/lib/types";

const item = (name: string, state: PublicItem["state"]): PublicItem => ({ kind: "monitor", id: 1, name, state, uptime_90d: 100, days: [] });

test("the headline names a single down system and counts several", () => {
  expect(headline("down", [item("API", "down"), item("Site", "up")])).toBe("API is down");
  expect(headline("down", [item("API", "down"), item("Site", "down")])).toBe("2 systems are down");
  expect(headline("issue", [item("API", "issue")])).toBe("Some systems have issues");
  expect(headline("up", [])).toBe("All systems operational");
});

test("a bar is partial under 1% downtime and down above it", () => {
  expect(barTone({ day: "d", uptime: null, downtime_s: 0 })).toBe("none");
  expect(barTone({ day: "d", uptime: 100, downtime_s: 0 })).toBe("up");
  expect(barTone({ day: "d", uptime: 99.5, downtime_s: 432 })).toBe("partial");
  expect(barTone({ day: "d", uptime: 98.9, downtime_s: 950 })).toBe("down");
});

const inc = (origin: PublicIncident["origin"], resolved_at: number | null): PublicIncident => ({
  title: "t",
  origin,
  severity: "warning",
  started_at: 1_790_000_000,
  resolved_at,
  items: [],
  updates: [],
});

test("a reported incident shows as Issue, never Down", () => {
  expect(incidentState(inc("manual", null))).toEqual(["issue", "Issue"]);
  expect(incidentState(inc("auto", null))).toEqual(["down", "Down"]);
  expect(incidentState(inc("auto", 1_790_000_360))).toEqual(["up", "Resolved"]);
});

test("when and month read in UTC", () => {
  expect(when(inc("auto", 1_790_000_360))).toBe("2026-09-21 14:13 UTC, resolved after 6m 0s");
  expect(when(inc("auto", null))).toBe("Since 2026-09-21 14:13 UTC");
  expect(month(1_790_000_000)).toBe("September 2026");
});

test("problems come first with bars, then 12 others; the rest are a list", () => {
  const calm = Array.from({ length: 14 }, (_, n) => item(`svc ${n}`, "up"));
  const { barred, rest } = splitSystems([...calm, item("Issue one", "issue"), item("Down one", "down")]);
  expect(barred.map((i) => i.name).slice(0, 3)).toEqual(["Down one", "Issue one", "svc 0"]);
  expect(barred).toHaveLength(14);
  expect(rest.map((i) => i.name)).toEqual(["svc 12", "svc 13"]);
});

test("a window of months reads from the oldest to the newest", () => {
  expect(monthName("2026-09")).toBe("September 2026");
  expect(windowLabel(["2026-10", "2026-08", "2026-09"])).toBe("August 2026 to October 2026");
  expect(windowLabel(["2026-10"])).toBe("October 2026");
});

test("a calendar week starts on Sunday", () => {
  expect(blanksBefore("2026-09-01")).toBe(2); // a Tuesday
  expect(blanksBefore("2026-11-01")).toBe(0); // a Sunday
});

test("an incident's span names the end day only when it differs", () => {
  setOffset(0);
  const oct15 = 1_792_022_400; // 2026-10-15 00:00 UTC
  const at = (h: number, m: number): number => oct15 + h * 3600 + m * 60;
  const base = { title: "t", origin: "auto" as const, severity: "warning" as const, items: [], updates: [] };
  expect(span({ ...base, started_at: at(18, 31), resolved_at: at(18, 56) })).toBe("Oct 15, 18:31 to 18:56 UTC");
  expect(span({ ...base, started_at: at(23, 50), resolved_at: at(24, 10) })).toBe("Oct 15, 23:50 to Oct 16, 00:10 UTC");
  expect(span({ ...base, started_at: at(9, 5), resolved_at: null })).toBe("Since Oct 15, 09:05 UTC");
});
