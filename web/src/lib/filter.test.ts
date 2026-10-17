import { expect, test } from "bun:test";
import { matches } from "@/lib/filter";

test("an empty query keeps every row", () => {
  expect(matches("  ", "API")).toBe(true);
});

test("the query matches any text, ignoring case, and a missing text never matches", () => {
  expect(matches(" PORTAL ", "API", "https://portal.example.com")).toBe(true);
  expect(matches("portal", "API", null, undefined)).toBe(false);
});
