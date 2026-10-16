import { expect, test } from "bun:test";
import { workersFor } from "./capacity";

test("a laptop gets half its cores, fewer when busy, at least one", () => {
  expect(workersFor({ cpus: 10, load1: 1, totalGiB: 16, ci: false })).toBe(4);
  expect(workersFor({ cpus: 4, load1: 3.5, totalGiB: 16, ci: false })).toBe(1);
  expect(workersFor({ cpus: 1, load1: 0, totalGiB: 8, ci: false })).toBe(1);
});

test("E2E_WORKERS wins, CI caps at four", () => {
  expect(workersFor({ cpus: 10, load1: 9, totalGiB: 8, ci: false, override: "3" })).toBe(3);
  expect(workersFor({ cpus: 16, load1: 0, totalGiB: 64, ci: true })).toBe(4);
});
