import { expect, test } from "bun:test";
import { nextSort, parseSort, sortRows } from "@/lib/sort";

test("a heading's clicks run ascending, descending, then off", () => {
  const a = nextSort(null, "name");
  expect(a).toEqual({ by: "name", dir: "asc" });
  const d = nextSort(a, "name");
  expect(d).toEqual({ by: "name", dir: "desc" });
  expect(nextSort(d, "name")).toBeNull();
  expect(nextSort(d, "age")).toEqual({ by: "age", dir: "asc" });
});

test("only a known column comes back from the URL", () => {
  expect(parseSort("name", "desc", ["name"])).toEqual({ by: "name", dir: "desc" });
  expect(parseSort("name", "sideways", ["name"])).toEqual({ by: "name", dir: "asc" });
  expect(parseSort("password", "asc", ["name"])).toBeNull();
});

type Row = { name: string; seen: number | null };
const rows: Row[] = [
  { name: "Svc 10", seen: 5 },
  { name: "svc 2", seen: null },
  { name: "Api", seen: 9 },
];
const keys = { name: (r: Row) => r.name, seen: (r: Row) => r.seen };

test("text sorts the way people read numbers, without case", () => {
  expect(sortRows(rows, { by: "name", dir: "asc" }, keys).map((r) => r.name)).toEqual(["Api", "svc 2", "Svc 10"]);
  expect(sortRows(rows, { by: "name", dir: "desc" }, keys).map((r) => r.name)).toEqual(["Svc 10", "svc 2", "Api"]);
});

test("an empty key goes last both ways", () => {
  expect(sortRows(rows, { by: "seen", dir: "asc" }, keys).map((r) => r.seen)).toEqual([5, 9, null]);
  expect(sortRows(rows, { by: "seen", dir: "desc" }, keys).map((r) => r.seen)).toEqual([9, 5, null]);
});

test("no order keeps the list's own order", () => {
  expect(sortRows(rows, null, keys)).toEqual(rows);
});
