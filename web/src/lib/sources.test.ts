import { expect, test } from "bun:test";
import { confirmPhrase, maintenancePath, noun, shownNames } from "./sources";

test("the maintenance form gets every picked source", () => {
  expect(maintenancePath("pay", "monitor", [3, 7])).toBe("/projects/pay/maintenance/new?item=monitor:3&item=monitor:7");
});

test("a long list names ten and counts the rest", () => {
  const names = Array.from({ length: 13 }, (_, i) => `m${i}`);
  expect(shownNames(names)).toEqual({ shown: names.slice(0, 10), more: 3 });
  expect(shownNames(["a"])).toEqual({ shown: ["a"], more: 0 });
});

test("words follow the count", () => {
  expect(noun("heartbeat", 1)).toBe("heartbeat");
  expect(noun("heartbeat", 2)).toBe("heartbeats");
  expect(confirmPhrase(4)).toBe("delete 4");
});
