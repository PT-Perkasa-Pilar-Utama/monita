import { expect, test } from "bun:test";
import { changeCount, releaseBody, releaseForm } from "@/lib/changelog";
import type { Release } from "@/lib/types";

const r: Release = { id: 1, version: "1.4.0", released_on: "2026-10-19", title: "Faster", sections: [{ tag: "Fixed", items: ["A", "B"] }, { tag: "Added", items: ["C"] }] };

test("a release fills one input per change", () => {
  const f = releaseForm(r, "2026-01-01");
  expect([f.added, f.changed, f.fixed, f.removed]).toEqual([["C"], [], ["A", "B"], []]);
  expect(f.released_on).toBe("2026-10-19");
});

test("a new release starts on today with one blank Added change", () => {
  expect(releaseForm(null, "2026-10-20")).toEqual({ version: "", released_on: "2026-10-20", title: "", added: [""], changed: [], fixed: [], removed: [] });
});

test("each change stays one line for the server, blanks dropped", () => {
  const f = { ...releaseForm(null, "2026-10-20"), added: [" A long change that wraps on screen ", ""], fixed: ["B"] };
  const body = releaseBody(f);
  expect(body.added).toBe("A long change that wraps on screen");
  expect(body.fixed).toBe("B");
  expect(body.changed).toBe("");
});

test("the change count adds every kind", () => {
  expect(changeCount(r)).toBe("3 changes");
});
