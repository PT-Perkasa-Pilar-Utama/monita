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

test("the query keeps a search and a later page, and leaves out the defaults", () => {
  expect(listQuery("", 1)).toBe("");
  expect(listQuery("  pay ", 1)).toBe("?q=pay");
  expect(listQuery("a&b", 3)).toBe("?q=a%26b&page=3");
});
