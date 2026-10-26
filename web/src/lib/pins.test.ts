import { expect, test } from "bun:test";
import { pinnedFirst } from "@/lib/pins";

test("pinned rows come first and each group keeps its order", () => {
  const rows = [
    { id: 1, pinned: false },
    { id: 2, pinned: true },
    { id: 3, pinned: false },
    { id: 4, pinned: true },
  ];
  expect(pinnedFirst(rows).map((r) => r.id)).toEqual([2, 4, 1, 3]);
  expect(pinnedFirst([])).toEqual([]);
});
