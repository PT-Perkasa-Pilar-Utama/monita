import { expect, test } from "bun:test";
import { changeCount, releaseForm } from "@/lib/changelog";
import type { Release } from "@/lib/types";

const r: Release = { id: 1, version: "1.4.0", released_on: "2026-10-19", title: "Faster", sections: [{ tag: "Fixed", items: ["A", "B"] }, { tag: "Added", items: ["C"] }] };

test("a release fills its four boxes one change per line", () => {
  const f = releaseForm(r, "2026-01-01");
  expect([f.added, f.changed, f.fixed, f.removed]).toEqual(["C", "", "A\nB", ""]);
  expect(f.released_on).toBe("2026-10-19");
});

test("a new release starts empty on today", () => {
  expect(releaseForm(null, "2026-10-20")).toEqual({ version: "", released_on: "2026-10-20", title: "", added: "", changed: "", fixed: "", removed: "" });
});

test("the change count adds every kind", () => {
  expect(changeCount(r)).toBe("3 changes");
});
