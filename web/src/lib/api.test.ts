import { expect, test } from "bun:test";
import { ApiError, dropField } from "@/lib/api";

const error = new ApiError(422, "invalid", "Check the form.", { title: "Enter a title.", message: "Write a message." });

test("changing a field drops its message and keeps the others", () => {
  expect(dropField(error, "message")?.fields).toEqual({ title: "Enter a title." });
});

test("changing the last field with a message clears the whole error", () => {
  const one = new ApiError(422, "invalid", "Write a message.", { message: "Write a message." });
  expect(dropField(one, "message")).toBeNull();
});

test("changing a field with no message keeps the error as it is", () => {
  expect(dropField(error, "severity")).toBe(error);
});
