import { expect, test } from "bun:test";
import { matches } from "@/lib/filter";

test("an empty query keeps every row", () => {
  expect(matches("  ", "API")).toBe(true);
});

test("the query matches any text, ignoring case, and a missing text never matches", () => {
  expect(matches(" PORTAL ", "API", "https://portal.example.com")).toBe(true);
  expect(matches("portal", "API", null, undefined)).toBe(false);
});

import { countLabel, keeps } from "@/lib/filter";

test("a set select matches its value exactly, an empty one matches all", () => {
  const f = { q: "portal", state: "down", severity: "" };
  expect(keeps(f, ["API", "https://portal.example.com"], "down", "critical")).toBe(true);
  expect(keeps(f, ["API", "https://portal.example.com"], "up", "critical")).toBe(false);
  expect(keeps({ ...f, severity: "warning" }, ["Portal"], "down", "critical")).toBe(false);
});

test("the count names the total and what the filter hides", () => {
  expect(countLabel(1, 1, "monitor")).toBe("1 monitor");
  expect(countLabel(2, 4, "monitor")).toBe("2 of 4 monitors");
});
