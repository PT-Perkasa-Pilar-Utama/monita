import { expect, test } from "bun:test";
import { switchResetsPage, tileLabel } from "@/lib/projectGrid";

test("a tile names its project and its state in words", () => {
  expect(tileLabel("Payments", "down")).toBe("Payments, down");
  expect(tileLabel("Search", "issue")).toBe("Search, has an issue");
  expect(tileLabel("Portal", "unknown")).toBe("Portal, no data yet");
});

test("only a switch into or out of the compact view goes back to page 1", () => {
  expect(switchResetsPage("list", "compact")).toBe(true);
  expect(switchResetsPage("compact", "card")).toBe(true);
  expect(switchResetsPage("card", "list")).toBe(false);
  expect(switchResetsPage("chart", "card")).toBe(false);
});
