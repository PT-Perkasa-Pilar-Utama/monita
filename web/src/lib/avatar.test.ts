import { expect, test } from "bun:test";
import { avatarUrl } from "@/lib/avatar";

test("a picture's URL names its user and changes with its time, and no picture has none", () => {
  expect(avatarUrl(4, null)).toBeNull();
  expect(avatarUrl(4, 1700000000)).toBe("/users/4/avatar?v=1700000000");
});
