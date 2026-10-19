import { expect, test } from "bun:test";
import { dateHead, nextDate } from "@/lib/dateSort";

test("a list that is newest by default flips to oldest and back", () => {
  expect(dateHead("released", "", true)).toEqual({ by: "released", dir: "desc" });
  expect(nextDate("", true)).toBe("oldest");
  expect(dateHead("released", "oldest", true)).toEqual({ by: "released", dir: "asc" });
  expect(nextDate("oldest", true)).toBe("");
});

test("a list with its own order runs newest, oldest, then its own order", () => {
  expect(dateHead("started", "", false)).toBeNull();
  expect([nextDate("", false), nextDate("newest", false), nextDate("oldest", false)]).toEqual(["newest", "oldest", ""]);
});
