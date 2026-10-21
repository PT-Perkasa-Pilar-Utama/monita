import { expect, test } from "bun:test";
import { clampQuorum, fresh, lastSeen, probeCommand } from "@/lib/locations";

test("the probe command keeps the token in a variable", () => {
  expect(probeCommand("https://monita.example.com", "ml_abc")).toBe(
    "export MONITA_PROBE_TOKEN='ml_abc'\n./monita probe --server https://monita.example.com",
  );
});

test("a probe silent for two minutes reads as stale", () => {
  expect(lastSeen(null, 100)).toEqual({ text: "Never", stale: true });
  expect(lastSeen(100, 130)).toEqual({ text: "Just now", stale: false });
  expect(lastSeen(100, 219)).toEqual({ text: "1m ago", stale: false });
  expect(lastSeen(100, 220)).toEqual({ text: "2m ago", stale: true });
  expect(lastSeen(0, 3 * 86_400)).toEqual({ text: "3d ago", stale: true });
});

test("the quorum stays between 1 and the number of places", () => {
  expect(clampQuorum("3", 2)).toBe("2");
  expect(clampQuorum("0", 3)).toBe("1");
  expect(clampQuorum("2", 3)).toBe("2");
  expect(clampQuorum("", 3)).toBe("");
});

test("a place's result counts for two intervals and 30 seconds", () => {
  expect(fresh(1000 - 150, 1000, 60)).toBe(true);
  expect(fresh(1000 - 151, 1000, 60)).toBe(false);
});
