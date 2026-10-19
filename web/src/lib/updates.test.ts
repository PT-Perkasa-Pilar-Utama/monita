import { expect, test } from "bun:test";
import { nextMessage, TEMPLATES } from "@/lib/updates";

test("an empty message takes the new label's template", () => {
  expect(nextMessage("  ", "identified")).toBe(TEMPLATES.identified);
});

test("an untouched template swaps for the new label's", () => {
  expect(nextMessage(TEMPLATES.monitoring, "resolved")).toBe(TEMPLATES.resolved);
});

test("a message the user changed stays", () => {
  const own = `${TEMPLATES.monitoring} The queue is draining.`;
  expect(nextMessage(own, "resolved")).toBe(own);
});
