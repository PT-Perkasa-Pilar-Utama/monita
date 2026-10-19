import { expect, test } from "bun:test";
import { parseHeaderLines, whenItFails } from "@/lib/checks";

test("each pair of failure boxes has its own sentence", () => {
  expect(whenItFails(true, true)).toBe("Sends alerts and opens an incident");
  expect(whenItFails(true, false)).toBe("Sends alerts only");
  expect(whenItFails(false, true)).toBe("Opens an incident only");
  expect(whenItFails(false, false)).toBe("Nothing: Monita only records it");
});

test("header lines become pairs, blank and colon-less lines skipped", () => {
  expect(parseHeaderLines("Authorization: Bearer a:b\n\nno colon\nX-Env: prod")).toEqual([
    ["Authorization", "Bearer a:b"],
    ["X-Env", "prod"],
  ]);
});
