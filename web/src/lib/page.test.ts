import { expect, test } from "bun:test";
import { listQuery, pageCount, pageParam, rangeLabel } from "@/lib/page";

test("pages fill 20 rows each and an empty list has one page", () => {
  expect(pageCount(0, 20)).toBe(1);
  expect(pageCount(20, 20)).toBe(1);
  expect(pageCount(21, 20)).toBe(2);
});

test("a missing, bad or low page in the URL is page 1", () => {
  expect(pageParam(null)).toBe(1);
  expect(pageParam("abc")).toBe(1);
  expect(pageParam("0")).toBe(1);
  expect(pageParam("-3")).toBe(1);
  expect(pageParam("3")).toBe(3);
});

test("the range names the rows on the page, and the last page ends at the total", () => {
  expect(rangeLabel(1, 20, 31, "project")).toBe("Showing 1 to 20 of 31 projects.");
  expect(rangeLabel(2, 20, 31, "project")).toBe("Showing 21 to 31 of 31 projects.");
  expect(rangeLabel(1, 20, 1, "project")).toBe("Showing 1 to 1 of 1 project.");
  expect(rangeLabel(1, 20, 0, "project")).toBe("No projects.");
});

test("the query keeps a search, filters and a later page, and leaves out the defaults", () => {
  expect(listQuery({ q: "", page: 1 })).toBe("");
  expect(listQuery({ q: "  pay ", page: 1, owner: "" })).toBe("?q=pay");
  expect(listQuery({ q: "a&b", page: 3, sort: "newest" })).toBe("?q=a%26b&page=3&sort=newest");
});

import { restoreQuery, skeletonCount } from "@/lib/page";

test("a bare visit opens the query used last, and a URL query wins", () => {
  expect(restoreQuery("", "?page=2")).toBe("?page=2");
  expect(restoreQuery("?q=pay", "?page=2")).toBe("?q=pay");
  expect(restoreQuery("", "")).toBe("");
  expect(restoreQuery("", "junk")).toBe("");
});

test("placeholders fill the page that loads, and 6 stand in before the total is known", () => {
  expect(skeletonCount(1, 20, undefined)).toBe(6);
  expect(skeletonCount(1, 20, 32)).toBe(20);
  expect(skeletonCount(2, 20, 32)).toBe(12);
  expect(skeletonCount(1, 20, 0)).toBe(1);
});
