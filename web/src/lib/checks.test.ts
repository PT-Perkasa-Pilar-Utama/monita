import { expect, test } from "bun:test";
import { whenItFails } from "@/lib/checks";

test("each pair of failure boxes has its own sentence", () => {
  expect(whenItFails(true, true)).toBe("Sends alerts and opens an incident");
  expect(whenItFails(true, false)).toBe("Sends alerts only");
  expect(whenItFails(false, true)).toBe("Opens an incident only");
  expect(whenItFails(false, false)).toBe("Nothing: Monita only records it");
});
