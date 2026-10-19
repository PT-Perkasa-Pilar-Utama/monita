import { expect, test } from "bun:test";
import { parseLast } from "@/lib/lastProject";

test("a stored project comes back with its name and icon", () => {
  expect(parseLast('{"slug":"payments-x1","name":"Payments","icon_url":"/i.png"}')).toEqual({ slug: "payments-x1", name: "Payments", icon_url: "/i.png" });
  expect(parseLast('{"slug":"payments-x1","name":"Payments"}')?.icon_url).toBeNull();
});

test("an empty, broken or foreign value is no project", () => {
  expect(parseLast("")).toBeNull();
  expect(parseLast("{not json")).toBeNull();
  expect(parseLast('{"slug":"","name":"x"}')).toBeNull();
  expect(parseLast('"payments"')).toBeNull();
});
