import { expect, test } from "bun:test";
import { fileFor, lookup, pageLinks, recordable, shiftTimes } from "./demo-site";

test("a page is saved as its folder's index.html", () => {
  expect(fileFor("/")).toBe("index.html");
  expect(fileFor("/projects/portal/monitors")).toBe("projects/portal/monitors/index.html");
});

test("the crawler follows plain page hrefs only", () => {
  const html = `<a href="/a">a</a><a href="/b?before=1.2">next</a><form action="/c"></form>
    <link href="/static/app.css?v=1"><a href="/assets/x.js">x</a><a href="https://monita.example.com/status/x/">x</a><a href="https://other.example.com/y">y</a>`;
  expect(pageLinks(html).sort()).toEqual(["/a", "/status/x"]);
});

test("the demo answers the UI API, project icons and user pictures", () => {
  expect(recordable("/api/ui/projects")).toBe(true);
  expect(recordable("/projects/portal/icon")).toBe(true);
  expect(recordable("/users/3/avatar")).toBe(true);
  expect(recordable("/projects/portal")).toBe(false);
  expect(recordable("/api/v1/ref-ids")).toBe(false);
});

test("times move by the delta, dates by whole days, and nothing else moves", () => {
  const got = shiftTimes(
    {
      id: 1_700_000_000,
      at: 1_700_000_000,
      last_check_at: 1_700_000_000,
      started_at: null,
      last: [1_700_000_000, true, null],
      prev_to: 1_700_000_000,
      day: "2026-10-27",
      bars: [{ day: "2026-10-31", up: 5, title: "2026-10-31 13:15: No pings" }],
      count_at: 12,
      total: 1_700_000_000,
    },
    2 * 86_400 + 3_600,
  );
  expect(got).toEqual({
    id: 1_700_000_000,
    at: 1_700_176_400,
    last_check_at: 1_700_176_400,
    started_at: null,
    last: [1_700_176_400, true, null],
    prev_to: 1_700_176_400,
    day: "2026-10-29",
    bars: [{ day: "2026-11-02", up: 5, title: "2026-11-02 13:15: No pings" }],
    count_at: 12,
    total: 1_700_000_000,
  });
  // Dates move by the nearest whole day.
  expect(shiftTimes({ day: "2026-10-27" }, 86_400 - 60)).toEqual({ day: "2026-10-28" });
});

test("a request finds its recording by query, by a moved cursor, then by path", () => {
  const keys = ["/api/ui/projects", "/api/ui/projects?view=list", "/api/ui/incidents?before=1700000000", "/api/ui/x?page=2"];
  expect(lookup(keys, "/api/ui/projects", "?view=list", 500)).toBe("/api/ui/projects?view=list");
  expect(lookup(keys, "/api/ui/incidents", "?before=1700000500", 500)).toBe("/api/ui/incidents?before=1700000000");
  // The page was opened under a shift a few minutes older than the worker's.
  expect(lookup(keys, "/api/ui/incidents", "?before=1700000300", 500)).toBe("/api/ui/incidents?before=1700000000");
  expect(lookup(keys, "/api/ui/projects", "?state=down", 500)).toBe("/api/ui/projects");
  expect(lookup(keys, "/api/ui/x", "?page=3", 500)).toBeNull();
  expect(lookup(keys, "/api/ui/missing", "", 500)).toBeNull();
});
