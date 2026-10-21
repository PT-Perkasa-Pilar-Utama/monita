import { expect, test } from "bun:test";
import { avatarUrl } from "@/lib/avatar";

test("a picture's URL changes with its time, and no picture has none", () => {
  expect(avatarUrl(null)).toBeNull();
  expect(avatarUrl(1700000000)).toBe("/me/avatar?v=1700000000");
});
