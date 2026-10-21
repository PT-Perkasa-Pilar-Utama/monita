import { expect, test } from "bun:test";
import { showsNote } from "@/lib/twoFactorNote";

test("the note shows only while two-factor is off and the note was never closed", () => {
  expect(showsNote(false, "")).toBe(true);
  expect(showsNote(false, "1")).toBe(false);
  expect(showsNote(true, "")).toBe(false);
  expect(showsNote(true, "1")).toBe(false);
});
